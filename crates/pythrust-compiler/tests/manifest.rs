//! The manifest subset against its oracle, CPython 3.13's `tomllib`
//! (`docs/grammar.md#manifest-pythrusttoml`): every form the subset
//! accepts parses to the same values, and every form it refuses gets a
//! named error, whether `tomllib` accepts it or not.
//!
//! Environment: `PYTHON`, the oracle interpreter (default `python3`, or
//! `python` on Windows), which must be CPython 3.13.

use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};

use pythrust_compiler::manifest::toml::{self, Table, Value};

/// Forms the subset accepts.
const ACCEPTED: &[&str] = &[
    "",
    "\n\n",
    "# only a comment\n",
    "[package]\nname = \"fclone-lite\"\nversion = \"0.1.0\"\n",
    "[package]\nname = \"x\"\n\n[rust]\nfastfs = { path = \"fastfs\" }\n",
    "[a]\nk = \"v\"",
    "[a]\r\nk = \"v\"\r\n",
    "  [ a ]  # c\n\tk\t=\t\"v\"\t# c\n",
    "[a]\nk = {}\n",
    "[a]\nk = {   }\n",
    "[a]\nk = {a=\"1\",b=\"2\"}\n",
    "[a]\nk = { a = \"1\" , b = \"2\" }\n",
    "[a-b_C9]\nx-Y_0 = \"v\"\n",
    "[1]\n2 = \"v\"\n",
    "[a]\nk = \"\"\n",
    "[a]\nk = \"q\\\" b\\\\ \\b\\t\\n\\f\\r\"\n",
    "[a]\nk = \"\\u00e9\\U0001F600\\u0000\"\n",
    "[a]\nk = \"é 😀 \u{85} \u{a0}\"\n",
    "[a]\nk = \"# not a comment\"\n",
    "[a]\nk = \"tab\tinside\"\n",
    "# é \u{85} tab\t\n[a]\n",
    "[a]\n[b]\nk = \"v\"\n",
];

/// Forms the subset refuses, valid TOML or not.
const REFUSED: &[&str] = &[
    "k = \"root\"\n",
    "[a]\nk = 'literal'\n",
    "[a]\nk = \"\"\"multi\"\"\"\n",
    "[a]\nk = '''multi'''\n",
    "[a]\n\"quoted\" = \"v\"\n",
    "[a]\n'quoted' = \"v\"\n",
    "[a]\nk.j = \"v\"\n",
    "[a.b]\n",
    "[\"a\"]\n",
    "[[a]]\n",
    "[a]\nk = [\"v\"]\n",
    "[a]\nk = 1\n",
    "[a]\nk = 0x1f\n",
    "[a]\nk = -1.5e3\n",
    "[a]\nk = inf\n",
    "[a]\nk = nan\n",
    "[a]\nk = true\n",
    "[a]\nk = false\n",
    "[a]\nk = 1979-05-27\n",
    "[a]\nk = 1979-05-27T07:32:00Z\n",
    "[a]\nk = { j = { i = \"v\" } }\n",
    "[a]\nk = { j = \"v\", }\n",
    "[a]\nk = { j = \"v\",\n i = \"w\" }\n",
    "[a]\nk = { j = \"v\", j = \"w\" }\n",
    "[a]\nk = \"v\"\nk = \"w\"\n",
    "[a]\n[a]\n",
    "[a]\nk = \"open\n",
    "[a]\nk = \"\\e\"\n",
    "[a]\nk = \"\\x41\"\n",
    "[a]\nk = \"\\uD800\"\n",
    "[a]\nk = \"\\U00110000\"\n",
    "[a]\nk = \"a\u{1}b\"\n",
    "[a]\nk = \"a\u{7f}b\"\n",
    "[a]\n# a\u{7f}\n",
    "[a]\n# a\u{0}\n",
    "[a]\nk = \"v\" junk\n",
    "[a]\nk = \"v\"\r\r\n",
    "[a]\nk = \"v\"\r",
    "[a]\r",
    "[a]\n# c\r",
    "[a]\nk\n",
    "[a\n",
    "[a]\n= \"v\"\n",
    "[a]\nk = \n",
    "[é]\n",
    "[a]\né = \"v\"\n",
    "[a]\nk = v\n",
];

