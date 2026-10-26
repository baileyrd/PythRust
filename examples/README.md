# Conformance corpus

Each `<name>.pyr` program has a golden `<name>.out`: the exact stdout it must print. The phase-2 subset is Python-compatible, so CPython is the oracle. The harness is `crates/pythrust-compiler/tests/conformance.rs`; it compares stdout and exit status, never stderr.

```sh
cargo test --test conformance                 # goldens agree with CPython and `pythrust run`
PYTHRUST_BLESS=1 cargo test --test conformance corpus_matches_cpython   # regenerate goldens
```

`PYTHON` picks the oracle (default `python3`, baseline 3.12). `PYTHRUST_BIN` picks the binary (default: the one Cargo built for the test run).

Directives go in the leading comment block, which Python ignores:

- `# expect-exit: N`: expected exit status (default 0). A runtime error exits 1, as an uncaught CPython exception does.
- `# deviation: why`: a deviation listed in `docs/grammar.md`. The golden is written by hand and holds Pythrust's output. Blessing skips it, and the harness fails if CPython stops differing.

- `# expect-error: text`: the program must fail `check` (lex, parse, infer) with a diagnostic containing `text`, and it needs `# expect-exit: 1`. Every program without it must pass `check`. `corpus_checks` asserts both in process, so diagnostics are covered while `run` is still unwired.

Blessing creates missing goldens, but it refuses to overwrite one when CPython's exit status differs from `expect-exit`. Each program is killed after 10 seconds, so a hang fails the test instead of stalling CI.

Adding a program: write `<name>.pyr` in the phase-2 subset, bless, read the `.out` before committing. Keep programs well-typed (homogeneous lists, one type per variable) so `pythrust check` can run over the corpus too.
