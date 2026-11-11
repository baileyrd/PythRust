//! The standard library (M6a): the modules a program may `import` and
//! the names each exposes, and the builtin `open` with its file's
//! methods. A Python-compatible surface over the Rust
//! runtime (`docs/roadmap.md#phase-5-m6-plan`): each name behaves as in
//! CPython or is on grammar.md's deviations list.
//!
//! Stages see no modules. The parser resolves `module.attr` after
//! `import module` to one name with a dot in it (`sys.argv`), which no
//! user name can be, so a module variable is an ordinary global and a
//! module function a builtin. `import m` assigns each of `m`'s variables
//! from its initializer, a call to a [`StdFn`] only the parser can write.

use crate::exc::ExcKind;
pub use crate::textio::{Kind, Mode};

/// A module and the dotted names it exposes.
pub struct ModuleDef {
    pub name: &'static str,
    /// Variables, each with the function that makes its value.
    pub vars: &'static [(&'static str, StdFn)],
    /// Functions, called by their [`StdFn::name`].
    pub funcs: &'static [StdFn],
}

/// Why `run` refuses a program that imports a Rust binding (M6b): the
/// interpreter cannot call Rust.
pub const RUST_RUN_GAP: &str = "run cannot call Rust bindings yet; use build";

/// The importable modules.
pub const MODULES: &[ModuleDef] = &[
    ModuleDef {
        name: "sys",
        vars: &[("sys.argv", StdFn::Argv)],
        funcs: &[],
    },
    ModuleDef {
        name: "os",
        vars: &[],
        funcs: &[
            StdFn::Listdir,
            StdFn::Join,
            StdFn::Isdir,
            StdFn::Isfile,
            StdFn::Islink,
            StdFn::Getsize,
        ],
    },
    ModuleDef {
        name: "hashlib",
        vars: &[],
        funcs: &[StdFn::Sha256],
    },
    // Imported as `import concurrent.futures` only (M6c).
    ModuleDef {
        name: "concurrent",
        vars: &[],
        funcs: &[StdFn::Executor],
    },
];

/// The one module a program imports by a dotted name: `concurrent`,
/// whose names all sit under `concurrent.futures`.
pub const DOTTED_IMPORT: (&str, &str) = ("concurrent", "futures");

/// A runtime function behind a module name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdFn {
    /// `sys.argv`'s value: a new `list[str]` of the program's arguments.
    Argv,
    /// `os.listdir(path='.')`: a new `list[str]` of the entry names.
    Listdir,
    /// `os.path.join(a, *p)`: POSIX rules (a listed deviation on Windows).
    Join,
    /// `os.path.isdir(p)`, following links; `False` on any error.
    Isdir,
    /// `os.path.isfile(p)`, following links; `False` on any error.
    Isfile,
    /// `os.path.islink(p)`; `False` on any error.
    Islink,
    /// `os.path.getsize(p)`, following links.
    Getsize,
    /// The builtin `open(p, mode, encoding=...)` (D91): its operands
    /// are the path, the mode literal and the encoding to show.
    Open,
    /// `f.read()` of a file: the rest of it, `str` or `bytes`.
    Read,
    /// `f.readline()` of a text file, `""` at the end; also each step
    /// of `for line in f`.
    Readline,
    /// `f.write(s)` of a writer: how many code points or bytes.
    Write,
    /// `f.flush()` of a file.
    Flush,
    /// `f.close()` of a file; closing it again does nothing.
    Close,
    /// `hashlib.sha256(data).hexdigest()` of `bytes`, one call: the hash
    /// object is not a value yet ([`StdFn::chain`]).
    Sha256,
    /// `concurrent.futures.ThreadPoolExecutor()`: an executor, used only
    /// as a `with` block's value (M6c).
    Executor,
    /// `s.split()` or `s.split(sep)` (D53): a new `list[str]`. An empty
    /// `sep` raises a `ValueError`, untracked.
    Split,
    /// `sep.join(xs)` of a `list[str]` (D53).
    StrJoin,
    /// `s.strip()` or `s.strip(chars)`.
    Strip,
    /// `s.lstrip()` or `s.lstrip(chars)`.
    Lstrip,
    /// `s.rstrip()` or `s.rstrip(chars)`.
    Rstrip,
    /// `s.replace(old, new)`.
    Replace,
    /// `s.startswith(prefix)`.
    Startswith,
    /// `s.endswith(suffix)`.
    Endswith,
    /// `s.find(sub)`: a code-point index, or -1.
    Find,
    /// `s.rfind(sub)`: a code-point index, or -1.
    Rfind,
    /// `s.index(sub)`: as `find`, raising a tracked `ValueError` when absent (D50).
    Index,
    /// `s.rindex(sub)`: as `rfind`, raising a tracked `ValueError` when absent.
    Rindex,
    /// `s.count(sub)`: non-overlapping matches.
    Count,
    /// `s.upper()`: full Unicode case mapping (D49).
    Upper,
    /// `s.lower()`: full Unicode case mapping (D49).
    Lower,
    /// The builtin `ord(c)` (D56): a fault unless `c` is one character.
    Ord,
    /// The builtin `chr(n)` (D56): raises `ValueError` out of range or
    /// for a surrogate.
    Chr,
    /// The builtin `sum` of floats in a list, tuple or view, from
    /// `start`: CPython's compensated sum (D57).
    FloatSum,
    /// `xs.index(x)` (D70): the first position of an item `==` to `x`,
    /// raising a tracked `ValueError` naming `x` when there is none (D71).
    ListIndex,
    /// The first position of an item of a list `==` to `x`, or -1: what
    /// `xs.remove(x)` removes. Only lowering calls it.
    ListFind,
    /// `xs.count(x)` (D70): how many items are `==` to `x`.
    ListCount,
}

