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

/// The probe binding's stub, which the end-to-end tests build.
const PROBE: &str = include_str!("../../../tests/rust/probe/probe.pyr");

#[test]
fn a_shim_follows_the_fixed_template() {
    let src = "temper def size(path: str, n: int) -> str raises OSError: ...\n";
    let lib = shims(&stubs("probe", src), &line).unwrap_or_else(|d| panic!("{d:?}"));
    assert!(lib.contains("extern crate pythrust_rt;\n"), "{lib}");
    let shim = "// stub probe.pyr@0: the stub for `size`\n\
                #[unsafe(no_mangle)]\n\
                pub unsafe extern \"C\" fn pt_rs_5probe_4size(\n    \
                p0: *const pythrust_rt::abi::Str,\n    \
                p1: i64,\n    \
                head: *const u8, head_len: usize, tail: *const u8, tail_len: usize,\n\
                ) -> *mut pythrust_rt::abi::Str {\n    \
                // SAFETY: generated code passes live values and a static site.\n    \
                let (a0, a1, at) = unsafe { (pythrust_rt::abi::arg_str(&p0), p1, \
                pythrust_rt::abi::site(head, head_len, tail, tail_len)) };\n    \
                pythrust_rt::abi::call_raising(at, || -> std::io::Result<String> \
                { r#probe::r#size(a0, a1) })\n}\n";
    assert!(lib.contains(shim), "{lib}");
}

#[test]
fn each_type_converts_as_the_table_says() {
    let cases = [
        ("temper def f() -> int: ...\n", "-> i64 {", "|| -> i64 {"),
        (
            "temper def f(x: float) -> float: ...\n",
            "p0: f64,",
            "|| -> f64 {",
        ),
        (
            "temper def f(x: bool) -> bool: ...\n",
            "arg_bool(p0)",
            "|| -> bool {",
        ),
        (
            "temper def f(x: int) -> None: ...\n",
            ") -> u8 {",
            "|| -> () {",
        ),
        (
            "temper def f(x: bytes) -> bytes: ...\n",
            "arg_bytes(&p0)",
            "|| -> Vec<u8> {",
        ),
        (
            "temper def f() -> bytes raises Exception: ...\n",
            "-> *mut pythrust_rt::abi::Bytes {",
            "call_raising(at, || -> std::io::Result<Vec<u8>> {",
        ),
    ];
    for (src, a, b) in cases {
        let lib = shims(&stubs("probe", src), &line).unwrap_or_else(|d| panic!("{d:?}"));
        assert!(lib.contains(a) && lib.contains(b), "{src}: {lib}");
    }
    let lib = shims(&stubs("probe", "temper def f() -> int: ...\n"), &line).unwrap_or_default();
    assert!(
        lib.contains("let at = unsafe { pythrust_rt::abi::site("),
        "{lib}"
    );
}

/// The audit (`docs/research/rust-interop.md#unsafe`): every shim of the
/// probe's stub holds exactly one `#[unsafe(no_mangle)]`, one `unsafe
/// extern "C" fn` and one `unsafe {}` block, and nothing else `unsafe`;
/// the header forbids unsafe operations outside that block.
#[test]
fn every_shim_holds_exactly_the_expected_unsafe_items() {
    let lib = shims(&stubs("probe", PROBE), &line).unwrap_or_else(|d| panic!("{d:?}"));
    let mut parts = lib.split(STUB_MARK);
    let header = parts.next().unwrap_or_default();
    assert!(header.starts_with("//! Generated"), "{header}");
    assert!(header.contains(&format!("\n{LINT}")), "{header}");
    let lint = LINT.matches("unsafe").count();
    assert_eq!(header.matches("unsafe").count(), lint, "{header}");
    let shims: Vec<&str> = parts.collect();
    assert_eq!(shims.len(), PROBE.lines().count(), "{lib}");
    for shim in shims {
        assert_eq!(shim.matches("#[unsafe(no_mangle)]\n").count(), 1, "{shim}");
        assert_eq!(
            shim.matches("pub unsafe extern \"C\" fn ").count(),
            1,
            "{shim}"
        );
        assert_eq!(shim.matches(" = unsafe { ").count(), 1, "{shim}");
        assert_eq!(shim.matches("unsafe").count(), 3, "{shim}");
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
    let lib = shims(&stubs("probe", PROBE), &line).unwrap_or_default();
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