/// The oracle interpreter, as `tests/conformance.rs` finds it.
fn python() -> String {
    let default = if cfg!(windows) { "python" } else { "python3" };
    std::env::var("PYTHON").unwrap_or_else(|_| default.to_string())
}

/// Each case through `tomllib`: its values as canonical JSON, or `None`
/// where `tomllib` refuses it. The cases go as hex words, so any byte
/// survives the pipe.
const ORACLE: &str = r#"
import json, sys, tomllib
assert sys.version_info[:2] == (3, 13), sys.version
for line in sys.stdin.read().split():
    src = bytes.fromhex(line[1:]).decode("utf-8")
    try:
        value = tomllib.loads(src)
    except tomllib.TOMLDecodeError:
        print("-")
        continue
    print(json.dumps(value, sort_keys=True, separators=(",", ":"), default=repr))
"#;

fn oracle(cases: &[&str]) -> Vec<Option<String>> {
    let mut child = Command::new(python())
        .args(["-I", "-c", ORACLE])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", python()));
    let input: String = cases.iter().map(|c| hex(c.as_bytes()) + "\n").collect();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "the oracle failed (it needs CPython 3.13 as $PYTHON):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<Option<String>> = stdout
        .lines()
        .map(|l| (l != "-").then(|| l.to_string()))
        .collect();
    assert_eq!(lines.len(), cases.len(), "{stdout}");
    lines
}

/// `bytes` as hex after an `x`, so an empty case is still a word.
fn hex(bytes: &[u8]) -> String {
    let digits: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("x{digits}")
}

/// `tables` as `json.dumps(..., sort_keys=True, separators=(",", ":"))`
/// writes the dict `tomllib` returns.
fn canonical(tables: &[Table]) -> String {
    let mut top: BTreeMap<String, String> = BTreeMap::new();
    for table in tables {
        let pairs = table.pairs.iter().map(|p| {
            let value = match &p.value {
                Value::Str(s) => json_str(&s.text),
                Value::Inline { pairs, .. } => object(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.name.clone(), json_str(&v.text)))
                        .collect(),
                ),
            };
            (p.key.name.clone(), value)
        });
        top.insert(table.name.name.clone(), object(pairs.collect()));
    }
    object(top)
}

fn object(fields: BTreeMap<String, String>) -> String {
    let items: Vec<String> = fields
        .iter()
        .map(|(k, v)| format!("{}:{v}", json_str(k)))
        .collect();
    format!("{{{}}}", items.join(","))
}

/// A string as Python's `json.dumps` writes it, ASCII only.
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

#[test]
fn accepted_forms_parse_as_tomllib_parses_them() {
    let want = oracle(ACCEPTED);
    for (src, want) in ACCEPTED.iter().zip(want) {
        let got = toml::parse(src.as_bytes()).map(|t| canonical(&t));
        let Ok(got) = got else {
            panic!("{src:?} refused: {got:?}");
        };
        assert_eq!(Some(got), want, "{src:?}");
    }
}

#[test]
fn refused_forms_get_a_named_error() {
    let tomllib = oracle(REFUSED);
    for (src, tomllib) in REFUSED.iter().zip(tomllib) {
        let Err(e) = toml::parse(src.as_bytes()) else {
            panic!("{src:?} accepted; tomllib gives {tomllib:?}");
        };
        assert!(!e.message.is_empty(), "{src:?}");
    }
}

/// The corpus covers both kinds of refusal: valid TOML outside the
/// subset, and text `tomllib` refuses too.
#[test]
fn the_refused_corpus_holds_both_kinds() {
    let tomllib = oracle(REFUSED);
    assert!(tomllib.iter().any(Option::is_some));
    assert!(tomllib.iter().any(Option::is_none));
}
