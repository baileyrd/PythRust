use super::{analyze, Analysis, Mode, Param, Precision};
use crate::diag::DiagKind;
use crate::{infer, lexer, parser};

fn analyzed(src: &str) -> Analysis {
    let module = match lexer::lex(src).and_then(|t| parser::parse(&t)) {
        Ok(m) => m,
        Err(d) => panic!("parse failed: {d}"),
    };
    match infer::infer(&module) {
        Ok(types) => analyze(&module, &types),
        Err(d) => panic!("check failed: {d}"),
    }
}

/// `name`'s parameters in a program that must analyze precisely.
fn params(src: &str, name: &str) -> Vec<Param> {
    let a = analyzed(src);
    assert_eq!(a.precision, Precision::Precise, "{src}");
    match a.funcs.get(name) {
        Some(s) => s.params.clone(),
        None => panic!("no summary for `{name}`"),
    }
}

/// `name`'s modes, and which parameters its result may alias.
fn modes(src: &str, name: &str) -> (Vec<Mode>, Vec<bool>) {
    let ps = params(src, name);
    (
        ps.iter().map(|p| p.mode).collect(),
        ps.iter().map(|p| p.result_aliases).collect(),
    )
}

const READ: Mode = Mode::Read;
const MUT: Mode = Mode::Mut;
const OWNED: Mode = Mode::Owned;

#[test]
fn reading_a_list_is_read() {
    let src = "def total(xs):\n    s = 0\n    for x in xs:\n        s += x\n    return s + len(xs) + xs[0]\nprint(total([1, 2]))\n";
    assert_eq!(modes(src, "total"), (vec![READ], vec![false]));
}

#[test]
fn append_and_item_assignment_are_mut() {
    let src = "def grow(xs):\n    xs.append(1)\ndef set(xs):\n    xs[0] = 2\nys = [0]\ngrow(ys)\nset(ys)\nprint(ys)\n";
    assert_eq!(modes(src, "set"), (vec![MUT], vec![false]));
}

#[test]
fn changing_a_list_inside_is_mut() {
    let src = "def bump(rows: list[list[int]]):\n    row = rows[0]\n    row.append(1)\ndef pair(t: tuple[list[int], int]):\n    t[0].append(t[1])\nbump([[0]])\npair(([0], 1))\n";
    assert_eq!(modes(src, "bump"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "pair"), (vec![MUT], vec![false]));
}

#[test]
fn returning_or_storing_is_owned() {
    let src = "def same(xs: list[int]) -> list[int]:\n    return xs\ndef wrap(xs: list[int]) -> list[list[int]]:\n    return [xs]\ndef put(dst: list[list[int]], x: list[int]):\n    dst.append(x)\nkept: list[list[int]] = []\ndef keep(xs: list[int]):\n    kept.append(xs)\ndef shelf() -> list[list[int]]:\n    return kept\ndef keep_late(xs: list[int]):\n    shelf().append(xs)\nput(wrap(same([1])), [2])\nkeep([3])\nkeep_late([4])\n";
    // Returning is the result's alias, which the caller accounts for.
    assert_eq!(modes(src, "same"), (vec![READ], vec![true]));
    assert_eq!(modes(src, "wrap"), (vec![READ], vec![true]));
    assert_eq!(modes(src, "put"), (vec![MUT, OWNED], vec![false, false]));
    assert_eq!(modes(src, "keep"), (vec![OWNED], vec![false]));
    assert_eq!(modes(src, "keep_late"), (vec![OWNED], vec![false]));
    let a = analyzed(src);
    assert!(a.funcs["shelf"].result_global);
    assert!(!a.funcs["same"].result_global);
}

#[test]
fn a_copy_shares_only_elements() {
    // The D3 fix-it: `list(xs)` of flat ints aliases nothing.
    let flat = "def copy(xs: list[int]) -> list[int]:\n    ys = list(xs)\n    ys.append(1)\n    return ys\nprint(copy([0]))\n";
    assert_eq!(modes(flat, "copy"), (vec![READ], vec![false]));
    // A copy of a list of lists still holds the inner lists.
    let nested = "def copy(xs: list[list[int]]) -> list[list[int]]:\n    ys = list(xs)\n    ys.append([1])\n    return ys\ndef poke(xs: list[list[int]]):\n    ys = list(xs)\n    ys[0].append(1)\nprint(copy([[0]]))\npoke([[0]])\n";
    assert_eq!(modes(nested, "copy"), (vec![READ], vec![true]));
    assert_eq!(modes(nested, "poke"), (vec![MUT], vec![false]));
}

