//! `build` for a program that calls Rust bindings (M6b,
//! `docs/research/rust-interop.md#build`): checks each binding's
//! `Cargo.lock`, writes the generated crate into `.pythrust/<stem>/`
//! beside the program, has cargo build it, holds cargo's lock to the
//! bindings', and gives the staticlib to link in place of the plain
//! runtime.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use pythrust_compiler::interop::gen::{self, CRATE};
use pythrust_compiler::interop::lock::{self, Lock};

use crate::Runtime;

/// The oldest Rust that builds `pythrust-rt`: its `rust-version`.
const MIN_RUST: &str = "1.88";

/// Where the runtime's source is, for an installed `pythrust`.
const RT_SRC_ENV: &str = "PYTHRUST_RT_SRC";

/// The staticlib cargo writes for the generated crate.
#[cfg(not(windows))]
const STATICLIB: &str = "libpythrust_bindings.a";
#[cfg(windows)]
const STATICLIB: &str = "pythrust_bindings.lib";

/// The profile `pythrust` was built with, so a debug build keeps the
/// runtime's leak check.
const RELEASE: bool = !cfg!(debug_assertions);

/// A binding the program calls: its name and absolute directory.
pub(crate) struct Crate {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
}

/// Builds the generated crate whose `lib.rs` is `shims` over `crates`,
/// for the executable `out`; gives the runtime to link.
pub(crate) fn build(shims: &str, crates: &[Crate], out: &Path) -> Result<Runtime, String> {
    let runtime = runtime_src()?;
    let mut locks = Vec::new();
    for c in crates {
        locks.push(binding_lock(c)?);
    }
    let named: Vec<(&str, &Lock)> = crates.iter().map(|c| c.name.as_str()).zip(&locks).collect();
    let seed = lock::seed(&named).map_err(|e| format!("pythrust: {e}\n"))?;
    let dir = crate_dir(out)?;
    let deps: Vec<(&str, String)> = crates
        .iter()
        .map(|c| Ok((c.name.as_str(), utf8(&c.dir)?)))
        .collect::<Result<_, String>>()?;
    let deps: Vec<(&str, &str)> = deps.iter().map(|(n, d)| (*n, d.as_str())).collect();
    write(&dir, "Cargo.toml", &gen::manifest(&utf8(&runtime)?, &deps))?;
    write(&dir, "lib.rs", shims)?;
    write(&dir, "Cargo.lock", &seed)?;
    let output = run(cargo_rustc(&dir))?;
    if !output.status.success() {
        return Err(failure(&output, shims, crates));
    }
    let built = read_lock(&dir.join("Cargo.lock"))?;
    lock::verify(&built, &named).map_err(|e| format!("pythrust: {e}\n"))?;
    let native = native_libs(&String::from_utf8_lossy(&output.stderr))?;
    let lib = dir
        .join("target")
        .join(host())
        .join(if RELEASE { "release" } else { "debug" })
        .join(STATICLIB);
    Ok(Runtime { lib, native })
}

/// `pythrust-rt`'s source: `$PYTHRUST_RT_SRC`, or where it was when this
/// `pythrust` was compiled.
fn runtime_src() -> Result<PathBuf, String> {
    let dir = match std::env::var_os(RT_SRC_ENV) {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../pythrust-rt"),
    };
    if !dir.join("Cargo.toml").is_file() {
        return Err(format!(
            "pythrust: the runtime's source is not at {}; set {RT_SRC_ENV} to the pythrust-rt directory\n",
            dir.display()
        ));
    }
    crate::absolute(&dir)
}

