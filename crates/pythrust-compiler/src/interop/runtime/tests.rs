use super::*;

const WORKSPACE_TOML: &str = "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nversion = \"0.3.0\"\nedition = \"2021\"\nlicense = \"MIT\"\n\n[other]\nversion = \"9\"\n";

#[test]
fn standalone_writes_out_what_the_runtime_inherits() {
    let manifest = "[package]\nname = \"rt\"\nversion.workspace = true\nedition.workspace = true\nlicense.workspace = true\nrust-version = \"1.97\"\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert_eq!(
        got,
        Ok("[package]\nname = \"rt\"\nversion = \"0.3.0\"\nedition = \"2021\"\nlicense = \"MIT\"\nrust-version = \"1.97\"\n\n[workspace]\n".to_string())
    );
}

#[test]
fn standalone_keeps_a_manifest_that_is_its_own_workspace() {
    let manifest = "[package]\nname = \"rt\"\nversion = \"1.0.0\"\n\n[workspace]\n";
    assert_eq!(
        standalone(manifest, WORKSPACE_TOML),
        Ok(manifest.to_string())
    );
}

#[test]
fn standalone_names_a_field_the_workspace_does_not_set() {
    let manifest = "[package]\nrepository.workspace = true\n";
    assert_eq!(
        standalone(manifest, WORKSPACE_TOML),
        Err("the workspace's [workspace.package] sets no `repository`".to_string())
    );
}

/// Keys under `package.metadata`, in any spelling, are left to tools:
/// a `workspace` key there is not inheritance.
#[test]
fn standalone_leaves_package_metadata_alone() {
    let manifest = "[package]\nmetadata = { tool = { workspace = true } }\nmetadata.other.workspace = true\n\"metadata\".x = { workspace = true }\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert_eq!(got, Ok(format!("{manifest}\n[workspace]\n")));
    let paths = [
        "[package.metadata.tool.dependencies]\npath = \"some/tool/path\"\n",
        "[package.metadata.tool]\ndependencies = { x = { path = \"p\" } }\n",
        "[package]\nmetadata.dependencies.path = \"p\"\n",
        "package.metadata.dev-dependencies.path = \"p\"\n",
    ];
    for manifest in paths {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert_eq!(got, Ok(format!("{manifest}\n[workspace]\n")), "{manifest}");
    }
}

/// A `\\u`/`\\U` escape in a basic-string key reads as cargo decodes it,
/// and an escape that makes a quote leaves the string's bounds alone.
#[test]
fn standalone_reads_escapes_in_basic_strings() {
    let manifest =
        "[package]\ndescription = \"a\\u0022 \\\" b\"\n\"\\u0076ersion\".workspace = true\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert!(got.as_ref().is_err_and(|e| e.contains(PLAIN)), "{got:?}");
    let plain = "[package]\ndescription = \"a\\u0022 \\\" b\"\nversion.workspace = true\n";
    let want = "[package]\ndescription = \"a\\u0022 \\\" b\"\nversion = \"0.3.0\"\n\n[workspace]\n";
    assert_eq!(standalone(plain, WORKSPACE_TOML), Ok(want.to_string()));
}

/// The supported inheritance line keeps working through a trailing
/// comment, and a `#` inside a string is not a comment.
#[test]
fn standalone_reads_comments_and_strings() {
    let manifest = "[package]\nversion.workspace = true # shared\nname = \"a#b\" # c\ndescription = \"q\\\"#\" # d\nreadme = 'a#b' # e\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert_eq!(
        got,
        Ok("[package]\nversion = \"0.3.0\"\nname = \"a#b\" # c\ndescription = \"q\\\"#\" # d\nreadme = 'a#b' # e\n\n[workspace]\n".to_string())
    );
}

