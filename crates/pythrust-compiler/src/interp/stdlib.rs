//! Module functions ([`crate::stdlib`]): `sys.argv` from the
//! interpreter's state, `os` and `os.path` over `std::fs`.

use std::{fs, io};

use super::builtins::Args;
use super::value::{str_repr, Value};
use super::{raised, rt_err, Exec, Interp};
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

impl<'m> Interp<'m, '_> {
    /// Calls `f`; inference checked the arguments' count and types.
    pub(super) fn std(&mut self, f: StdFn, args: &Args<'m>) -> Exec<'m, Value<'m>> {
        let span = args.span;
        if !args.keywords.is_empty() {
            let msg = format!("internal: `{}` takes no keyword arguments", f.name());
            return Err(rt_err(span, msg).into());
        }
        let paths = args
            .positional
            .iter()
            .map(|v| path(v, span))
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
/// names, and `[Errno N] text: 'p'`.
fn os_error<'m>(e: &io::Error, p: &str, span: Span) -> super::Unwind<'m> {
    let (kind, message) = os_message(e, p);
    raised(kind, message, span)
}

fn os_message(e: &io::Error, p: &str) -> (ExcKind, String) {
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
    let message = match e.raw_os_error() {
        Some(n) => format!("[Errno {n}] {detail}: {}", str_repr(p)),
        None => format!("{detail}: {}", str_repr(p)),
    };
    (kind, message)
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
        assert_eq!(
            os_message(&missing, "nope"),
            (
                ExcKind::FileNotFound,
                "[Errno 2] No such file or directory: 'nope'".to_string()
            )
        );
        let denied = io::Error::from_raw_os_error(13);
        assert_eq!(
            os_message(&denied, "it's"),
            (
                ExcKind::Permission,
                "[Errno 13] Permission denied: \"it's\"".to_string()
            )
        );
        let not_dir = io::Error::from_raw_os_error(20);
        assert_eq!(os_message(&not_dir, "f").0, ExcKind::NotADirectory);
        let is_dir = io::Error::from_raw_os_error(21);
        assert_eq!(os_message(&is_dir, "d").0, ExcKind::IsADirectory);
        let other = io::Error::from_raw_os_error(36);
        assert_eq!(os_message(&other, "x").0, ExcKind::Os);
    }
}
