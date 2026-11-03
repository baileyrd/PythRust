//! `build` for a program that calls Rust bindings (M6b,
//! `docs/research/rust-interop.md#build`): writes the generated crate
//! into `.pythrust/<stem>/` beside the program, resolves it from the
//! project's `pythrust.lock` and refuses any entry the lock does not
//! cover (M6d), has cargo build it with `--locked`, and gives the
//! staticlib to link in place of the plain runtime.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use pythrust_compiler::interop::gen::{self, CRATE, MIN_RUST};

use crate::json::{self, Json};
use crate::lockfile::{self, Hold};
use crate::project::Found;
use crate::Runtime;

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
#[derive(Debug, Clone)]
pub(crate) struct Crate {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
}

/// A binding cargo can depend on: its name, its Cargo package's name,
/// which may differ (`fastfs = { path = ... }` over package
/// `fastfs-binding`), and its directory as UTF-8.
pub(crate) struct Binding {
    name: String,
    package: String,
    dir: String,
}

impl Binding {
    fn dep(&self) -> gen::Dep<'_> {
        gen::Dep {
            name: &self.name,
            package: &self.package,
            dir: &self.dir,
        }
    }
}

/// Builds the generated crate whose `lib.rs` is `shims` over `crates`,
/// for the executable `out`, from the lock of `project`; gives the
/// runtime to link.
pub(crate) fn build(
    shims: &str,
    crates: &[Crate],
    out: &Path,
    project: &Found,
) -> Result<Runtime, String> {
    let runtime = runtime_src()?;
    // The project lock first, then the build lock: never the other way.
    let held_project = lockfile::hold(project.dir(), Hold::Shared)?;
    let bindings = crates.iter().map(binding).collect::<Result<Vec<_>, _>>()?;
    let locked = lockfile::read(project)?;
    let dir = crate_dir(out)?;
    let held_build = lockfile::hold_build(&dir)?;
    write(&dir, "Cargo.toml", &manifest(&runtime, &bindings)?)?;
    write(&dir, "lib.rs", shims)?;
    // Resolved from the lock first: a program importing only some
    // bindings needs only part of it, which `--locked` alone refuses.
    let resolved = lockfile::resolve_in(&dir, Some(&locked.text))?;
    lockfile::check_covered(&locked, &resolved, &dir.join("Cargo.lock"))?;
    let output = run(cargo_rustc(&dir))?;
    if !output.status.success() {
        return Err(failure(&output, shims, &bindings));
    }
    let profile = dir
        .join("target")
        .join(host())
        .join(if RELEASE { "release" } else { "debug" });
    let mut native = search_paths(&String::from_utf8_lossy(&output.stdout))?;
    native.extend(native_libs(&String::from_utf8_lossy(&output.stderr))?);
    Ok(Runtime {
        lib: profile.join(STATICLIB),
        native,
        _held: vec![held_build, held_project],
    })
}

/// The generated crate's `Cargo.toml` over `bindings` and the runtime
/// at `runtime`.
pub(crate) fn manifest(runtime: &Path, bindings: &[Binding]) -> Result<String, String> {
    let deps: Vec<gen::Dep> = bindings.iter().map(Binding::dep).collect();
    Ok(gen::manifest(&utf8(runtime)?, &deps))
}

/// The linker arguments for the library search paths this build's build
/// scripts add (`rustc-link-search`, or `-L` in `rustc-flags`), which
/// `native-static-libs` leaves out: without them the final link would
/// miss a library there or find another of its name. Taken from cargo's
/// `build-script-executed` messages for this invocation (`linked_paths`,
/// fresh or cached), never from older outputs left in the target dir.
fn search_paths(messages: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    for line in messages.lines() {
        if !line.contains("\"reason\":\"build-script-executed\"") {
            continue;
        }
        let paths = json::parse(line)
            .and_then(|m| linked_paths(&m))
            .ok_or_else(|| format!("pythrust: cannot read cargo's message `{line}`\n"))?;
        args.extend(paths.iter().filter_map(|p| search_arg(p)));
    }
    args.dedup();
    Ok(args)
}

/// The `linked_paths` of a `build-script-executed` message.
fn linked_paths(message: &Json) -> Option<Vec<String>> {
    let paths = message.get("linked_paths")?.as_arr()?;
    paths
        .iter()
        .map(|p| p.as_str().map(str::to_string))
        .collect()
}

/// The linker argument for one `[KIND=]PATH` search path; none for the
/// kinds that name Rust crates, which the staticlib already holds.
fn search_arg(value: &str) -> Option<String> {
    let (kind, path) = match value.split_once('=') {
        Some((kind @ ("native" | "all" | "framework" | "dependency" | "crate"), path)) => {
            (kind, path)
        }
        _ => ("all", value),
    };
    match kind {
        "dependency" | "crate" => None,
        "framework" if cfg!(target_os = "macos") => Some(format!("-F{path}")),
        "framework" => None,
        _ if cfg!(windows) => Some(format!("/LIBPATH:{path}")),
        _ => Some(format!("-L{path}")),
    }
}