/// `c`'s lock, once `c` is checked to be its own workspace, with a
/// committed lock that has no git package and matches its manifest
/// (`cargo metadata --locked` fails when the manifest asks for anything
/// the lock does not pin).
fn binding_lock(c: &Crate) -> Result<Lock, String> {
    let name = &c.name;
    let manifest = c.dir.join("Cargo.toml");
    if !manifest.is_file() {
        let shown = c.dir.display();
        return Err(format!(
            "pythrust: binding `{name}`: no Cargo.toml in {shown}\n"
        ));
    }
    own_workspace(name, &manifest)?;
    let path = c.dir.join("Cargo.lock");
    if !path.is_file() {
        return Err(format!(
            "pythrust: binding `{name}` has no Cargo.lock; run `cargo generate-lockfile` in it and commit the file\n"
        ));
    }
    let lock = read_lock(&path)?;
    lock::refuse_git(name, &lock).map_err(|e| format!("pythrust: {e}\n"))?;
    let mut metadata = cargo();
    metadata
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&manifest);
    let output = run(metadata)?;
    if !output.status.success() {
        return Err(format!(
            "pythrust: binding `{name}`: dependencies differ from its Cargo.lock; update and commit the binding's lockfile\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(lock)
}

/// Refuses a binding inside another workspace, whose lock is that
/// workspace's.
fn own_workspace(name: &str, manifest: &Path) -> Result<(), String> {
    let mut locate = cargo();
    locate
        .args([
            "locate-project",
            "--workspace",
            "--message-format",
            "plain",
            "--manifest-path",
        ])
        .arg(manifest);
    let output = run(locate)?;
    if !output.status.success() {
        return Err(format!(
            "pythrust: binding `{name}`: cargo cannot read its manifest\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if same(&root, manifest)? {
        return Ok(());
    }
    Err(format!(
        "pythrust: binding `{name}` is part of the workspace at {}; add an empty `[workspace]` to its Cargo.toml so it has its own Cargo.lock\n",
        root.display()
    ))
}

fn same(a: &Path, b: &Path) -> Result<bool, String> {
    let canonical = |p: &Path| {
        std::fs::canonicalize(p)
            .map_err(|e| format!("pythrust: cannot resolve {}: {e}\n", p.display()))
    };
    Ok(canonical(a)? == canonical(b)?)
}

fn read_lock(path: &Path) -> Result<Lock, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("pythrust: cannot read {}: {e}\n", path.display()))?;
    lock::parse(&text).map_err(|e| format!("pythrust: {}: {e}\n", path.display()))
}

/// `.pythrust/<name>/` beside the executable `out`, named by its file
/// name without the host's executable extension, which no other
/// program's output in that directory shares (`a.pyr` builds `a`,
/// `a.b.pyr` builds `a.b`, or `a.exe` and `a.b.exe`), created if missing.
fn crate_dir(out: &Path) -> Result<PathBuf, String> {
    let name = match std::env::consts::EXE_EXTENSION {
        "" => out.file_name(),
        _ => out.file_stem(),
    }
    .unwrap_or_default();
    let dir = crate::output_dir(out).join(".pythrust").join(name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("pythrust: cannot create {}: {e}\n", dir.display()))?;
    Ok(dir)
}

/// `path` as the UTF-8 a TOML string must hold.
fn utf8(path: &Path) -> Result<String, String> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        format!(
            "pythrust: {} is not UTF-8, which Cargo.toml needs\n",
            path.display()
        )
    })
}

/// Writes `text` to `dir/name` through a temporary file and a rename,
/// so cargo never reads half a file. Rewriting `lib.rs` every build
/// makes cargo compile the generated crate again, and so report its
/// native libraries again.
fn write(dir: &Path, name: &str, text: &str) -> Result<(), String> {
    let tmp = dir.join(format!(".{name}.tmp"));
    let path = dir.join(name);
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("pythrust: cannot write {}: {e}\n", path.display()))
}

/// Cargo: `$CARGO`, as cargo sets it for what it runs, or `cargo`.
/// Without color, so its output reads plainly when parsed or shown.
fn cargo() -> Command {
    let mut c = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    c.args(["--color", "never"]);
    c
}