impl StdFn {
    pub const ALL: [StdFn; 36] = [
        StdFn::Argv,
        StdFn::Listdir,
        StdFn::Join,
        StdFn::Isdir,
        StdFn::Isfile,
        StdFn::Islink,
        StdFn::Getsize,
        StdFn::Open,
        StdFn::Read,
        StdFn::Readline,
        StdFn::Write,
        StdFn::Flush,
        StdFn::Close,
        StdFn::Sha256,
        StdFn::Executor,
        StdFn::Split,
        StdFn::StrJoin,
        StdFn::Strip,
        StdFn::Lstrip,
        StdFn::Rstrip,
        StdFn::Replace,
        StdFn::Startswith,
        StdFn::Endswith,
        StdFn::Find,
        StdFn::Rfind,
        StdFn::Index,
        StdFn::Rindex,
        StdFn::Count,
        StdFn::Upper,
        StdFn::Lower,
        StdFn::Ord,
        StdFn::Chr,
        StdFn::FloatSum,
        StdFn::ListIndex,
        StdFn::ListFind,
        StdFn::ListCount,
    ];

    /// The builtin name stages call it by: the dotted name a program
    /// writes. An initializer's name has a `__` part, so no program can
    /// write it.
    pub fn name(self) -> &'static str {
        match self {
            StdFn::Argv => "sys.__argv__",
            StdFn::Listdir => "os.listdir",
            StdFn::Join => "os.path.join",
            StdFn::Isdir => "os.path.isdir",
            StdFn::Isfile => "os.path.isfile",
            StdFn::Islink => "os.path.islink",
            StdFn::Getsize => "os.path.getsize",
            StdFn::Open => "open",
            StdFn::Read => "file.read",
            StdFn::Readline => "file.readline",
            StdFn::Write => "file.write",
            StdFn::Flush => "file.flush",
            StdFn::Close => "file.close",
            StdFn::Sha256 => "hashlib.sha256",
            StdFn::Executor => "concurrent.futures.ThreadPoolExecutor",
            StdFn::Split => "str.split",
            StdFn::StrJoin => "str.join",
            StdFn::Strip => "str.strip",
            StdFn::Lstrip => "str.lstrip",
            StdFn::Rstrip => "str.rstrip",
            StdFn::Replace => "str.replace",
            StdFn::Startswith => "str.startswith",
            StdFn::Endswith => "str.endswith",
            StdFn::Find => "str.find",
            StdFn::Rfind => "str.rfind",
            StdFn::Index => "str.index",
            StdFn::Rindex => "str.rindex",
            StdFn::Count => "str.count",
            StdFn::Upper => "str.upper",
            StdFn::Lower => "str.lower",
            StdFn::Ord => "builtins.ord",
            StdFn::Chr => "builtins.chr",
            StdFn::FloatSum => "builtins.sum",
            StdFn::ListIndex => "list.index",
            StdFn::ListFind => "list.__find__",
            StdFn::ListCount => "list.count",
        }
    }

    /// The method a call to this function must be followed by, which
    /// the call stands for with it: `hexdigest` after `hashlib.sha256`.
    pub fn chain(self) -> Option<&'static str> {
        match self {
            StdFn::Sha256 => Some("hexdigest"),
            _ => None,
        }
    }

    /// A file's method called `attr` (D91).
    pub fn file_method(attr: &str) -> Option<StdFn> {
        match attr {
            "read" => Some(StdFn::Read),
            "readline" => Some(StdFn::Readline),
            "write" => Some(StdFn::Write),
            "flush" => Some(StdFn::Flush),
            "close" => Some(StdFn::Close),
            _ => None,
        }
    }

    /// What file method `self` of a file of `kind` raises, tracked
    /// (D93, D96): an `OSError`, and a `UnicodeDecodeError` reading
    /// text. Closing a reader raises nothing; using a closed file raises
    /// an untracked `ValueError` (D21).
    pub fn file_raises(self, kind: Kind) -> &'static [ExcKind] {
        match (self, kind.is_text(), kind.writes()) {
            (StdFn::Read | StdFn::Readline, true, false) => &[ExcKind::Os, ExcKind::UnicodeDecode],
            (StdFn::Close, _, false) => &[],
            _ => &[ExcKind::Os],
        }
    }

    /// A `str`'s method called `attr` (D53).
    pub fn str_method(attr: &str) -> Option<StdFn> {
        match attr {
            "split" => Some(StdFn::Split),
            "join" => Some(StdFn::StrJoin),
            "strip" => Some(StdFn::Strip),
            "lstrip" => Some(StdFn::Lstrip),
            "rstrip" => Some(StdFn::Rstrip),
            "replace" => Some(StdFn::Replace),
            "startswith" => Some(StdFn::Startswith),
            "endswith" => Some(StdFn::Endswith),
            "find" => Some(StdFn::Find),
            "rfind" => Some(StdFn::Rfind),
            "index" => Some(StdFn::Index),
            "rindex" => Some(StdFn::Rindex),
            "count" => Some(StdFn::Count),
            "upper" => Some(StdFn::Upper),
            "lower" => Some(StdFn::Lower),
            _ => None,
        }
    }

    /// Whether a call may leave an exception pending, tracked or not:
    /// what [`StdFn::raises`] names, `split`'s untracked `ValueError`
    /// for an empty separator (D53), and a file method's (D96).
    pub fn may_raise(self) -> bool {
        self.raises().is_some()
            || matches!(
                self,
                StdFn::Split | StdFn::Readline | StdFn::Write | StdFn::Flush | StdFn::Close
            )
    }

    /// The class of what it may raise, tracked in `raises` (D8). An
    /// `OSError` is raised as the subclass the error's kind names. A
    /// file method's depends on the file's mode
    /// ([`StdFn::file_raises`]); `Read` here is a binary file's.
    pub fn raises(self) -> Option<ExcKind> {
        match self {
            StdFn::Listdir | StdFn::Getsize | StdFn::Open | StdFn::Read => Some(ExcKind::Os),
            StdFn::Index | StdFn::Rindex | StdFn::Chr | StdFn::ListIndex => Some(ExcKind::Value),
            StdFn::Argv
            | StdFn::Join
            | StdFn::Isdir
            | StdFn::Isfile
            | StdFn::Islink
            | StdFn::Readline
            | StdFn::Write
            | StdFn::Flush
            | StdFn::Close
            | StdFn::Sha256
            | StdFn::Executor
            | StdFn::Split
            | StdFn::StrJoin
            | StdFn::Strip
            | StdFn::Lstrip
            | StdFn::Rstrip
            | StdFn::Replace
            | StdFn::Startswith
            | StdFn::Endswith
            | StdFn::Find
            | StdFn::Rfind
            | StdFn::Count
            | StdFn::Upper
            | StdFn::Lower
            | StdFn::Ord
            | StdFn::FloatSum
            | StdFn::ListFind
            | StdFn::ListCount => None,
        }
    }

    /// The one parameter a call may pass by keyword, as CPython's
    /// signature names it: `os.listdir(path='.')`. The others take
    /// positional arguments only (grammar.md, omissions).
    pub fn keyword(self) -> Option<&'static str> {
        match self {
            StdFn::Listdir => Some("path"),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<StdFn> {
        StdFn::ALL.into_iter().find(|f| f.name() == name)
    }
}