/// `pythrust-rt`'s source: `$PYTHRUST_RT_SRC`, or where it was when this
/// `pythrust` was compiled.
pub(crate) fn runtime_src() -> Result<PathBuf, String> {
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

/// `c` checked for cargo: a crate that is its own workspace, as
/// `pythrust.lock` resolves each binding (its own `Cargo.lock`, if any,
/// is ignored), with its package's name.
pub(crate) fn binding(c: &Crate) -> Result<Binding, String> {
    let name = &c.name;
    let manifest = c.dir.join("Cargo.toml");
    if !manifest.is_file() {
        let shown = c.dir.display();
        return Err(format!(
            "pythrust: binding `{name}`: no Cargo.toml in {shown}\n"
        ));
    }
    own_workspace(name, &manifest)?;
    Ok(Binding {
        name: name.clone(),
        package: package_name(name, &manifest)?,
        dir: utf8(&c.dir)?,
    })
}

/// The package name of the binding `name` at `manifest`, from `cargo
/// metadata --no-deps`, which needs no lock and writes none: the
/// package whose `manifest_path` is `manifest`.
fn package_name(name: &str, manifest: &Path) -> Result<String, String> {
    let mut metadata = cargo();
    metadata
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest);
    let output = run(metadata)?;
    let unnamed = || {
        format!(
            "pythrust: binding `{name}`: cargo cannot name its package\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    };
    if !output.status.success() {
        return Err(unnamed());
    }
    let parsed = json::parse(&String::from_utf8_lossy(&output.stdout)).ok_or_else(unnamed)?;
    let packages = parsed
        .get("packages")
        .and_then(Json::as_arr)
        .ok_or_else(unnamed)?;
    for p in packages {
        let at = p.get("manifest_path").and_then(Json::as_str);
        let named = p.get("name").and_then(Json::as_str);
        let (Some(at), Some(named)) = (at, named) else {
            return Err(unnamed());
        };
        if same(Path::new(at), manifest)? {
            return Ok(named.to_string());
        }
    }
    Err(unnamed())
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
pub(crate) fn utf8(path: &Path) -> Result<String, String> {
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
pub(crate) fn write(dir: &Path, name: &str, text: &str) -> Result<(), String> {
    let tmp = dir.join(format!(".{name}.tmp"));
    let path = dir.join(name);
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("pythrust: cannot write {}: {e}\n", path.display()))
}

/// Cargo: `$CARGO`, as cargo sets it for what it runs, or `cargo`.
/// Without color, so its output reads plainly when parsed or shown.
pub(crate) fn cargo() -> Command {
    let mut c = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    c.args(["--color", "never"]);
    c
}

fn cargo_rustc(dir: &Path) -> Command {
    let mut c = cargo();
    c.args([
        "rustc",
        "--lib",
        "--locked",
        "--message-format=json-render-diagnostics",
        "--manifest-path",
    ])
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

pub(crate) fn run(mut command: Command) -> Result<Output, String> {
    command.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            format!("pythrust: building Rust bindings needs cargo (Rust {MIN_RUST} or later), which is not installed\n")
        }
        _ => format!("pythrust: cannot run cargo: {e}\n"),
    })
}

/// A failed build, named by what failed: a shim (the stub does not match
/// the Rust function), a binding, or something else, with cargo's output.
fn failure(output: &Output, shims: &str, bindings: &[Binding]) -> String {
    let deps: Vec<gen::Dep> = bindings.iter().map(Binding::dep).collect();
    let log = String::from_utf8_lossy(&output.stderr);
    let head = match failed_crate(&log) {
        Some(name) if name == CRATE => match shim_line(&log).and_then(|l| gen::stub_of(shims, l)) {
            Some(stub) => format!("{stub} does not match the Rust function"),
            None => "pythrust: the generated bindings crate failed to build".to_string(),
        },
        Some(package) if deps.iter().any(|d| d.package == package) => {
            let name = deps
                .iter()
                .find(|d| d.package == package)
                .map_or(package, |d| d.name);
            format!("pythrust: binding `{name}` failed to build")
        }
        _ => match deps.as_slice() {
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
    fn search_paths_come_from_this_builds_messages() {
        let messages = concat!(
            r#"{"reason":"compiler-artifact","filenames":["/x"]}"#,
            "\n",
            r#"{"reason":"build-script-executed","package_id":"a","linked_libs":["dylib=foo"],"linked_paths":["native=/a","dependency=/rust","C:\\b\u0041"],"cfgs":[]}"#,
            "\n",
            r#"{"reason":"build-script-executed","package_id":"b","linked_libs":[],"linked_paths":[]}"#,
            "\n",
        );
        let arg = |p: &str| match cfg!(windows) {
            true => format!("/LIBPATH:{p}"),
            false => format!("-L{p}"),
        };
        assert_eq!(
            search_paths(messages),
            Ok(["/a", "C:\\bA"].map(arg).to_vec())
        );
        let broken = r#"{"reason":"build-script-executed","linked_paths":[oops]}"#;
        assert!(search_paths(broken).is_err());
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