fn cargo_rustc(dir: &Path) -> Command {
    let mut c = cargo();
    c.args(["rustc", "--lib", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(dir.join("target"))
        .args(["--target", &host()]);
    if RELEASE {
        c.arg("--release");
    }
    c.args(["--", "--print", "native-static-libs"]);
    c
}

/// The target `pythrust` emits code for, so cargo cannot pick another
/// default (a `windows-gnu` one cannot meet MSVC's `link.exe`).
fn host() -> String {
    target_lexicon::Triple::host().to_string()
}

fn run(mut command: Command) -> Result<Output, String> {
    command.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            format!("pythrust: building Rust bindings needs cargo (Rust {MIN_RUST} or later), which is not installed\n")
        }
        _ => format!("pythrust: cannot run cargo: {e}\n"),
    })
}

/// A failed build, named by what failed: a shim (the stub does not match
/// the Rust function), a binding, or something else, with cargo's output.
fn failure(output: &Output, shims: &str, crates: &[Crate]) -> String {
    let log = String::from_utf8_lossy(&output.stderr);
    let head = match failed_crate(&log) {
        Some(name) if name == CRATE => match shim_line(&log).and_then(|l| gen::stub_of(shims, l)) {
            Some(stub) => format!("{stub} does not match the Rust function"),
            None => "pythrust: the generated bindings crate failed to build".to_string(),
        },
        Some(name) if crates.iter().any(|c| c.name == name) => {
            format!("pythrust: binding `{name}` failed to build")
        }
        _ => match crates {
            [only] => format!("pythrust: binding `{}` failed to build", only.name),
            _ => "pythrust: the Rust bindings failed to build".to_string(),
        },
    };
    format!("{head}\n{log}")
}

/// The crate cargo names in "could not compile `name`".
fn failed_crate(log: &str) -> Option<&str> {
    let (_, rest) = log.split_once("could not compile `")?;
    rest.split('`').next()
}

/// The line of the generated `lib.rs` rustc's first error points at.
fn shim_line(log: &str) -> Option<usize> {
    log.lines()
        .filter_map(|l| l.trim_start().strip_prefix("--> "))
        .find_map(|at| {
            let (_, rest) = at.rsplit_once("lib.rs:")?;
            rest.split(':').next()?.parse().ok()
        })
}

/// The system libraries rustc reports for the staticlib.
fn native_libs(log: &str) -> Result<Vec<String>, String> {
    log.lines()
        .find_map(|l| l.split_once("native-static-libs: ").map(|(_, libs)| libs))
        .map(|libs| libs.split_whitespace().map(str::to_string).collect())
        .ok_or_else(|| {
            "pythrust: cargo did not report the bindings' native libraries\n".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_crate_and_shim_line_read_cargos_output() {
        let log = "error[E0308]: mismatched types\n --> lib.rs:7:5\n  |\n\
                   error: could not compile `pythrust_bindings` (lib) due to 1 previous error\n";
        assert_eq!(failed_crate(log), Some("pythrust_bindings"));
        assert_eq!(shim_line(log), Some(7));
        let windows = "  --> C:\\x\\.pythrust\\p\\lib.rs:12:9\n";
        assert_eq!(shim_line(windows), Some(12));
        assert_eq!(failed_crate("error: failed to download"), None);
    }

    #[test]
    fn the_named_minimum_is_the_runtimes_rust_version() {
        let manifest = include_str!("../../pythrust-rt/Cargo.toml");
        assert!(manifest.contains(&format!("\nrust-version = \"{MIN_RUST}\"\n")));
    }

    #[test]
    fn native_libs_come_from_rustcs_note() {
        let log = "   Compiling x\nnote: native-static-libs: -lgcc_s -lutil  -lc\n";
        assert_eq!(
            native_libs(log),
            Ok(vec!["-lgcc_s".into(), "-lutil".into(), "-lc".into()])
        );
        assert!(native_libs("nothing").is_err());
    }
}