/// Only `key.workspace = true` in `[package]`, from a workspace above,
/// is rewritten (D39); any other inheritance is a named error asking for
/// a standalone manifest, not a guess.
#[test]
fn standalone_refuses_other_inheritance_forms() {
    let refused = [
        "[package]\nedition = { workspace = true }\n",
        "[package]\n\"version\".workspace = true\n",
        "[package]\nversion.\"workspace\" = true\n",
        "[package]\nversion = { \"workspace\" = true }\n",
        "[package.version]\nworkspace = true\n",
        "[package]\nversion.workspace = true\n[workspace]\n[workspace.package]\nversion = \"1.0.0\"\n",
        "[package]\nversion.workspace = true\n[workspace.package]\nversion = \"7.7.7\"\n",
        "workspace.resolver = \"2\"\n[package]\nversion.workspace = true\n",
        "[lints]\nworkspace = true\n",
        "lints.workspace = true\n[package]\n",
        "[dependencies]\nlibc = { workspace = true }\n",
        "[dependencies]\nlibc.workspace = true\n",
        "[dependencies.libc]\nworkspace = true\n",
        "[target.'cfg(unix)'.dev-dependencies]\nlibc = { version = \"1\", workspace = true }\n",
        "package.name = \"rt\"\npackage.version.workspace = true\npackage.edition = \"2021\"\n",
        "package = { name = \"rt\", version = { workspace = true } }\n",
        "package = { metadata = { tool = { workspace = true } }, version = { workspace = true } }\n",
        "package = { version = { workspace = true }, metadata = { workspace = true } }\n",
    ];
    for manifest in refused {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert!(
            refused_with(&got, "make the override's Cargo.toml standalone"),
            "{manifest}: {got:?}"
        );
    }
    let features = "[features]\nworkspace = []\n[workspace.dependencies]\nx = \"1\"\n";
    assert_eq!(standalone(features, ""), Ok(features.to_string()));
}

/// Any `workspace` table or root key makes a manifest its own workspace
/// root, as cargo reads it; a bare `[workspace]` header is not needed.
#[test]
fn any_workspace_table_makes_a_root() {
    for root in [
        "[workspace]\n",
        "[workspace.package]\nversion = \"1\"\n",
        "[\"workspace\".lints]\n",
        "workspace.members = []\n[package]\n",
    ] {
        assert_eq!(is_workspace_root(root), Ok(true), "{root}");
    }
    for member in [
        "[package]\nworkspace = \"..\"\n",
        "[package.metadata.workspace]\n",
    ] {
        assert_eq!(is_workspace_root(member), Ok(false), "{member}");
    }
}

/// The manifest is read as statements, not lines: string and array
/// contents are never headers or keys, a multi-line inherited value is
/// copied whole, and only `[package]` fields are inherited.
#[test]
fn standalone_reads_statements_across_lines() {
    let workspace = "[workspace]\n[workspace.package]\nauthors = [\n  \"a\",\n  \"b\", # c\n]\nversion = \"1.0.0\"\n";
    let manifest = "[package]\ndescription = \"\"\"\n[workspace]\nversion.workspace = true\n\"\"\"\nkeywords = [\n  \"[workspace]\",\n]\nauthors.workspace = true\nversion.workspace = true\n\n[package.metadata.tool]\nfoo = { workspace = true }\n";
    let got = standalone(manifest, workspace);
    let want = "[package]\ndescription = \"\"\"\n[workspace]\nversion.workspace = true\n\"\"\"\nkeywords = [\n  \"[workspace]\",\n]\nauthors = [\n  \"a\",\n  \"b\", # c\n]\nversion = \"1.0.0\"\n\n[package.metadata.tool]\nfoo = { workspace = true }\n\n[workspace]\n";
    assert_eq!(got, Ok(want.to_string()));
}

/// A closing run of four or five quotes ends a multi-line string, and a
/// quoted guarded header is refused as not plain (D42).
#[test]
fn standalone_reads_quote_runs_and_quoted_headers() {
    let manifest = "[package]\nauthors = [\"\"\"A\"\"\"\"]\nnote = '''B'''''\nversion.workspace = true\n[dependencies.helper]\npath = \"../helper\"\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert!(
        got.as_ref().is_err_and(|e| e.contains("path dependency")),
        "{got:?}"
    );
    let manifest = "[package]\nauthors = [\"\"\"A\"\"\"\"]\nversion.workspace = true\n";
    let got = standalone(manifest, WORKSPACE_TOML).unwrap();
    assert!(got.contains("\nversion = \"0.3.0\"\n"), "{got}");
    let got = standalone("[ 'package' ]\nversion.workspace = true\n", WORKSPACE_TOML);
    assert!(got.as_ref().is_err_and(|e| e.contains(PLAIN)), "{got:?}");
}

/// A manifest that is its own workspace root is kept as it is, and a
/// header's trailing comment does not hide its `[workspace]`.
#[test]
fn standalone_keeps_its_own_commented_workspace() {
    let manifest = "[package]\nversion = \"1.0.0\"\n[workspace] # standalone\n";
    assert_eq!(
        standalone(manifest, WORKSPACE_TOML),
        Ok(manifest.to_string())
    );
}