#[test]
fn a_local_container_is_not_the_parameter() {
    let src = "def count(xs: list[int]) -> int:\n    box = [xs]\n    box.append([1])\n    return len(box)\ndef through(xs: list[int]):\n    box = [xs]\n    box[0].append(1)\nprint(count([0]))\nthrough([0])\n";
    assert_eq!(modes(src, "count"), (vec![READ], vec![false]));
    assert_eq!(modes(src, "through"), (vec![MUT], vec![false]));
}

#[test]
fn values_that_hold_no_list_are_read() {
    let src = "def f(s: str, n: int, t: tuple[int, str]) -> str:\n    return s + str(n) + t[1]\nprint(f(\"a\", 1, (2, \"b\")))\n";
    assert_eq!(modes(src, "f"), (vec![READ; 3], vec![false; 3]));
}

#[test]
fn modes_follow_calls_and_returned_aliases() {
    let src = "def grow(xs):\n    xs.append(1)\ndef twice(ys):\n    grow(ys)\n    grow(ys)\ndef ident(xs):\n    return xs\ndef via(ys):\n    z = ident(ys)\n    z.append(2)\ndef look(ys):\n    return len(ident(ys))\nl = [0]\ntwice(l)\nvia(l)\nprint(look(l))\n";
    assert_eq!(modes(src, "twice"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "via"), (vec![MUT], vec![false]));
    // `ident` gives back its argument, which `look` only reads.
    assert_eq!(modes(src, "look"), (vec![READ], vec![false]));
}

#[test]
fn a_callee_may_store_one_argument_in_another() {
    let src = "def put(dst: list[list[int]], x: list[int]):\n    dst.append(x)\ndef gather(xs: list[int]) -> list[list[int]]:\n    out: list[list[int]] = []\n    put(out, xs)\n    return out\nprint(gather([1]))\n";
    // `put` may also have kept `xs` anywhere the module can see.
    assert_eq!(modes(src, "gather"), (vec![OWNED], vec![true]));
}

#[test]
fn mutual_recursion_reaches_a_fixpoint() {
    let src = "def a(xs, n):\n    if n > 0:\n        b(xs, n - 1)\ndef b(xs, n):\n    xs.append(n)\n    a(xs, n)\na([0], 2)\n";
    assert_eq!(modes(src, "a"), (vec![MUT, READ], vec![false, false]));
    assert_eq!(modes(src, "b"), (vec![MUT, READ], vec![false, false]));
    let rec = "def last(xs, n):\n    if n == 0:\n        return xs\n    return last(xs, n - 1)\nprint(last([1], 3))\n";
    assert_eq!(modes(rec, "last"), (vec![READ; 2], vec![true, false]));
}

#[test]
fn generic_instances_are_joined() {
    let src = "def same(x):\n    return x\nprint(same(1))\nprint(same([2]))\n";
    assert_eq!(modes(src, "same"), (vec![READ], vec![true]));
}

#[test]
fn the_module_and_uncalled_functions_have_no_summary() {
    let a = analyzed("def unused(xs):\n    xs.append(1)\nprint(1)\n");
    assert_eq!(a.precision, Precision::Precise);
    assert!(a.funcs.is_empty(), "{:?}", a.funcs);
}

#[test]
fn an_operation_on_a_never_returning_generic_result_is_precise() {
    let src = "def f(n):\n    return f(n - 1) + 1\ndef g(xs):\n    xs.append(1)\n    if len(xs) > 9:\n        print(f(1))\ng([0])\n";
    assert_eq!(modes(src, "g"), (vec![MUT], vec![false]));
}

/// A program lowering cannot express still analyzes, conservatively.
fn conservative(src: &str) {
    let a = analyzed(src);
    let Precision::Conservative(gap) = &a.precision else {
        panic!("analyzed precisely, expected a lowering gap: {src}");
    };
    assert_eq!(gap.kind, DiagKind::Backend, "{gap}");
    assert!(gap.message.contains("does not support"), "{gap}");
    let owned = Param {
        mode: OWNED,
        result_aliases: true,
        holds_global: true,
    };
    for (name, s) in &a.funcs {
        assert!(s.params.iter().all(|p| *p == owned), "{name}: {s:?}");
    }
}

#[test]
fn raising_and_catching_lower_precisely() {
    let src = "def f(xs, n):\n    if n < 0:\n        raise ValueError(\"neg\")\n    xs.append(n)\n    return n\ntry:\n    print(f([1], -1))\nexcept ValueError as e:\n    print(e)\n";
    assert_eq!(modes(src, "f"), (vec![MUT, READ], vec![false, false]));
}

#[test]
fn an_exception_holding_a_list_falls_back() {
    let src = "def f(xs, n):\n    raise ValueError(xs)\nf([1], 2)\n";
    conservative(src);
    assert_eq!(analyzed(src).funcs["f"].params.len(), 2);
}

#[test]
fn a_builtin_called_before_its_shadowing_def_falls_back() {
    conservative("x = print(7)\ndef print(n):\n    return 1\n");
}