/// The module named `name`.
pub fn module(name: &str) -> Option<&'static ModuleDef> {
    MODULES.iter().find(|m| m.name == name)
}

impl ModuleDef {
    /// Every dotted name a program may write, sorted, for messages.
    pub fn names(&self) -> Vec<&'static str> {
        let vars = self.vars.iter().map(|(n, _)| *n);
        let mut names: Vec<&str> = vars.chain(self.funcs.iter().map(|f| f.name())).collect();
        names.sort_unstable();
        names
    }

    /// Whether `path` is a name a program may write.
    pub fn exposes(&self, path: &str) -> bool {
        self.names().contains(&path)
    }

    /// Whether some exposed name continues `path` (`os.path` for
    /// `os.path.join`).
    pub fn continues(&self, path: &str) -> bool {
        let prefix = format!("{path}.");
        self.names().iter().any(|n| n.starts_with(&prefix))
    }
}

/// The importable module names, sorted, for messages.
pub fn module_names() -> String {
    let (top, sub) = DOTTED_IMPORT;
    let dotted = format!("{top}.{sub}");
    let shown = |m: &ModuleDef| {
        if m.name == top {
            dotted.as_str()
        } else {
            m.name
        }
    };
    let mut names: Vec<&str> = MODULES.iter().map(shown).collect();
    names.sort_unstable();
    names.join(", ")
}