/// A path dependency would resolve from the extracted copy, not from
/// the override, so it is a named error; `[lib] path` is the crate's own.
#[test]
fn standalone_refuses_a_path_dependency() {
    let refused = [
        "[dependencies]\nhelper = { path = \"../helper\" }\n",
        "[dev-dependencies.helper]\npath = \"../helper\"\n",
        "[target.'cfg(unix)'.build-dependencies]\nh = { version = \"1\", path = \"h\" }\n",
        "[dependencies] # local helpers\nhelper = { path = \"../helper\" }\n",
        "[dependencies]\nhelper = { \"path\" = \"../helper\" }\n",
        "[dependencies]\nhelper = {'path'=\"../helper\"}\n",
        "[dependencies]\nhelper.path = \"../helper\"\n",
        "[dependencies.helper]\n\"path\" = \"../helper\"\n",
        "dependencies.helper.path = \"../helper\"\n[package]\n",
        "dependencies = { helper = { path = \"../helper\" } }\n",
        "[dependencies]\nlibc = { x = \"\"\"a\"b\"\"\", path = \"../libc\" }\n",
        "[dependencies]\nlibc = { x = '''a'b''', path = \"../libc\" }\n",
        "[patch.crates-io]\nlibc = { path = \"../libc\" }\n",
        "[patch.crates-io.libc]\npath = \"../libc\"\n",
        "[replace]\nfoo = { path = \"f\" }\n",
        "target = { 'cfg(unix)' = { dependencies = { helper = { path = \"../helper\" } } } }\n",
        "target.'cfg(unix)' = { build-dependencies = { h = { path = \"h\" } } }\n",
        "target.'cfg(target_os = \"linux\")' = { dependencies = { helper = { path = \"../helper\" } } }\n",
        "[target.\"cfg(a = \\\"b\\\")\".dependencies]\nh = { path = \"h\" }\n",
        "[dependencies]\nhelper = { \"\\u0070ath\" = \"../helper\" }\n",
        "[dependencies.helper]\n\"\\U00000070ath\" = \"h\"\n",
        "[\"dep\\u0065ndencies\".helper]\npath = \"h\"\n",
    ];
    for manifest in refused {
        let got = standalone(manifest, "");
        assert!(refused_with(&got, "path dependency"), "{manifest}: {got:?}");
    }
    let lib =
        "[lib]\npath = \"src/lib.rs\"\n[package]\nname = \"path\"\n[dependencies]\nxpath = \"1\"\n";
    assert!(standalone(lib, "").is_ok());
    let literal = "[dependencies.helper]\n'\\u0070ath' = \"h\"\n";
    let got = standalone(literal, "");
    assert!(refused_with(&got, PLAIN), "{got:?}");
}

#[test]
fn the_hash_covers_every_path_and_byte() {
    let file = |path: &str, text: &str| File {
        path: path.to_string(),
        text: text.as_bytes().to_vec(),
    };
    let base = [file("a", "xy"), file("b", "z")];
    let h = hash(&base);
    assert_eq!(h.len(), 16);
    assert_eq!(h, hash(&base.clone()));
    for other in [
        vec![file("a", "xy"), file("b", "y")],
        vec![file("a", "x"), file("b", "yz")],
        vec![file("a", "xy"), file("c", "z")],
        vec![file("a", "xy")],
    ] {
        assert_ne!(hash(&other), h, "{other:?}");
    }
    assert_eq!(digest(b""), "cbf29ce484222325");
}

/// Every file of `crates/pythrust-rt` but its build output is embedded,
/// so a new runtime file cannot be left out of the shipped source.
#[test]
fn every_runtime_file_is_embedded() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../pythrust-rt");
    let mut on_disk = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path.strip_prefix(&root).unwrap();
            let rel: Vec<_> = rel
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            on_disk.push(rel.join("/"));
        }
    }
    on_disk.sort();
    let embedded: Vec<&str> = EMBEDDED.iter().map(|(p, _)| *p).collect();
    assert_eq!(embedded, on_disk);
    for (path, text) in EMBEDDED {
        let disk = std::fs::read_to_string(root.join(path)).unwrap();
        assert_eq!(*text, disk, "{path}");
    }
}

/// The embedded set is the runtime with its `Cargo.toml` standalone.
#[test]
fn embedded_files_are_the_runtime_made_standalone() {
    let files = embedded().unwrap();
    assert_eq!(files.len(), EMBEDDED.len());
    let manifest = std::str::from_utf8(&files[0].text).unwrap();
    assert_eq!(files[0].path, "Cargo.toml");
    assert!(!manifest.contains(".workspace = true"), "{manifest}");
    assert!(manifest.contains(&format!("\nversion = \"{}\"\n", env!("CARGO_PKG_VERSION"))));
    assert!(manifest.ends_with("\n[workspace]\n"), "{manifest}");
}