#[test]
fn modes_see_through_optionals_loops_and_callees() {
    let src = "def opt(xs: list[int] | None):\n    if xs is not None:\n        xs.append(1)\ndef rows(rs: list[list[int]]):\n    for r in rs:\n        r.append(1)\ndef poke(b: list[list[int]]):\n    b[0].append(1)\ndef boxed(xs: list[int]):\n    poke([xs])\nopt([0])\nrows([[0]])\nboxed([0])\n";
    assert_eq!(modes(src, "opt"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "rows"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "boxed"), (vec![MUT], vec![false]));
}

#[test]
fn a_rebound_parameter_is_summarized_over_every_binding() {
    // Flow-insensitive: sound, if coarser than the body needs.
    let src = "def f(xs: list[int]):\n    xs = [1]\n    xs.append(2)\nf([0])\n";
    assert_eq!(modes(src, "f"), (vec![MUT], vec![false]));
}

#[test]
fn a_fresh_result_may_be_nested_to_any_depth() {
    let src = "def make() -> list[list[list[int]]]:\n    return [[]]\ndef f(xs: list[int]) -> list[list[list[int]]]:\n    box = make()\n    box[0].append(xs)\n    return box\ndef g(ys: list[int]):\n    b = f(ys)\n    b[0][0].append(1)\nprint(f([1]))\ng([2])\n";
    // Sound but coarse: `box[0]` stands for every list in `box`, and
    // `xs` is one, so appending to it may change `xs`.
    assert_eq!(modes(src, "f"), (vec![MUT], vec![true]));
    assert_eq!(modes(src, "g"), (vec![MUT], vec![false]));
}

#[test]
fn a_changed_argument_may_come_to_hold_a_global() {
    let src = "saved: list[list[int]] = []\ndef stash(dst: list[list[list[int]]]):\n    dst.append(saved)\ndef f(xs: list[int]):\n    box: list[list[list[int]]] = []\n    stash(box)\n    box[0].append(xs)\nf([1])\n";
    assert_eq!(modes(src, "stash"), (vec![MUT], vec![false]));
    assert!(params(src, "stash")[0].holds_global);
    assert_eq!(modes(src, "f"), (vec![OWNED], vec![false]));
}

#[test]
fn a_changed_argument_may_come_to_hold_a_fresh_list() {
    let src = "def fill(dst: list[list[list[int]]]):\n    dst.append([])\ndef f(xs: list[int]) -> list[list[list[int]]]:\n    box: list[list[list[int]]] = []\n    fill(box)\n    box[0].append(xs)\n    return box\nprint(f([1]))\n";
    assert_eq!(modes(src, "fill"), (vec![MUT], vec![false]));
    assert!(!params(src, "fill")[0].holds_global);
    // Coarse as above: `box[0]` may be `xs`.
    assert_eq!(modes(src, "f"), (vec![MUT], vec![true]));
}

#[test]
fn a_result_stored_in_a_parameter_or_global_aliases_it() {
    let src = "kept: list[list[int]] = []\ndef make(out: list[list[int]]) -> list[int]:\n    child: list[int] = []\n    out.append(child)\n    return child\ndef leak() -> list[int]:\n    child: list[int] = []\n    kept.append(child)\n    return child\ndef g(xs: list[list[int]]):\n    c = make(xs)\n    c.append(1)\ndef h(ys: list[int]):\n    leak().append(1)\n    c = leak()\n    c.append(len(ys))\nprint(make([[0]]))\ng([[0]])\nh([0])\n";
    assert_eq!(modes(src, "make"), (vec![MUT], vec![true]));
    assert!(!analyzed(src).funcs["make"].result_global);
    assert!(analyzed(src).funcs["leak"].result_global);
    assert_eq!(modes(src, "g"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "h"), (vec![READ], vec![false]));
}

#[test]
fn storing_a_list_of_a_parameter_elsewhere_keeps_it() {
    // `child` joins `xs`, then `ys` keeps it.
    let src = "def f(xs: list[list[int]], ys: list[list[int]]):\n    child: list[int] = []\n    xs.append(child)\n    ys.append(child)\nf([[0]], [[1]])\n";
    assert_eq!(modes(src, "f"), (vec![OWNED, OWNED], vec![false, false]));
}

#[test]
fn recursion_building_nested_results_aliases_its_parameter() {
    let src = "def deep(xs: list[int], n: int) -> list[list[int]]:\n    if n == 0:\n        return [xs]\n    r = deep(xs, n - 1)\n    r.append([n])\n    return r\nprint(deep([1], 2))\n";
    // Coarse: a result that may hold `xs` may also be it, so
    // `r.append` may change `xs`.
    assert_eq!(modes(src, "deep"), (vec![MUT, READ], vec![true, false]));
}
