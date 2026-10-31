//! Module functions ([`crate::stdlib`]): `sys.argv` from the
//! interpreter's state, `os` and `os.path` over `std::fs`.

use std::{fs, io};

use super::builtins::Args;
use std::rc::Rc;

use super::value::{str_repr, Exc, ExcArg, Value};
use super::{rt_err, Exec, Interp};
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

impl<'m> Interp<'m, '_> {
    /// Calls `f`; inference checked the arguments' count and types.
    pub(super) fn std(&mut self, f: StdFn, args: &Args<'m>) -> Exec<'m, Value<'m>> {
        let span = args.span;
        let named = args.keywords.iter().map(|(kw, v)| (kw.name.as_str(), v));
        let keywords = named.map(|(kw, v)| match f.keyword() == Some(kw) {
            true => Ok(v),
            false => {
                let msg = format!("internal: `{}` got keyword `{kw}`", f.name());
                Err(rt_err(span, msg).into())
            }
        });
        let paths = args
            .positional
            .iter()
            .map(Ok)
            .chain(keywords)
            .map(|v| v.and_then(|v| path(v, span)))
            .collect::<Exec<'m, Vec<&str>>>()?;
        Ok(match (f, paths.as_slice()) {
            (StdFn::Argv, []) => Value::list(self.argv.iter().map(|a| Value::str(a)).collect()),
            (StdFn::Listdir, []) => listdir(".", span)?,
            (StdFn::Listdir, [p]) => listdir(p, span)?,
            (StdFn::Join, [first, rest @ ..]) => Value::str(&join(first, rest)),
            (StdFn::Isdir, [p]) => Value::Bool(fs::metadata(p).is_ok_and(|m| m.is_dir())),
            (StdFn::Isfile, [p]) => Value::Bool(fs::metadata(p).is_ok_and(|m| m.is_file())),
            (StdFn::Islink, [p]) => {
                let link = fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
                Value::Bool(link)
            }
            (StdFn::Getsize, [p]) => {
                let len = fs::metadata(p).map_err(|e| os_error(&e, p, span))?.len();
                let len = i64::try_from(len)
                    .map_err(|_| rt_err(span, "OverflowError: file size beyond i64".into()))?;
                Value::Int(len)
            }
            _ => {
                let msg = format!("internal: `{}` got {} arguments", f.name(), paths.len());
                return Err(rt_err(span, msg).into());
            }
        })
    }
}

/// A path argument: inference made it a `str`.
fn path<'v, 'm>(v: &'v Value<'m>, span: Span) -> Exec<'m, &'v str> {
    match v {
        Value::Str(s) => Ok(s),
        other => {
            let msg = format!("internal: a path is a str, not {}", other.type_name());
            Err(rt_err(span, msg).into())
        }
    }
}

/// `os.listdir(p)`: entry names in the order the system gives them, as
/// CPython's. A name that is not UTF-8 is decoded lossily (a listed
/// deviation).
fn listdir<'m>(p: &str, span: Span) -> Exec<'m, Value<'m>> {
    let entries = fs::read_dir(p).map_err(|e| os_error(&e, p, span))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| os_error(&e, p, span))?;
        names.push(Value::str(&entry.file_name().to_string_lossy()));
    }
    Ok(Value::list(names))
}

/// `posixpath.join`: an absolute part restarts the path; a `/` joins
/// parts unless the path is empty or already ends in one.
fn join(first: &str, rest: &[&str]) -> String {
    let mut out = first.to_string();
    for part in rest {
        if part.starts_with('/') {
            out.clear();
        } else if !out.is_empty() && !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(part);
    }
    out
}

/// The `OSError` CPython raises for `e` on `p`: the subclass its kind
/// names, holding `errno`, `strerror` and `filename` when the system
/// gave an errno, else a message alone (`text: 'p'`).
fn os_error<'m>(e: &io::Error, p: &str, span: Span) -> super::Unwind<'m> {
    let (kind, errno, strerror) = os_parts(e);
    let arg = match errno {
        Some(errno) => ExcArg::Errno {
            errno,
            strerror,
            filename: Some(p.to_string()),
        },
        None => ExcArg::One(Value::str(&format!("{strerror}: {}", str_repr(p)))),
    };
    super::Unwind::Raise(Rc::new(Exc { kind, arg }), span)
}

/// The `OSError` subclass `e`'s kind names, its errno if the system gave
/// one, and the system's text (`No such file or directory`).
fn os_parts(e: &io::Error) -> (ExcKind, Option<i32>, String) {
    let kind = match e.kind() {
        io::ErrorKind::NotFound => ExcKind::FileNotFound,
        io::ErrorKind::PermissionDenied => ExcKind::Permission,
        io::ErrorKind::IsADirectory => ExcKind::IsADirectory,
        io::ErrorKind::NotADirectory => ExcKind::NotADirectory,
        _ => ExcKind::Os,
    };
    let text = e.to_string();
    let detail = text
        .rsplit_once(" (os error ")
        .map_or(text.as_str(), |(d, _)| d);
    (kind, e.raw_os_error(), detail.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_follows_posixpath() {
        assert_eq!(join("a", &[]), "a");
        assert_eq!(join("a", &["b", "c"]), "a/b/c");
        assert_eq!(join("a/", &["b"]), "a/b");
        assert_eq!(join("", &["b"]), "b");
        assert_eq!(join("a", &[""]), "a/");
        assert_eq!(join("a", &["/b", "c"]), "/b/c");
    }

    #[cfg(unix)]
    #[test]
    fn os_errors_read_as_cpython_on_posix() {
        let missing = io::Error::from_raw_os_error(2);
        let text = "No such file or directory".to_string();
        assert_eq!(os_parts(&missing), (ExcKind::FileNotFound, Some(2), text));
        let kinds = [
            (13, ExcKind::Permission),
            (20, ExcKind::NotADirectory),
            (21, ExcKind::IsADirectory),
            (36, ExcKind::Os),
        ];
        for (errno, kind) in kinds {
            assert_eq!(os_parts(&io::Error::from_raw_os_error(errno)).0, kind);
        }
        let synthetic = io::Error::from(io::ErrorKind::InvalidInput);
        assert_eq!(os_parts(&synthetic).1, None);
    }
}
