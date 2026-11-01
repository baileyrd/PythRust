use super::*;
use crate::{lexer, parser};

fn stubs(name: &str, src: &str) -> Vec<StubDef> {
    let tokens = lexer::lex(src).unwrap_or_else(|d| panic!("{d:?}"));
    let binding = parser::parse_stub(&tokens, name).unwrap_or_else(|d| panic!("{d:?}"));
    binding.funcs
}

fn line(span: Span) -> String {
    format!("probe.pyr@{}", span.start)
}

#[test]
fn symbols_prefix_each_part_with_its_length() {
    assert_eq!(symbol("fastfs.file_xxh3"), "pt_rs_6fastfs_9file_xxh3");
    assert_ne!(symbol("ab.c"), symbol("a.bc"));
}

#[test]
fn shims_call_the_binding_with_scalars() {
    let src = "temper def double(x: int) -> int: ...\n\
               temper def mix(a: float, b: int) -> float: ...\n";
    let lib = shims(&stubs("probe", src), &line).unwrap_or_else(|d| panic!("{d:?}"));
    assert!(lib.contains("extern crate pythrust_rt;\n"), "{lib}");
    let double = "// stub probe.pyr@0: the stub for `double`\n\
                  #[unsafe(no_mangle)]\n\
                  pub extern \"C\" fn pt_rs_5probe_6double(a0: i64) -> i64 {\n    \
                  pythrust_rt::abi::call(|| r#probe::r#double(a0))\n}\n";
    assert!(lib.contains(double), "{lib}");
    assert!(
        lib.contains(
            "fn pt_rs_5probe_3mix(a0: f64, a1: i64) -> f64 {\n    pythrust_rt::abi::call(|| r#probe::r#mix(a0, a1))\n}"
        ),
        "{lib}"
    );
}

#[test]
fn every_shim_holds_exactly_one_unsafe_attribute() {
    let src = "temper def double(x: int) -> int: ...\ntemper def half(x: float) -> float: ...\n";
    let lib = shims(&stubs("probe", src), &line).unwrap_or_default();
    assert_eq!(lib.matches("unsafe").count(), 2, "{lib}");
    assert_eq!(lib.matches("#[unsafe(no_mangle)]").count(), 2, "{lib}");
}

#[test]
fn other_types_and_raises_are_named_gaps() {
    let cases = [
        ("temper def f(x: str) -> int: ...\n", "pass `str`", "str"),
        (
            "temper def f(x: int) -> bytes: ...\n",
            "pass `bytes`",
            "bytes",
        ),
        ("temper def f() -> None: ...\n", "pass `None`", "None"),
        (
            "temper def f(x: int) -> int raises OSError: ...\n",
            "call a Rust function that raises yet",
            "OSError",
        ),
    ];
    for (src, what, at) in cases {
        let err = shims(&stubs("probe", src), &line).err();
        let Some(d) = err else {
            panic!("{src}");
        };
        assert_eq!(d.kind, DiagKind::Backend);
        assert!(
            d.message.starts_with(&format!("build cannot {what}")),
            "{}",
            d.message
        );
        assert_eq!(&src[d.span.start..d.span.end], at, "{src}");
    }
}

#[test]
fn rust_path_words_cannot_name_a_function() {
    let err = shims(&stubs("probe", "temper def self() -> int: ...\n"), &line).err();
    let message = err.map(|d| d.message).unwrap_or_default();
    assert_eq!(
        message,
        "a Rust function named `self` cannot be called; rename it"
    );
    assert!(reserved_crate("crate") && reserved_crate("std") && reserved_crate(CRATE));
    assert!(!reserved_crate("fastfs"));
}

#[test]
fn stub_of_finds_the_shim_a_line_is_in() {
    let src = "temper def double(x: int) -> int: ...\ntemper def half(x: float) -> float: ...\n";
    let lib = shims(&stubs("probe", src), &line).unwrap_or_default();
    let line_of = |needle: &str| {
        lib.lines()
            .position(|l| l.contains(needle))
            .map_or(0, |i| i + 1)
    };
    assert_eq!(stub_of(&lib, line_of("extern crate")), None);
    let first = stub_of(&lib, line_of("r#double("));
    assert_eq!(first, Some("probe.pyr@0: the stub for `double`"));
    let second = stub_of(&lib, line_of("r#half("));
    assert!(second.is_some_and(|s| s.ends_with("`half`")), "{second:?}");
}

#[test]
fn manifest_quotes_and_escapes_paths() {
    let dep = Dep {
        name: "fastfs",
        package: "fastfs-binding",
        dir: "C:\\b \"q\"\n",
    };
    let toml = manifest("/rt", &[dep]);
    assert!(
        toml.contains("pythrust-rt = { path = \"/rt\" }\n"),
        "{toml}"
    );
    assert!(
        toml.contains(
            "fastfs = { package = \"fastfs-binding\", path = \"C:\\\\b \\\"q\\\"\\u000A\" }\n"
        ),
        "{toml}"
    );
    for part in [
        "name = \"pythrust_bindings\"",
        "crate-type = [\"staticlib\"]",
        "panic = \"unwind\"",
        "\n[workspace]\n",
    ] {
        assert!(toml.contains(part), "{part}: {toml}");
    }
}

#[test]
fn a_file_name_cannot_break_out_of_the_comment() {
    let evil = |_: Span| "x.pyr\n#[unsafe(no_mangle)] fn evil() {}\r//".to_string();
    let lib = shims(&stubs("probe", "temper def f() -> int: ...\n"), &evil).unwrap_or_default();
    assert!(
        lib.contains("// stub x.pyr\\n#[unsafe(no_mangle)] fn evil() {}\\r//: the stub for `f`\n"),
        "{lib}"
    );
    assert_eq!(lib.matches("#[unsafe(no_mangle)]\n").count(), 1, "{lib}");
}