const PLAIN: &str = "plainly written standalone manifest";
const QUOTED: &str = "without dots or escapes";

/// Whether `got` is a refusal: the `specific` error, or a fail-closed one.
fn refused_with(got: &Result<String, String>, specific: &str) -> bool {
    got.as_ref()
        .is_err_and(|e| [specific, PLAIN, QUOTED].iter().any(|m| e.contains(m)))
}

/// A quoted key or header part holding a `.` or an escape is refused
/// wherever it is, in the override or in its workspace manifest, since
/// cargo may read it differently from the checks (D42).
#[test]
fn quoted_parts_with_dots_or_escapes_are_refused() {
    let overrides = [
        "[target.'cfg(unix)'.\"\\x64ependencies\"]\npb = { path = \"../pb\" }\n",
        "[\"\\x77orkspace\".package]\nversion = \"1\"\n[package]\nversion.workspace = true\n",
        "[features]\n\"a.b\" = []\n",
    ];
    for manifest in overrides {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert!(
            got.as_ref().is_err_and(|e| e.contains(QUOTED)),
            "{manifest}: {got:?}"
        );
    }
    let parent = "[workspace]\n[\"workspace.package\"]\nversion = \"6.6.6\"\n";
    let got = standalone("[package]\nversion.workspace = true\n", parent);
    assert!(got.as_ref().is_err_and(|e| e.contains(QUOTED)), "{got:?}");
    let escaped = is_workspace_root("[\"\\x77orkspace\"]\n");
    assert!(
        escaped.as_ref().is_err_and(|e| e.contains(QUOTED)),
        "{escaped:?}"
    );
}

/// Fail closed (D42): in `[package]`, `[workspace.package]` and
/// dependency tables, a spelling the check does not plainly read is the
/// named error, whatever it would mean to cargo.
#[test]
fn standalone_refuses_spellings_it_does_not_plainly_read() {
    let refused = [
        "[package]\n\"name\" = \"rt\"\n",
        "[package]\n'name' = \"rt\"\n",
        "[ 'package' ]\nname = \"rt\"\n",
        "[\"dep\\u0065ndencies\".helper]\nversion = \"1\"\n",
        "[dependencies]\nhelper.version = \"1\"\n",
        "[dependencies]\nhelper = { \"version\" = \"1\" }\n",
        "[dependencies]\nhelper = { version = \"1\", nested = { a = 1 } }\n",
        "[package]\nmetadata2 = { a = 1 }\n",
        "[package]\nversion.workspace = false\n",
        "[workspace.package]\n\"version\" = \"1\"\n",
        "package.name = \"rt\"\n",
        "package = { name = \"rt\" }\n",
        "package = { name = \"rt\", metadata = { tool = { workspace = true } } }\n",
        "dependencies.helper = \"1\"\n",
        "target = { 'cfg(unix)' = { dependencies = { h = \"1\" } } }\n",
        "[target.'cfg(unix)']\ndependencies = { h = \"1\" }\n",
        "[target.'cfg(unix)'.\"dependencies\"]\nh = \"1\"\n",
        "[dependencies.target.'x']\nversion = \"1\"\n",
        "[package]\n\"metadata.x\" = 1\n",
        "[dependencies]\nlibc = { version = \"0.2\", features = [\"std\"] }\n",
        "[dependencies]\nlibc = {}\n",
        "[target.'cfg(unix)'.dependencies]\nlibc = { version = \"0.2\" }\n",
    ];
    for manifest in refused {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert!(refused_with(&got, PLAIN), "{manifest}: {got:?}");
    }
}

/// The plain spellings still pass: bare keys, one header per table, a
/// plain-string or `[dependencies.<name>]` dependency, a quoted target
/// cfg, and anything outside the guarded tables.
#[test]
fn standalone_accepts_plain_spellings() {
    let plain = [
        "[package]\nname = \"rt\"\nkeywords = [\"a\", \"b\"]\n",
        "[dependencies]\nlog = \"0.4\"\n",
        "[dependencies.libc]\nversion = \"0.2\"\nfeatures = [\"std\"]\n",
        "[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n",
        "[target.'cfg(target_os = \"linux\")'.dev-dependencies]\nlibc = \"0.2\"\n",
        "[package.metadata.\"tool\"]\n\"x\".y = { z = { a = 1 } }\n",
        "[lints.rust]\nunexpected_cfgs = { level = \"warn\", check-cfg = [\"cfg(x)\"] }\n",
        "[features]\n\"a-b\" = []\n",
    ];
    for manifest in plain {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert!(got.is_ok(), "{manifest}: {got:?}");
    }
}
