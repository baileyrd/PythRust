# M6d: the package tool, first cut

**Status: decided 2026-11-03.** Nano accepted the scope's six recommendations, with conditions: [scope](https://claude.ai/artifact/T8zQ1ew154ifDNbmmT7NSD). Nano accepted the nine [follow-on choices](#follow-on-choices) the same day. Code follows in the PR order below.

## Objective

[design.md](../design.md#tooling-and-library) promises one tool, one manifest and one lockfile for all three dependency kinds: Pythrust packages, Rust crates and Python packages. Two of the three cannot exist yet. A program is one file, so there are no Pythrust packages, and Python packages wait for M6e. This cut makes the Rust kind real: the manifest replaces M6b's interim `--rust name=path` ([rust-interop.md](rust-interop.md)), and one lockfile pins every crate a program's bindings use. The other kinds get their names in the format now and arrive with their features.

## Decided

1. **Rust bindings only.** A `[python]` or `[pythrust]` table is the named error "`[python]` is reserved: Python packages arrive with M6e" (with "Pythrust packages arrive with multi-file programs" for `[pythrust]`). It is an error even when the table is empty.
2. **Manifest `pythrust.toml`, in a strict subset of TOML** ([grammar](../grammar.md#manifest-pythrusttoml)), read by a parser in the compiler, with no new dependency.
   - An oracle test runs every case through CPython 3.13's `tomllib`. A form the subset accepts must parse to the same values. Every other form gets a named error, whether `tomllib` accepts it or not.
   - Duplicate and unknown keys are errors.
3. **One `pythrust.lock`** holds the full resolved set of the generated crate: `pythrust-rt`, the bindings, and every crate they pull in.
   - `build` copies it in as the generated crate's `Cargo.lock` and runs cargo with `--locked`. M6b's seed-and-verify goes ([below](#why-plain---locked-now-works)).
   - `pythrust lock` writes it. A missing lock is the named error "no pythrust.lock; run `pythrust lock`". A stale one gets "pythrust.lock is out of date; run `pythrust lock`", followed by the packages that differ ([below](#why-plain---locked-now-works)).
   - A binding no longer needs its own committed `Cargo.lock` ([choice 2](#follow-on-choices)): `build` ignores it, and nothing deletes it.
4. **`--rust` is removed** in the same PR as the manifest, and the interop tests and fclone-lite move to manifests. The public CLI change is approved (Nano, 2026-11-03).
5. **Discovery.** From the source file's directory, walk up to the first `pythrust.toml` or the filesystem root.
   - The manifest is read only when the program imports a binding, so every other program builds exactly as today, with or without a manifest above it.
   - An import of a non-standard module with no manifest found is the named error "`fastfs` is not a standard module, and no pythrust.toml names it", followed by the directories searched.
   - A manifest that is found is parsed whole, so its errors show even if the program uses one binding.
6. **The runtime source ships inside `pythrust`.** Each file of `crates/pythrust-rt` is embedded with `include_str!`.
   - A test compares the embedded file list with the runtime directory, so a new runtime file cannot be left out.
   - On first use, the files are written to `.pythrust/rt/<version>-<hash>/`, where the hash covers the embedded sources. A build whose directory is missing or does not match writes it afresh. No cache is shared between projects in this cut.
   - The embedded set carries a standalone `Cargo.toml` for the runtime, with the workspace's inherited `version`, `edition`, `license` and `repository` written out and an empty `[workspace]`, since a copy outside the repository has no workspace to inherit from. The file-list test treats that `Cargo.toml` as the embedded counterpart of the repository's, and a test builds the extracted copy.
   - `PYTHRUST_RT_SRC` stays as a development override ([choice 5](#follow-on-choices)). When it is set, `build` prints one stderr line naming the path, and the extracted copy is keyed by that source's hash as for the embedded one.
   - The PR reports first-build time.

## The manifest

```toml
[package]
name = "fclone-lite"
version = "0.1.0"

[rust]
fastfs = { path = "fastfs" }
```

- **`[package]`** is required ([choice 3](#follow-on-choices)). `name` is a string of ASCII letters, digits, `-` and `_`, starting with a letter. `version` is a string. Neither is used yet beyond being required, which keeps `publish` open.
- **`[rust]`** maps a binding's name to an inline table `{ path = "..." }`.
  - The path is relative to the manifest's directory.
  - The name and the stub follow M6b's rules ([rust-interop.md](rust-interop.md#resolution)): the stub is `path/name.pyr`, and a reserved name or a missing directory or stub is an error.
  - The path is non-empty and relative; `..` is allowed, an absolute path is an error ([choice 9](#follow-on-choices)).
  - Diagnostics point into `pythrust.toml` with line and column.
- **The generated crate depends only on the bindings a program imports** ([choice 1](#follow-on-choices)). `pythrust lock` resolves a crate that names every binding in `[rust]`, and a build that names a subset still passes `--locked` with that lock, unchanged (checked on cargo 1.94.1 and 1.99.1). So one lock fits every program, and a program never compiles or links a binding it does not import. Cargo does not document that it accepts unused lock entries, so PR 3 tests it on every CI OS and with Rust 1.88, the supported minimum, in CI's oldest-toolchain job.
- **Each binding stays its own workspace root**, as in M6b ([choice 7](#follow-on-choices)).
- **Build directories** move to `.pythrust/` in the manifest's directory ([choice 4](#follow-on-choices)). Today they sit next to the output file, as [rust-interop.md](rust-interop.md) now says.
  - `.pythrust/build/<stem>-<hash>/` per program, where the hash covers the program's path relative to the manifest, so `a/main.pyr` and `b/main.pyr` never share a directory, and no program name can collide with the runtime's.
  - `.pythrust/rt/<version>-<hash>/` for the runtime.
  - `.pythrust/lock/` for `pythrust lock`'s generated crate.
- **Concurrency.** Two file locks (std's `File::lock`, no new dependency) keep the shared paths consistent:
  - *The project lock*, `.pythrust/project.lock`. A build holds it shared for its whole run, so nothing it reads (the runtime directory, `pythrust.lock`) changes under it. `pythrust lock`, and a build that must extract or repair the runtime, hold it exclusively.
  - *The build lock*, in each program's build directory, held exclusively for the whole build (generated files, cargo and the link), so two builds of the same program run one after the other.
- **Extraction.** A runtime directory is used only when every file matches the embedded source byte for byte, which also catches a half-written or edited copy. Otherwise the build drops its shared project lock, takes it exclusively, checks again (another build may have extracted it meanwhile), writes the files to a temporary directory, removes any mismatched copy and renames the new one into place, then goes back to a shared lock and checks once more. No build is running while it does this, since builds hold the lock shared.

## Why plain `--locked` now works

M6b could not use `--locked`, because each binding's lock lacked the generated package and `pythrust-rt` ([rust-interop.md](rust-interop.md#build), step 3). `pythrust.lock` is the generated crate's own lock, so it lists them.

Checked on 2026-11-03 (cargo 1.99.1), with fclone-lite's generated crate:

- Its lock, copied to a different directory with the paths in `Cargo.toml` rewritten, built with `--locked` and stayed byte-identical. A path dependency's entry in a lock records no path.
- Adding a dependency (`itoa = "1"`) made `--locked` refuse with "cannot update the lock file".
- A crate naming only some of the locked bindings built with `--locked` and left the lock byte-identical (the pythrust-expert review, cargo 1.94.1 and 1.99.1).
- One lock fits every OS: a lock made on Linux lists a binding's Windows-only dependencies too.

`pythrust lock` works as follows:

1. It writes the generated `Cargo.toml` for all of `[rust]`.
2. It clears `.pythrust/lock/`'s `Cargo.lock` and copies in the current `pythrust.lock`, if one exists. With no `pythrust.lock`, resolution starts from nothing, so deleting the lock re-resolves everything; a test covers an update after an earlier lock.
3. It runs `cargo metadata` without `--locked`. Cargo keeps every pin that still fits and resolves only what is new, as `cargo build` does.
4. It writes the result to a temporary file beside `pythrust.lock` and renames it into place. If any step fails, the previous `pythrust.lock` stays as it was.

`pythrust lock` holds the project lock exclusively from step 1 to step 4, so two runs cannot share the scratch crate and no build reads a half-written `pythrust.lock`.

`cargo update` is not wrapped in this cut ([choice 6](#follow-on-choices)). **To update dependencies**, delete `pythrust.lock` and run `pythrust lock`, which re-resolves everything; the user docs say so. A targeted update flag waits for a real need.

`pythrust lock` sets `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback` and the generated crate declares the runtime's `rust-version`, so a fresh resolution prefers crates that build on the minimum Rust ([choice 8](#follow-on-choices)). The version is one named constant in the compiler, and a test checks it equals `pythrust-rt`'s `rust-version` and CI's oldest-toolchain job (1.88 today).

`pythrust lock` strips its header line before copying the lock in and writes it once on the way out, since cargo adds its own header and rewrites line endings.

**A stale lock** is detected without reading cargo's wording, which changes between versions. When a `--locked` build fails, `build` runs `pythrust lock`'s resolution on a copy, over the crate that names every binding (not the program's own crate, which would drop the others). If resolution succeeds and any whole entry differs from the lock's (name, version, source, checksum or dependencies, as `interop::lock::Package` parses them), it reports "pythrust.lock is out of date; run `pythrust lock`" with the entries that differ, in place of cargo's hint to remove `--locked`. Comparing whole entries catches a binding that adds a dependency on a crate already in the lock, which changes only that binding's `dependencies`. If resolution fails, as offline with an empty cargo cache, or the entries match, `build` shows cargo's own error. The check runs only after a failed `--locked` build, and its resolution may use the network.

`cargo.rs` names a binding's package with `cargo pkgid --locked`, which needs the binding's own lock. It moves to `cargo metadata --no-deps`, which needs none and writes nothing; the package is the one whose `manifest_path` is the binding's `Cargo.toml`.

The format is the generated crate's `Cargo.lock`, verbatim, under one header line, `# Generated by pythrust lock; do not edit.` When M6e adds Python packages, the file gains a section for them, and that format change is decided then.

What carries over from M6b's lock tests:

- A git dependency is still refused, with its named error.
- A dev-dependency of a binding stays out of the build.
- Two bindings that pin one crate no longer conflict: there is one resolution. The drift error goes with seed-and-verify.

## PR order

1. **This note.** It also brings the manifest subset into the grammar and the roadmap items.
2. **Manifest:**
   - the subset reader with its `tomllib` oracle test, the schema and its errors, and discovery;
   - `check`, `run` and `build` taking bindings from the manifest;
   - `--rust` removed;
   - `apps/fclone-lite/pythrust.toml`, and the interop tests migrated.

   Until PR 3, `build` keeps M6b's lock mechanism, reading each binding's `Cargo.lock`.
3. **Lockfile:** `pythrust lock`, plain `--locked`, and the missing and stale errors. M6b's seed and verify are removed, and the bindings' `Cargo.lock` files stay in place, ignored ([choice 2](#follow-on-choices)). `apps/fclone-lite/pythrust.lock` is committed. Its tests include: a lock, then the lock deleted and made again, which must not reuse the old pins; two `pythrust lock` runs at once; and `pythrust lock` beside a running build.
4. **Shipped runtime:** the embedded source, its file-list test, keyed extraction, and a test that builds a binding with `PYTHRUST_RT_SRC` unset, from outside the checkout. Its tests include a corrupted runtime directory, repaired, and two builds extracting at once, one of them while another program builds. The PR reports first-build time.
5. **Driver.** CI copies `apps/fclone-lite` outside the repository and builds `fclone_fast.pyr` from its manifest and lock, with no flags or environment variables, on Linux, macOS and Windows. Its groups must match `fclone_lite.py`'s, as the existing interop test checks.

Exit:

- PR 5's build passes.
- A missing lock, a stale lock and a `[python]` table each fail with their named error.

## Follow-on choices

The review of this note found choices the six decisions did not settle. Nano accepted all nine on 2026-11-03, four of them with conditions, which are written into the sections above.

1. **Build only the imported bindings**, with one lock resolved over all of `[rust]`.
2. **Pythrust ignores the bindings' own `Cargo.lock`s**, with no seeding from them. They are not deleted from any repository.
3. **`[package]` is required**, with `name` and `version`.
4. **Build directories sit under `.pythrust/`** in the manifest's directory.
5. **`PYTHRUST_RT_SRC` stays**, announced by one stderr line naming the path, and its extraction is keyed by the source hash.
6. **The lock is the verbatim `Cargo.lock` under one header line.** `cargo update` is not wrapped. To update dependencies, delete `pythrust.lock` and run `pythrust lock`; a targeted update flag waits for a real need.
7. **Each binding stays its own workspace root.**
8. **Resolution prefers crates that build on the minimum Rust**, one named constant equal to the toolchain the project already uses (1.88: `pythrust-rt`'s `rust-version` and CI's oldest-toolchain job). If the two ever differ, ask Nano first.
9. **A `[rust]` path is non-empty and relative**; `..` is allowed and an absolute path is an error.

## Not in this cut

- `new`, `test` and `publish`, and a registry.
- Pythrust packages, which need multi-file programs.
- Python packages (M6e), and the typed-versus-untyped dependency report, which comes with M6e.
- crates.io crates named directly in `import`.
- Git and registry dependencies in `[rust]`. A binding is a local path, and its own `Cargo.toml` names its crates.io dependencies.
- A wrapper for `cargo update`.
- A runtime cache shared between projects.
- Publishing the compiler to crates.io: `include_str!` reaches outside the compiler crate, so the compiler packages only inside this repository until the runtime source is packaged with it.

## Risks

| Risk | Mitigation |
| --- | --- |
| The subset rejects valid TOML someone writes | Each unsupported form has a named error saying what to write instead; the subset grows when a real manifest needs it |
| A new `pythrust` with a changed runtime version or dependencies makes every project's `pythrust.lock` stale | The stale-lock error names `pythrust lock`; the release notes say so |
| The embedded runtime grows `pythrust` (about 250 KB of source) and an extracted copy goes stale | Extraction is keyed by version and source hash; a mismatch writes a fresh copy |
| `--locked` refuses after a binding's `Cargo.toml` changes | Intended: the stale-lock error names `pythrust lock` |
| Removing `--rust` breaks scripts that use it | None outside the repository are known; the error for an unknown flag names `pythrust.toml` |