/// The error for `import name` of a module that is neither standard
/// nor a binding the project's `pythrust.toml` names (M6d); `why` says
/// what was found instead, as "no pythrust.toml names it".
pub fn not_standard(name: &str, why: &str) -> String {
    format!(
        "`{name}` is not a standard module, and {why}; the standard modules are: {}",
        module_names()
    )
}

/// CPython 3.13's `sys.stdlib_module_names`: every top-level standard
/// module name, implemented here or not (D81). A local module may not
/// take one, and importing one Pythrust lacks is "not supported yet".
const PYTHON_MODULES: &str = "\
__future__ _abc _aix_support _android_support _apple_support _ast _asyncio _bisect _blake2
_bz2 _codecs _codecs_cn _codecs_hk _codecs_iso2022 _codecs_jp _codecs_kr _codecs_tw
_collections _collections_abc _colorize _compat_pickle _compression _contextvars _csv
_ctypes _curses _curses_panel _datetime _dbm _decimal _elementtree _frozen_importlib
_frozen_importlib_external _functools _gdbm _hashlib _heapq _imp _interpchannels
_interpqueues _interpreters _io _ios_support _json _locale _lsprof _lzma _markupbase _md5
_multibytecodec _multiprocessing _opcode _opcode_metadata _operator _osx_support
_overlapped _pickle _posixshmem _posixsubprocess _py_abc _pydatetime _pydecimal _pyio
_pylong _pyrepl _queue _random _scproxy _sha1 _sha2 _sha3 _signal _sitebuiltins _socket
_sqlite3 _sre _ssl _stat _statistics _string _strptime _struct _suggestions _symtable
_sysconfig _thread _threading_local _tkinter _tokenize _tracemalloc _typing _uuid
_warnings _weakref _weakrefset _winapi _wmi _zoneinfo abc antigravity argparse array ast
asyncio atexit base64 bdb binascii bisect builtins bz2 cProfile calendar cmath cmd code
codecs codeop collections colorsys compileall concurrent configparser contextlib
contextvars copy copyreg csv ctypes curses dataclasses datetime dbm decimal difflib dis
doctest email encodings ensurepip enum errno faulthandler fcntl filecmp fileinput fnmatch
fractions ftplib functools gc genericpath getopt getpass gettext glob graphlib grp gzip
hashlib heapq hmac html http idlelib imaplib importlib inspect io ipaddress itertools json
keyword linecache locale logging lzma mailbox marshal math mimetypes mmap modulefinder
msvcrt multiprocessing netrc nt ntpath nturl2path numbers opcode operator optparse os
pathlib pdb pickle pickletools pkgutil platform plistlib poplib posix posixpath pprint
profile pstats pty pwd py_compile pyclbr pydoc pydoc_data pyexpat queue quopri random re
readline reprlib resource rlcompleter runpy sched secrets select selectors shelve shlex
shutil signal site smtplib socket socketserver sqlite3 sre_compile sre_constants sre_parse
ssl stat statistics string stringprep struct subprocess symtable sys sysconfig syslog
tabnanny tarfile tempfile termios textwrap this threading time timeit tkinter token
tokenize tomllib trace traceback tracemalloc tty turtle turtledemo types typing
unicodedata unittest urllib uuid venv warnings wave weakref webbrowser winreg winsound
wsgiref xml xmlrpc zipapp zipfile zipimport zlib zoneinfo";

