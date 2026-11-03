# M6d: the package tool, first cut

**Status: decided 2026-11-03.** Nano accepted the scope's six recommendations, with conditions: [scope](https://claude.ai/artifact/T8zQ1ew154ifDNbmmT7NSD). Code follows in the PR order below.

## Objective

[design.md](../design.md#tooling-and-library) promises one tool, one manifest and one lockfile for all three dependency kinds: Pythrust packages, Rust crates and Python packages. Two of the three cannot exist yet. A program is one file, so there are no Pythrust packages, and Python packages wait for M6e. This cut makes the Rust kind real: the manifest replaces M6b's interim `--rust name=path` ([rust-interop.md](rust-interop.md)), and one lockfile pins every crate a program's bindings use. The other kinds get their names in the format now and arrive with their features.

## Decided

1. **Rust bindings only.** A `[python]` or `[pythrust]` table is the named error "`[python]` is reserved: Python packages arrive with M6e" (with "Pythrust packages arrive with multi-file programs" for `[pythrust]`). It is an error even when the table is empty.
2. **Manifest `pythrust.toml`, in a strict subset of TOML** ([grammar](../grammar.md#manifest-pythrusttoml)), read by a parser in the compiler, with no new dependency.
   - An oracle test runs every case through CPython 3.13's `tomllib`. A form the subset accepts must parse to the same values. Every other form gets a named error, whether `tomllib` accepts it or not.
   - Duplicate and unknown keys are errors.
3. **One `pythrust.lock`** holds the full resolved set of the generated crate: `pythrust-rt`, the bindings, and every crate they pull in.
   - `build` copies it in as the generated crate's `Cargo.lock` and runs cargo with `--locked`. M6b's seed-and-verify goes ([below](#why-plain---locked-now-works)).
   - `pythrust lock` writes it. A missing lock is the named error "no pythrust.lock; run `pythrust lock`". A stale one gets "pythrust.lock is out of date; run `pythrust lock`", followed by cargo's reason.
   - A binding no longer needs its own committed `Cargo.lock`.
4. **`--rust` is removed** in the same PR as the manifest, and the interop tests and fclone-lite move to manifests. The public CLI change is approved (Nano, 2026-11-03).
5. **Discovery.** From the source file's directory, walk up to the first `pythrust.toml` or the filesystem root.
   - The manifest is read only when the program imports a binding, so every other program builds exactly as today, with or without a manifest above it.
   - An import of a non-standard module with no manifest found is the named error "`fastfs` is not a standard module, and no pythrust.toml names it", followed by the directories searched.
   - A manifest that is found is parsed whole, so its errors show even if the program uses one binding.
6. **The runtime source ships inside `pythrust`.** Each file of `crates/pythrust-rt` is embedded with `include_str!`.
   - A test compares the embedded file list with the runtime directory, so a new runtime file cannot be left out.
   - On first use, the files are written to `.pythrust/rt/<version>-<hash>/`, where the hash covers the embedded sources. A build whose directory is missing or does not match writes it afresh. No cache is shared between projects in this cut.
   - `PYTHRUST_RT_SRC` stays as a development override.
   - The PR reports first-build time.

## The manifest

```toml
[package]
name = "fclone-lite"
version = "0.1.0"

[rust]
fastfs = { path = "fastfs" }
```

- **`[package]`** is required. `name` is a string of ASCII letters, digits, `-` and `_`, starting with a letter. `version` is a string. Neither is used yet beyond being required, which keeps `publish` open.
- **`[rust]`** maps a binding's name to an inline table `{ path = "..." }`.
  - The path is relative to the manifest's directory.
  - The name and the stub follow M6b's rules ([rust-interop.md](rust-interop.md#resolution)): the stub is `path/name.pyr`, and a reserved name or a missing directory or stub is an error.
  - Diagnostics point into `pythrust.toml` with line and column.
- **The generated crate depends on every binding in `[rust]`**, not only those a program imports. That way one lock fits every program in the project. Shims are still generated only for the stubs a program calls.
- **Build directories** move to the manifest's directory: `.pythrust/<stem>/` for each program and `.pythrust/rt/` for the runtime.

## Why plain `--locked` now works

M6b could not use `--locked`, because each binding's lock lacked the generated package and `pythrust-rt` ([rust-interop.md](rust-interop.md#build), step 3). `pythrust.lock` is the generated crate's own lock, so it lists them.

Checked on 2026-11-03 (cargo 1.99.1), with fclone-lite's generated crate:

- Its lock, copied to a different directory with the paths in `Cargo.toml` rewritten, built with `--locked` and stayed byte-identical. A path dependency's entry in a lock records no path.
- Adding a dependency (`itoa = "1"`) made `--locked` refuse with "cannot update the lock file".

`pythrust lock` works as follows:

1. It writes the generated `Cargo.toml` for all of `[rust]`.
2. It copies in the current `pythrust.lock`, if one exists.
3. It runs `cargo metadata` without `--locked`. Cargo keeps every pin that still fits and resolves only what is new, as `cargo build` does.
4. It copies the result back.

`cargo update` is not wrapped in this cut; deleting `pythrust.lock` and running `pythrust lock` re-resolves everything.

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
3. **Lockfile:** `pythrust lock`, plain `--locked`, and the missing and stale errors. M6b's seed and verify are removed, and so are the bindings' `Cargo.lock` files. `apps/fclone-lite/pythrust.lock` is committed.
4. **Shipped runtime:** the embedded source, its file-list test, keyed extraction, and a test that builds a binding with `PYTHRUST_RT_SRC` unset, from outside the checkout. The PR reports first-build time.
5. **Driver.** CI copies `apps/fclone-lite` outside the repository and builds `fclone_fast.pyr` from its manifest and lock, with no flags or environment variables, on Linux, macOS and Windows. Its groups must match `fclone_lite.py`'s, as the existing interop test checks.

Exit:

- PR 5's build passes.
- A missing lock, a stale lock and a `[python]` table each fail with their named error.

## Not in this cut

- `new`, `test` and `publish`, and a registry.
- Pythrust packages, which need multi-file programs.
- Python packages (M6e), and the typed-versus-untyped dependency report, which comes with M6e.
- crates.io crates named directly in `import`.
- Git and registry dependencies in `[rust]`. A binding is a local path, and its own `Cargo.toml` names its crates.io dependencies.
- A wrapper for `cargo update`.
- A runtime cache shared between projects.

## Risks

| Risk | Mitigation |
| --- | --- |
| The subset rejects valid TOML someone writes | Each unsupported form has a named error saying what to write instead; the subset grows when a real manifest needs it |
| The generated crate builds every binding, even ones a program does not call | One lock per project needs it; a project's bindings are its own crates, and cargo builds each once per program directory |
| The embedded runtime grows `pythrust` (about 250 KB of source) and an extracted copy goes stale | Extraction is keyed by version and source hash; a mismatch writes a fresh copy |
| `--locked` refuses after a binding's `Cargo.toml` changes | Intended: the stale-lock error names `pythrust lock` |
| Removing `--rust` breaks scripts that use it | None outside the repository are known; the error for an unknown flag names `pythrust.toml` |
