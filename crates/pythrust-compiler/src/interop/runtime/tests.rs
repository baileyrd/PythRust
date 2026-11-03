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