/// Whether `name` is a top-level module of CPython's standard library.
pub fn is_python_module(name: &str) -> bool {
    PYTHON_MODULES.split_ascii_whitespace().any(|n| n == name)
}

/// The error for `import name` that names no module the program can
/// import: a standard one Pythrust lacks is "not supported yet" (D81).
pub fn not_found(name: &str) -> String {
    if is_python_module(name) {
        return format!(
            "the standard module `{name}` is not supported yet; the standard modules are: {}",
            module_names()
        );
    }
    not_standard(name, "no pythrust.toml names it")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_modules_are_known_whether_implemented_or_not() {
        for name in ["sys", "json", "__future__", "zoneinfo", "concurrent"] {
            assert!(is_python_module(name), "{name}");
        }
        for name in ["fastfs", "concurrent.futures", "", "Json"] {
            assert!(!is_python_module(name), "{name}");
        }
        assert!(not_found("json").contains("`json` is not supported yet"));
        assert!(not_found("mymod").contains("no pythrust.toml names it"));
    }

    #[test]
    fn every_initializer_is_unwritable_and_found_by_name() {
        for f in StdFn::ALL {
            assert_eq!(StdFn::from_name(f.name()), Some(f));
        }
        for m in MODULES {
            for (var, init) in m.vars {
                assert!(var.starts_with(&format!("{}.", m.name)), "{var}");
                assert!(init.name().contains("__"), "{}", init.name());
                assert!(!m.exposes(init.name()));
            }
        }
    }

    #[test]
    fn functions_belong_to_their_module() {
        for m in MODULES {
            for f in m.funcs {
                assert!(f.name().starts_with(&format!("{}.", m.name)), "{f:?}");
                assert!(m.exposes(f.name()));
            }
        }
        let Some(os) = module("os") else {
            panic!("os is importable")
        };
        assert!(os.continues("os.path"));
        assert!(!os.exposes("os.path"));
        assert!(os.exposes("os.path.getsize"));
    }

    #[test]
    fn file_and_str_methods_are_unwritable_names() {
        for attr in ["read", "readline", "write", "flush", "close"] {
            let Some(f) = StdFn::file_method(attr) else {
                panic!("{attr} is a file method")
            };
            assert!(f.name().ends_with(&format!(".{attr}")));
            assert!(MODULES.iter().all(|m| !m.exposes(f.name())));
        }
        assert_eq!(StdFn::file_method("seek"), None);
        let str_methods = [
            "split",
            "join",
            "strip",
            "lstrip",
            "rstrip",
            "replace",
            "startswith",
            "endswith",
            "find",
            "rfind",
            "index",
            "rindex",
            "count",
            "upper",
            "lower",
        ];
        for attr in str_methods {
            let Some(f) = StdFn::str_method(attr) else {
                panic!("{attr} is a str method")
            };
            assert_eq!(f.name(), format!("str.{attr}"));
            assert!(MODULES.iter().all(|m| !m.exposes(f.name())));
        }
        assert_eq!(StdFn::str_method("read"), None);
        assert_eq!(StdFn::from_name("open"), Some(StdFn::Open));
    }

    #[test]
    fn sys_exposes_argv_only() {
        let sys = module("sys").map(ModuleDef::names);
        assert_eq!(sys, Some(vec!["sys.argv"]));
        assert!(module("os.path").is_none());
        let Some(sys) = module("sys") else {
            panic!("sys is importable")
        };
        assert!(!sys.continues("sys.argv"));
        assert!(!sys.exposes("sys.path"));
    }
}
