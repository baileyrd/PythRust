//! Module functions ([`crate::stdlib`]): `sys.argv` from the
//! interpreter's state, `os` and `os.path` over `std::fs`, and `open`
//! with its file's `read` (`close` is in [`super::methods`]), and
//! `hashlib.sha256(data).hexdigest()`.

use std::cell::RefCell;
use std::rc::Rc;
use std::{fs, io};

use super::builtins::Args;
use super::value::{str_repr, Exc, ExcArg, File, Value};
use super::{raised, rt_err, Exec, Interp};
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::{Mode, StdFn};
use crate::textio::{self, Data, FileError};

use super::methods::Name;

/// The runtime's SHA-256, one source for both backends.
#[path = "../../../pythrust-rt/src/sha256.rs"]
mod sha256;

impl<'m> Interp<'m, '_> {
    /// Calls `f`; inference checked the arguments' count and types.
    pub(super) fn std(&mut self, f: StdFn, args: &Args<'m>) -> Exec<'m, Value<'m>> {
        let span = args.span;
        if f == StdFn::Executor {
            return Ok(Value::Executor);
        }
        if f == StdFn::Open {
            return open(args, span);
        }
        if let (StdFn::Sha256, [Value::Bytes(data)]) = (f, args.positional.as_slice()) {
            return Ok(Value::str(&sha256::hex(&sha256::digest(data))));
        }
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
                let len = fs::metadata(p)
                    .map_err(|e| os_error(&e, Some(p), span))?
                    .len();
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

/// `open(p, mode, encoding=...)` (D91, D95): a NUL in `p` is CPython's
/// untracked `ValueError`, before any file system call.
fn open<'m>(args: &Args<'m>, span: Span) -> Exec<'m, Value<'m>> {
    let keyword = |name: &str| {
        let found = args.keywords.iter().find(|(kw, _)| kw.name == name);
        found.map(|(_, v)| v)
    };
    let text = |v: Option<&Value<'m>>| match v {
        Some(Value::Str(s)) => Some(s.to_string()),
        _ => None,
    };
    let p = text(args.positional.first());
    let mode = text(args.positional.get(1).or_else(|| keyword("mode")));
    let mode = mode.as_deref().map_or(Some(Mode::R), Mode::parse);
    let (Some(p), Some(mode)) = (p, mode) else {
        return Err(rt_err(span, "internal: `open` of no path or mode".to_string()).into());
    };
    let encoding = text(keyword("encoding")).unwrap_or_else(|| "utf-8".to_string());
    if p.contains('\0') {
        return Err(raised(ExcKind::Value, NUL.to_string(), span));
    }
    let file = textio::File::open(&p, mode, &encoding).map_err(|e| os_error(&e, Some(&p), span))?;
    let file = RefCell::new(file);
    Ok(Value::File(Rc::new(File { file })))
}

/// CPython's message for a path holding a NUL: Windows converts the
/// path to wide characters, POSIX to bytes.
const NUL: &str = if cfg!(windows) {
    "embedded null character"
} else {
    "embedded null byte"
};

/// Calls method `name` of file `f` with `args`, which inference checked
/// against its mode (D91, D92).
pub(super) fn file_call<'m>(
    name: Name,
    f: &File,
    args: Vec<Value<'m>>,
    span: Span,
) -> Exec<'m, Value<'m>> {
    let mut file = f.file.borrow_mut();
    let done = match (name, args.as_slice()) {
        (Name::Read, []) => file.read().map(|data| match data {
            Data::Text(s) => Value::str(&s),
            Data::Bytes(b) => Value::Bytes(Rc::from(b)),
        }),
        (Name::Readline, []) => file.readline().map(|s| Value::str(&s)),
        (Name::Write, [Value::Str(s)]) => file.write_text(s).map(count),
        (Name::Write, [Value::Bytes(b)]) => file.write_bytes(b).map(count),
        (Name::Flush, []) => file.flush().map(|()| Value::None),
        (Name::Close, []) => file.close().map(|()| Value::None).map_err(FileError::Os),
        _ => {
            let msg = format!("internal: file method `{}` here", name.as_str());
            return Err(rt_err(span, msg).into());
        }
    };
    done.map_err(|e| file_error(e, span))
}

/// The lines of text file `f` left to read, for `for` and the builtins
/// that read an iterable whole.
pub(super) fn lines<'m>(f: &File, span: Span) -> Exec<'m, Vec<Value<'m>>> {
    let mut file = f.file.borrow_mut();
    let mut out = Vec::new();
    loop {
        let line = file.readline().map_err(|e| file_error(e, span))?;
        if line.is_empty() {
            return Ok(out);
        }
        out.push(Value::str(&line));
    }
}

/// The next line of text file `f`, `None` at the end: a step of `for`.
pub(super) fn next_line<'m>(f: &File, span: Span) -> Exec<'m, Option<Value<'m>>> {
    let line = f.file.borrow_mut().readline();
    let line = line.map_err(|e| file_error(e, span))?;
    Ok((!line.is_empty()).then(|| Value::str(&line)))
}

/// A count as an `int`.
fn count<'m>(n: usize) -> Value<'m> {
    Value::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// The exception a failed file operation raises (D93, D96).
fn file_error<'m>(e: FileError, span: Span) -> super::Unwind<'m> {
    match e {
        FileError::Os(e) => os_error(&e, None, span),
        FileError::Decode(msg) => raised(ExcKind::UnicodeDecode, msg, span),
        FileError::Closed(msg) => raised(ExcKind::Value, msg.to_string(), span),
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
    let entries = fs::read_dir(p).map_err(|e| os_error(&e, Some(p), span))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| os_error(&e, Some(p), span))?;
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
fn os_error<'m>(e: &io::Error, p: Option<&str>, span: Span) -> super::Unwind<'m> {
    let (kind, errno, strerror) = os_parts(e);
    let arg = match errno {
        Some(errno) => ExcArg::Errno {
            errno,
            strerror,
            filename: p.map(str::to_string),
        },
        None => ExcArg::One(Value::str(&match p {
            Some(p) => format!("{strerror}: {}", str_repr(p)),
            None => strerror,
        })),
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
