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
        "[package]\nversion.workspace = true\n[workspace]\n[workspace.package]\nversion = \"1.0.0\"\n",
    ];
    for manifest in refused {
        let got = standalone(manifest, WORKSPACE_TOML);
        assert!(
            got.as_ref()
                .is_err_and(|e| e.contains("make the override's Cargo.toml standalone")),
            "{manifest}: {got:?}"
        );
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
/// quoted table name is the table it names.
#[test]
fn standalone_reads_quote_runs_and_quoted_headers() {
    let manifest = "[\"package\"]\nauthors = [\"\"\"A\"\"\"\"]\nnote = '''B'''''\nversion.workspace = true\n[dependencies.helper]\npath = \"../helper\"\n";
    let got = standalone(manifest, WORKSPACE_TOML);
    assert!(
        got.as_ref().is_err_and(|e| e.contains("path dependency")),
        "{got:?}"
    );
    let manifest = "[ 'package' ]\nauthors = [\"\"\"A\"\"\"\"]\nversion.workspace = true\n";
    let got = standalone(manifest, WORKSPACE_TOML).unwrap();
    assert!(got.contains("\nversion = \"0.3.0\"\n"), "{got}");
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
    ];
    for manifest in refused {
        let got = standalone(manifest, "");
        assert!(
            got.as_ref().is_err_and(|e| e.contains("path dependency")),
            "{manifest}: {got:?}"
        );
    }
    let lib =
        "[lib]\npath = \"src/lib.rs\"\n[package]\nname = \"path\"\n[dependencies]\nxpath = \"1\"\n";
    assert!(standalone(lib, "").is_ok());
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
