# M6b: Rust binary-level interop

**Status: decided 2026-10-31.** Nano approved the scope proposal's five recommendations with conditions, then revised them after the reviews of PR #75 (the EGM review, `pythrust-expert`, Codex and Dottius): the lockfile mechanism, the raise convention, the driver check, and three cuts (`abi::Raise`, the parallel `group_xxh3` with lists and threads, native `run`). Not yet implemented; [roadmap.md](../roadmap.md#phase-5-m6-plan) orders the work.

## Objective

[design.md](../design.md#interop) promises that a Pythrust program can call Rust crates at the binary level. M6b delivers the smallest real form of that: a program calls functions of a Rust crate the app owns, with types checked on both sides and no change to the compiler's dependencies or its `forbid(unsafe_code)`. fclone-lite is the app that forces the work: natively it is bound by hashing speed (see the roadmap's M6a hashing entry), and the Rust crate ecosystem has faster hashes.

## Decided

1. **Binding crate plus stub.** The app owns a *binding crate*: ordinary Rust with no Pythrust dependency. Its *stub* is a `.pyr` file of `temper def` signatures whose bodies are `...`. Outside a stub a `...` body is a named error ("stub bodies only; use pass"), a deliberate omission listed in the deviations, to revisit when Protocol or trait support lands. A stub parameter written `mut` or `owned` is a named error ("not supported in stubs yet"), since a change Rust makes would be lost.
2. **`run` refuses for now.** The interpreter cannot call Rust: loading a library at run time needs `unsafe` in the compiler. `run` of a program that imports a binding is the named gap "run cannot call Rust bindings yet; use build". Revisit when an app needs `run` for interop programs.
3. **Naming a binding: `--rust name=path`** on `check` and `build` (and on `run`, which then refuses). Interim: M6d's manifest supersedes it.
4. **Driver:** fclone-lite hashing with xxh3 through a `fastfs` binding (below).
5. **Toolchain and cache.** cargo and rustc are needed only for programs that import a binding; every other program builds exactly as today. The generated crate and cargo's target directory live in `.pythrust/<stem>/` next to the program's `.pyr` file (one directory per program, so two programs side by side never share a build), ignored by the repo's `.gitignore`, so a rebuild recompiles only what changed.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A builtin xxh3 in `pythrust-rt`, as `hashlib.sha256` is | Fixes the driver, not the design's promise of calling Rust crates; every later crate would need the same |
| A proc-macro export attribute in the binding | Adds `syn` and `quote`, and the binding then depends on Pythrust |
| Hand-written `extern "C"` functions in the binding | Puts `unsafe` in every app, unaudited |
| Calling arbitrary crate functions directly | Results such as `u128` do not fit `int`; the binding is where an app adapts a crate |
| One staticlib per binding | Each would carry its own copy of std; two in one executable clash |
| Loading a library at run time | Needs `unsafe` in the compiler, or a C loader in the runtime, for `run` only |

## How a call works

```text
apps/fclone-lite/fastfs/Cargo.toml     # depends on xxhash-rust; declares an empty [workspace]
apps/fclone-lite/fastfs/Cargo.lock     # committed; required
apps/fclone-lite/fastfs/src/lib.rs     # pub fn file_xxh3(path: &str) -> std::io::Result<String>
apps/fclone-lite/fastfs/fastfs.pyr     # the stub, below
```

```python
temper def file_xxh3(path: str) -> str raises OSError:
    ...
```

The program writes `import fastfs` and calls `fastfs.file_xxh3(p)`. Imports keep M6a's rules: first in the module, no `from`, a module is not a value.

### Resolution

- `--rust name=path` comes before the file and may repeat. `path` is the binding's directory, and the stub is `path/name.pyr`. `build` makes `path` absolute and writes it into the generated manifest as a TOML basic string with `\` and `"` escaped, so a relative path, a Windows path, a space or an apostrophe works.
- `name` is the binding crate's library name, a Rust identifier, and the shim calls `name::function`: stub functions live at the crate root.
- A name that is a standard module (`sys`, `os`, `hashlib`), a builtin (`len`, `print`), an exception class or a reserved word (`pause`, `shared`), or that repeats, is an error. So is a missing directory or stub, and an `import` of a name no `--rust` gives.

### Front end

The driver passes the parsed stubs to the parser (`parser::parse` gains the bindings), which resolves `fastfs.file_xxh3` to one dotted name as it does `os.listdir`. A stub function is a function with a signature and no body: `infer` checks calls against it and skips the return check; `own` gives it a fixed summary (every parameter `read`, a fresh result), as it does module functions today; `rc` treats its result as fresh. The lexer gains a `...` token, and a stub body may sit on the `def` line or the next. Stub diagnostics are rendered against the stub's own file, so spans carry which source they belong to. The `raises Exception` lint skips stubs, whose bodies say nothing.

### Build

1. `check` reads the stubs and checks calls against them. No cargo.
2. `build` writes a generated crate into `.pythrust/`: one `extern "C"` shim per stub function, which converts the arguments, calls the binding, and converts the result. It depends on every binding and on `pythrust-rt` by path, and cargo builds it as the one staticlib the program links in place of the plain runtime.
3. **Reproducible dependencies.** The bindings' committed `Cargo.lock` files decide every crates.io version. `cargo build --locked` cannot be used: the generated crate is the root, so its lockfile must also list the generated package and `pythrust-rt`, which a binding's lock lacks, and cargo refuses to add them under `--locked` (checked with cargo 1.99.1 on a scratch binding). Instead `build`:
    1. checks each binding's lock is current with `cargo metadata --locked` in the binding's directory, which fails when the manifest asks for anything the lock does not pin (checked: a manifest moved to `itoa = "1.0"` over a lock pinning `0.4.8` fails), so the union below holds only each binding's real resolution, and a version another binding happens to pin cannot satisfy a stale one; a lock entry with a `git` source is a named error in this cut ("binding `fastfs`: git dependencies are not supported yet; use a crates.io version"), since the registry-only seed and check below would not hold its commit;
    2. seeds `.pythrust/<stem>/Cargo.lock` with the union of the bindings' registry entries; two locks pinning one crate at different but semver-compatible versions (same major for `1.0` and up, same minor for `0.x` with `x > 0`, the same exact version for `0.0.x`, so `0.0.1` and `0.0.2` coexist) is the named drift error below, since cargo would unify them and one pin would change; semver-incompatible versions (`itoa 0.4.8` and `1.0.1`) stay side by side, as cargo keeps them;
    3. runs `cargo build` without `--locked`; cargo keeps every version the seed pins and adds only the path packages (also checked: a seeded `itoa 1.0.1` stayed `1.0.1`);
    4. checks one direction: every registry entry (name, version, checksum) of the resulting lock must appear in the union with the same version and checksum, and every dependency edge it shares with a binding's lock (same package, same dependency name) must point at the same version, and an added or changed entry fails with a named error ("binding `fastfs`: dependencies differ from its Cargo.lock; update and commit the binding's lockfile"). Entries the build did not need, such as a binding's dev-dependencies or optional dependencies behind features left off, drop out of the generated lock and are fine. Drift never counts as a successful build. `pythrust-rt` has no dependencies, so any registry entry comes from a binding.

    A binding without a `Cargo.lock` is a named error ("binding `fastfs` has no Cargo.lock; run `cargo generate-lockfile` in it and commit the file"). Regression tests (plumbing PR): a clean first build from an ordinary binding lock, not only a warm rebuild; a binding with a registry dev-dependency builds clean; a lock pinning an older version than the manifest allows keeps it; a dependency missing from the lock fails with the named error; a binding whose manifest moved to `itoa = "1.0"` while its lock still pins `0.4.8` fails even beside a second binding pinning `1.0.1`; two bindings with different registry dependencies build together; two bindings pinning `0.4.x` and `1.0.x` of one crate build; two pinning `1.0.1` and `1.0.9` fail with the named error; two pinning `0.0.1` and `0.0.2` build; a binding with a git dependency fails with its named error.
4. rustc compiles the shims against the real signatures, so a stub saying `bytes` where Rust takes `&str` fails the build. Each shim carries a comment naming its stub line, so Pythrust reports "fastfs/fastfs.pyr:1: the stub for `file_xxh3` does not match the Rust function" (the comment's text, which names the stub file as given) and then rustc's output, as Rust's output, not as a Pythrust diagnostic. An error outside the shims (in the binding, or fetching a crate) is reported as "binding `fastfs` failed to build" with cargo's output.
5. The link reuses `link_in` with the generated staticlib in place of `runtime_lib()`, and with the native libraries cargo reports for it (`--print native-static-libs`) in place of the fixed `NATIVE_LIBS` list, since a binding's dependencies may need more.

### Plumbing

| Item | Choice |
| --- | --- |
| Runtime as a dependency | `pythrust-rt` gains `rlib` beside `staticlib` |
| Runtime source | a path fixed when `pythrust` is compiled (its `CARGO_MANIFEST_DIR`), overridable by `PYTHRUST_RT_SRC`; an interop build ignores `PYTHRUST_RT` |
| Generated manifest | edition 2021, an empty `[workspace]`, crate `pythrust_bindings` |
| Profile | the profile `pythrust` itself was built with, so the debug leak check stays on in tests; `panic = "unwind"` always |
| Target | `--target` set to the host triple `pythrust` targets, so a `windows-gnu` default cannot meet MSVC's `link.exe` |
| Directories | `--target-dir .pythrust/<stem>/target`; generated sources written to a temporary file and renamed. The object file and final link still use the private build directory (`private_dir`) |
| Minimum toolchain | Rust 1.88, since every interop build compiles `pythrust-rt`, which uses `<[T]>::as_chunks` (1.88, in M6a's SHA-256; this note first said 1.87 for `usize::is_multiple_of`, and the plumbing PR's clippy MSRV check found the newer call); `#[unsafe(no_mangle)]` needs only 1.82. The plumbing PR records it as `pythrust-rt`'s `rust-version` and a CI job builds the probe with 1.88. This is a demonstrated lower bound, not a full MSRV audit. A missing `cargo` or an older rustc is a named error |

### The shim

Shims follow the M6a functions that raise (`pt_os_path_getsize` in `os.rs`): scalars and pointers in, the call site's rendered diagnostic as `head`/`tail` pointers, and on an error the runtime's pending exception (`exc.rs`) is set and a placeholder returned; after a stub that declares `raises OSError` or `raises Exception` (the stubs that use `call_raising` below), lowering tests the pending flag (`check_raised`), as for any call that may raise, and after any other stub it does not. There is no per-call status return, and the C ABI is unchanged. The whole template for `file_xxh3`:

```rust
// stub fastfs/fastfs.pyr:1: the stub for `file_xxh3`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_rs_6fastfs_9file_xxh3(
    path: *const pythrust_rt::abi::Str,
    head: *const u8, head_len: usize, tail: *const u8, tail_len: usize,
) -> *mut pythrust_rt::abi::Str {
    // SAFETY: codegen passes a live string and a static site.
    let (path, at) = unsafe { (pythrust_rt::abi::arg_str(&path), pythrust_rt::abi::site(head, head_len, tail, tail_len)) };
    pythrust_rt::abi::call_raising(at, || fastfs::file_xxh3(path))
}
```

The symbol length-prefixes each part, so no two binding and function names collide. `abi` is a new public module of `pythrust-rt` holding everything else, and part of its audited C ABI surface:

- `arg_*` conversions in (a `bool` arrives as a `u8`) and `site` take raw pointers, so they are `unsafe fn`s with a `# Safety` section, like `str::borrow` and `os::site` today. A borrowing conversion takes a reference to the shim's own argument, `arg_str<'a>(p: &'a *const Str) -> &'a str`, so the borrow is tied to the shim's frame: a binding taking `&'static str` fails rustc ("borrowed value does not live long enough") instead of keeping a string the runtime may free.
- The `IntoValue` trait out, implemented for each result type in the table, and two safe calls. Each flushes the runtime's buffered stdout, runs the function under `catch_unwind`, and converts the result or the panic. `call` takes a function returning `T: IntoValue`; `call_raising` takes one returning `Result<T, std::io::Error>` and raises on `Err`. The generator picks `call_raising` exactly when the stub declares `raises OSError` (or `Exception`).

So the stub must match the Rust signature, and rustc rejects a mismatch: `-> str` over a function returning `i64` (no `IntoValue` for `i64` as `str`), no `raises` over a function returning `Result` (`Result` is not `IntoValue`), and `raises OSError` over one that cannot fail (`call_raising` needs a `Result`). Regression tests (conversions PR): each of the three mismatches, and a `&'static str` or `&'static [u8]` parameter, fails the build with the stub-mismatch error.

## Unsafe

The compiler stays `forbid(unsafe_code)`. The new `unsafe` is the generated shims, all from the one fixed template above, and the `unsafe fn`s of `pythrust_rt::abi` they call (`arg_*`, `site`), which join `pythrust-rt`'s audited C ABI surface; `call`, `call_raising` and `IntoValue` are safe. A test generates the shims for the probe binding's stub and checks each holds exactly one `#[unsafe(no_mangle)]`, one `unsafe extern "C" fn` and one `unsafe {}` block, and nothing else marked `unsafe`; `abi.rs` is audited by hand with the rest of the runtime's C surface. The template's names avoid the words the vocabulary test (`tests/vocabulary.rs`) bans. [CLAUDE.md](../../CLAUDE.md)'s `unsafe` exception names the generated shims. The binding's own dependencies may use `unsafe`; trusting them is the same as trusting any crate the app depends on.

## Types across the boundary

Every parameter is `read`: Rust borrows for the call, and a borrow cannot outlive it.

| Pythrust | Rust parameter | Rust result | Notes |
| --- | --- | --- | --- |
| `int` | `i64` | `i64` | |
| `float` | `f64` | `f64` | |
| `bool` | `bool` | `bool` | Crosses the C ABI as a `u8` |
| `None` | not allowed | `()` | |
| `str` | `&str` | `String` | No copy in; a fresh string out |
| `bytes` | `&[u8]` | `Vec<u8>` | No copy in |

### Threads

Pythrust values never reach Rust threads; Rust threads see plain Rust data and call-scoped borrows (Nano, 2026-10-31). A binding may hand a `&str` or `&[u8]` argument to scoped threads (`std::thread::scope`) joined before it returns: the borrow checker already keeps them from outliving the call, and the runtime buffer cannot change during it. Runtime handles and refcount operations happen only on the calling thread, so refcounts stay single-threaded until M6c (concurrency). A panic in a thread the binding detaches and never joins is not caught. The conversions PR tests this now (Nano, 2026-10-31), independent of `group_xxh3`: a probe function hands its `&str` to a scoped thread that reads it and returns its length.

### Errors

A function returning `Result<T, std::io::Error>` raises on `Err`, and its stub must say `raises OSError` (or `Exception`); rustc enforces the match through `call` and `call_raising`, so there is no runtime check. M6a's mapping applies: the four known subclasses map (`FileNotFoundError`, `PermissionError`, `IsADirectoryError`, `NotADirectoryError`), and any other error is a plain `OSError`, a listed deviation (CPython raises e.g. `FileExistsError`). An `io::Error` carries no file name, so `str(e)` lacks CPython's `: 'path'`; one without an errno raises `OSError` with its message alone. Other error types are not accepted.

### Panics

`call` catches a panic, which would otherwise abort across `extern "C"`, and stops the program as a fault does: it flushes stdout, prints Pythrust's fault diagnostic at the call site with the panic's message (a non-string payload reads "Rust panic"), and exits 1. A panic hook keeps Rust's own "thread panicked" line from also printing, but only while `call` runs on the calling thread (a thread-local flag); any other panic, such as one in a thread a binding detaches, falls through to the default hook and prints as usual. The conversions PR tests both: a panic inside `call` prints only Pythrust's fault diagnostic, and a detached thread's panic still prints its message. A panic in a scoped thread prints twice by design: the default hook's line from the worker (the flag is per thread), then Pythrust's fault diagnostic when the scope re-raises it on the calling thread. A panic is never catchable, like overflow.

### Ownership

Every result is a new value owned by the caller, which is how `own` and `rc` (the ownership and refcount analyses) already treat a module function's result. Tuples stay out of the ABI, as Nano required for the boxed layout ([native.md](../native.md#choices-made-inside-the-settled-design)).

## Oracle

CPython cannot import a Rust binding, so an interop program has no CPython oracle. Its tests compare it with a pure-Pythrust program doing the same work; for the driver, `fclone_lite.py`. Interop is a Pythrust extension, listed as such in [grammar.md](../grammar.md#deviations-from-python).

## Driver: fast fclone-lite

`apps/fclone-lite/fclone_fast.pyr` keeps fclone-lite's walk and grouping by size, and hashes each file with `fastfs.file_xxh3(path: str) -> str raises OSError`: xxh3-128 of the file streamed in Rust, as 32 lowercase hex digits. Partial hashing, which rusty_fclone also uses, is an algorithm change pure Pythrust can make once a sized `read` exists, so it is out of scope; the rusty_fclone row is not like-for-like.

Done means: the same duplicate groups as `fclone_lite.py` on the M6a trees, compared as sets of groups, since `fclone_lite.py` prints a size's groups in SHA-256 digest order and xxh3 orders them differently; fixtures with two distinct duplicated payloads of one size, and with an unreadable file (made unreadable by a method that holds for root, and skipped visibly where none exists, as on Windows); benchmark rows for native fclone-lite, `fclone_fast`, CPython and rusty_fclone; and xxhash-rust justified in the driver PR as the app's dependency. No speedup is claimed before the benchmark runs.

**Follow-up only if measured:** if `fclone_fast` is not already I/O-bound, a later PR adds `group_xxh3(paths: list[str]) -> list[str]`, hashing a size group in parallel with rayon, with the `list[str]` row.

## Not in this cut

Each waits for an app that needs it: named-builtin raises (`abi::Raise`); lists, `dict`, `set` and tuples crossing; other integer widths and `&'static str` results; native `run`; Rust objects held by Pythrust (a hasher with `update`); callbacks from Rust into Pythrust; `mut` parameters; generic functions; naming crates.io crates directly in `import`; and the manifest (M6d).

## Risks

| Risk | Mitigation |
| --- | --- |
| `build` runs the binding's build scripts and fetches its crates | The same trust as `cargo build`; `--rust` names each binding explicitly, and the lockfile check pins versions |
| First build time (runtime plus binding with cargo, probably tens of seconds) | Incremental after that; measured in the plumbing PR, which fails if a first CI build exceeds 120 s |
| Runtime source missing for an installed `pythrust` | `PYTHRUST_RT_SRC` names it; shipping it is M6d's |
| A binding's dependencies need system libraries | Taken from cargo's `native-static-libs` |
| Binding output interleaving with Pythrust's buffered stdout | `call` flushes before every shim call; a binding calling `std::process::exit` loses unflushed output, a documented limit |
| Windows linking | Target pinned to MSVC; the existing Windows job builds the probe |
