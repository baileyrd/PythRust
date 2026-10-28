use super::{analyze, check, Analysis, Mode, Param, Precision};
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
fn an_uncalled_concrete_function_is_summarized() {
    // So modes are checked whether or not anything calls it.
    let src = "def unused(xs):\n    xs.append(1)\ntemper def spare(xs: list[int]) -> None:\n    xs[0] = 2\nprint(1)\n";
    assert_eq!(modes(src, "unused"), (vec![MUT], vec![false]));
    assert_eq!(modes(src, "spare"), (vec![MUT], vec![false]));
}

#[test]
fn the_module_and_uncalled_generic_functions_have_no_summary() {
    let a = analyzed("def unused(xs):\n    return xs\nprint(1)\n");
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
fn an_exception_holding_a_list_keeps_it() {
    // It may outlive the call and render the list later.
    let src = "def f(xs, n):\n    raise ValueError(xs)\nf([1], 2)\n";
    assert_eq!(modes(src, "f"), (vec![OWNED, READ], vec![false, false]));
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

/// `check`'s verdict on `src`: `Ok`, or the message and the source text
/// its span covers.
fn checked(src: &str) -> Result<(), (String, String)> {
    let module = match lexer::lex(src).and_then(|t| parser::parse(&t)) {
        Ok(m) => m,
        Err(d) => panic!("parse failed: {d}"),
    };
    let types = match infer::infer(&module) {
        Ok(t) => t,
        Err(d) => panic!("check failed: {d}"),
    };
    check(&module, &analyze(&module, &types))
        .map_err(|d| (d.message, src[d.span.start..d.span.end].to_string()))
}

fn mode_err(src: &str) -> (String, String) {
    match checked(src) {
        Ok(()) => panic!("expected a mode error for:\n{src}"),
        Err(e) => e,
    }
}

#[test]
fn tempered_functions_write_the_modes_their_bodies_need() {
    let grow = "temper def grow(xs: list[int]) -> None:\n    xs.append(1)\ngrow([0])\n";
    assert_eq!(
        mode_err(grow),
        (
            "tempered `grow` changes `xs` in place; write `mut xs`".into(),
            "xs".into()
        )
    );
    let ok = "temper def grow(mut xs: list[int]) -> None:\n    xs.append(1)\ngrow([0])\n";
    assert_eq!(checked(ok), Ok(()));
    let keep = "kept: list[list[int]] = []\ntemper def keep(n: int, mut xs: list[int]) -> None:\n    kept.append(xs)\nkeep(1, [0])\n";
    assert_eq!(
        mode_err(keep).0,
        "tempered `keep` keeps `xs` after it returns; write `owned xs`"
    );
    assert_eq!(checked(&keep.replace("mut xs", "owned xs")), Ok(()));
}

#[test]
fn loose_functions_check_only_the_modes_they_write() {
    let unwritten = "kept: list[list[int]] = []\ndef keep(xs):\n    kept.append(xs)\ndef grow(xs):\n    xs.append(1)\nkeep([0])\ngrow([0])\n";
    assert_eq!(checked(unwritten), Ok(()));
    let weak = unwritten.replace("keep(xs)", "keep(mut xs)");
    assert_eq!(
        mode_err(&weak),
        (
            "`keep` keeps `xs` after it returns; write `owned xs`".into(),
            "xs".into()
        )
    );
}

#[test]
fn writing_more_than_the_body_needs_is_allowed() {
    // As over-declaring `raises` is: it reserves the right to change.
    let src = "temper def total(mut xs: list[int], owned ys: list[int], mut n: int) -> int:\n    return len(xs) + len(ys) + n\ndef peek(owned xs):\n    return xs[0]\nprint(total([1], [2], 3), peek([4]))\n";
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn uncalled_functions_are_checked_too() {
    let src = "temper def grow(xs: list[int]) -> None:\n    xs.append(1)\nprint(1)\n";
    assert_eq!(
        mode_err(src).0,
        "tempered `grow` changes `xs` in place; write `mut xs`"
    );
}

#[test]
fn a_body_with_try_and_raise_is_checked() {
    // B3 lowers `try`/`raise`, so they no longer fall back.
    let src = "temper def f(xs: list[int], n: int) -> int raises ValueError:\n    try:\n        if n < 0:\n            raise ValueError(\"neg\")\n        xs.append(n)\n    except ValueError:\n        xs.append(0)\n    return n\nprint(f([1], -1))\n";
    assert_eq!(analyzed(src).precision, Precision::Precise);
    assert_eq!(
        mode_err(src).0,
        "tempered `f` changes `xs` in place; write `mut xs`"
    );
}

#[test]
fn a_conservative_analysis_diagnoses_nothing() {
    // A lowering gap elsewhere leaves `grow` unchecked until it lowers.
    let src = "temper def grow(xs: list[int]) -> None:\n    xs.append(1)\nx = print(7)\ndef print(n):\n    return 1\n";
    conservative(src);
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn returning_a_parameter_is_not_a_mode() {
    // D3 bars it from tempered code with the body checks (A6), not here.
    let src = "def same(owned xs: list[int]) -> list[int]:\n    return xs\ndef also(xs: list[int]) -> list[int]:\n    return xs\nprint(same([1]), also([2]))\n";
    assert_eq!(checked(src), Ok(()));
}

/// `check`'s error for `src`, if any: for these tests, an aliasing one.
fn alias_err(src: &str) -> Option<(String, String)> {
    checked(src).err()
}

#[test]
fn a_second_name_changed_while_the_first_is_used_is_an_error() {
    // D4, with its fix-it.
    let src = "temper def f() -> int:\n    a = [1]\n    b = a\n    b.append(2)\n    return len(a)\nprint(f())\n";
    assert_eq!(
        alias_err(src),
        Some((
            "tempered `f` changes `b` while `a` may be the same list and is used later; copy instead of sharing it: `b = list(a)`".into(),
            "b.append(2)".into()
        ))
    );
    assert_eq!(checked(&src.replace("b = a", "b = list(a)")), Ok(()));
    let item = src.replace("b.append(2)", "b[0] = 2");
    assert_eq!(alias_err(&item).map(|e| e.1), Some("b[0]".into()));
}

#[test]
fn aliasing_is_checked_where_names_are_still_used() {
    // Non-lexical: `a` is never read after `b` changes.
    let dead = "temper def f() -> int:\n    a = [1]\n    b = a\n    n = len(a)\n    b.append(2)\n    return n + len(b)\nprint(f())\n";
    assert_eq!(checked(dead), Ok(()));
    // Rebinding `b` ends the sharing.
    let rebound = "temper def f() -> int:\n    a = [1]\n    b = a\n    b = [0]\n    b.append(2)\n    return len(a)\nprint(f())\n";
    assert_eq!(checked(rebound), Ok(()));
    // In a loop, the next iteration's read counts.
    let looped = "temper def f(n: int) -> int:\n    a = [1]\n    b = a\n    for i in range(n):\n        b.append(i)\n        n = len(a)\n    return n\nprint(f(2))\n";
    assert!(alias_err(looped).is_some());
}

#[test]
fn a_list_inside_another_is_its_own_list() {
    let src = "temper def f(mut rows: list[list[int]]) -> int:\n    r = rows[0]\n    r.append(1)\n    for s in rows:\n        s[0] = 0\n    return len(rows)\nprint(f([[1]]))\n";
    assert_eq!(checked(src), Ok(()));
    // Two names for one element are two names for one list.
    let twice = "temper def f(mut rows: list[list[int]]) -> int:\n    r = rows[0]\n    s = rows[0]\n    r.append(1)\n    return len(s)\nprint(f([[1]]))\n";
    assert!(alias_err(twice).is_some());
    let through = "temper def f(mut rows: list[list[int]]) -> int:\n    r = rows[0]\n    rows[0].append(1)\n    return len(r)\nprint(f([[1]]))\n";
    assert_eq!(
        alias_err(through),
        Some((
            "tempered `f` changes a list while `r` may be the same list and is used later; copy instead of sharing it: `r = list(...)`".into(),
            "rows[0].append(1)".into()
        ))
    );
    // A list stored in another is reached through it, not a second name.
    let stored = "temper def f() -> int:\n    b = [1]\n    outer: list[list[int]] = []\n    outer.append(b)\n    b.append(2)\n    x = outer[0]\n    b.append(3)\n    return len(x)\nprint(f())\n";
    assert_eq!(alias_err(stored).map(|e| e.1), Some("b.append(3)".into()));
}

#[test]
fn a_mut_argument_is_a_change() {
    let src = "temper def grow(mut xs: list[int]) -> None:\n    xs.append(1)\ntemper def f() -> int:\n    a = [1]\n    b = a\n    grow(b)\n    return len(a)\nprint(f())\n";
    assert_eq!(alias_err(src).map(|e| e.1), Some("grow(b)".into()));
    // A `mut` argument may change the lists inside it too.
    let inner = "temper def clear(mut rows: list[list[int]]) -> None:\n    rows[0][0] = 0\ntemper def f(mut rows: list[list[int]]) -> int:\n    r = rows[0]\n    clear(rows)\n    return len(r)\nprint(f([[1]]))\n";
    assert_eq!(alias_err(inner).map(|e| e.1), Some("clear(rows)".into()));
}

#[test]
fn changing_a_list_a_loop_runs_over_is_an_error() {
    let src = "temper def f() -> int:\n    xs = [1, 2]\n    for x in xs:\n        if x < 0:\n            xs.append(x)\n    return 0\nprint(f())\n";
    assert_eq!(
        alias_err(src),
        Some((
            "tempered `f` changes `xs` while a `for` loop over it is running; loop over a copy: `for ... in list(...)`".into(),
            "xs.append(x)".into()
        ))
    );
    assert_eq!(checked(&src.replace("in xs", "in list(xs)")), Ok(()));
}

#[test]
fn each_run_of_an_allocation_is_a_new_list() {
    let src = "temper def f(n: int) -> int:\n    prev = [0]\n    cur = [1]\n    rows: list[list[int]] = []\n    for i in range(n):\n        prev = cur\n        cur = [i]\n        cur.append(i)\n        rows.append(cur)\n    return len(prev) + len(rows)\nprint(f(3))\n";
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn a_narrowed_optional_is_the_same_name() {
    let src = "temper def f(mut xs: list[int] | None) -> int:\n    if xs is not None:\n        xs.append(1)\n        return len(xs)\n    return 0\nprint(f([1]))\n";
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn tempered_code_returns_no_parameter() {
    // D3, with its fix-it.
    let src = "temper def same(xs: list[int]) -> list[int]:\n    return xs\nprint(same([1]))\n";
    assert_eq!(
        alias_err(src),
        Some((
            "tempered `same` returns its parameter `xs`; return a copy: `return list(xs)`".into(),
            "return xs".into()
        ))
    );
    assert_eq!(
        checked(&src.replace("return xs", "return list(xs)")),
        Ok(())
    );
    let inner = "temper def first(rows: list[list[int]]) -> list[int]:\n    return rows[0]\nprint(first([[1]]))\n";
    assert_eq!(
        alias_err(inner).map(|e| e.0),
        Some("tempered `first` returns a value sharing a list with its parameter `rows`; return a copy made with `list(...)`".into())
    );
    let boxed = "temper def pair(xs: list[int]) -> tuple[list[int], int]:\n    return (xs, 1)\nprint(pair([1]))\n";
    assert!(alias_err(boxed).is_some());
    // A list it stored in a parameter is the caller's too.
    let stored = "temper def put(mut rows: list[list[int]]) -> list[int]:\n    r = [1]\n    rows.append(r)\n    return r\nprint(put([]))\n";
    assert!(alias_err(stored).is_some());
}

#[test]
fn aliasing_comes_before_modes() {
    // Fixing the alias may make the mode unnecessary.
    let src = "temper def f(xs: list[int]) -> int:\n    ys = xs\n    ys.append(1)\n    return len(xs)\nprint(f([1]))\n";
    assert_eq!(alias_err(src).map(|e| e.1), Some("ys.append(1)".into()));
}

#[test]
fn loose_code_keeps_python_aliasing() {
    let src = "def f(xs):\n    a = [1]\n    b = a\n    b.append(2)\n    for x in xs:\n        xs.append(x)\n        break\n    return xs\nprint(f([1]), len([1]))\n";
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn slots_of_a_tuple_from_elsewhere_may_be_one_list() {
    // The caller may pass `(xs, xs)`; A5 cannot see inside one argument.
    let src = "temper def f(mut pair: tuple[list[int], list[int]]) -> int:\n    a = pair[0]\n    b = pair[1]\n    a.append(2)\n    return len(b)\nxs = [1]\nprint(f((xs, xs)))\n";
    assert_eq!(alias_err(src).map(|e| e.1), Some("a.append(2)".into()));
    let made = "temper def two() -> tuple[list[int], list[int]]:\n    return ([1], [2])\ntemper def f() -> int:\n    pair = two()\n    a = pair[0]\n    b = pair[1]\n    a.append(2)\n    return len(b)\nprint(f())\n";
    assert_eq!(alias_err(made).map(|e| e.1), Some("a.append(2)".into()));
    let nested = "temper def f(mut g: list[list[list[int]]]) -> int:\n    a = g[0][0]\n    b = g[1][0]\n    a.append(2)\n    return len(b)\nprint(f([[[1]], [[2]]]))\n";
    assert_eq!(alias_err(nested).map(|e| e.1), Some("a.append(2)".into()));
}

#[test]
fn fields_of_a_tuple_the_body_built_are_known_apart() {
    let apart = "temper def f() -> int:\n    pair = ([1], [2])\n    a = pair[0]\n    b = pair[1]\n    a.append(3)\n    return len(b)\nprint(f())\n";
    assert_eq!(checked(apart), Ok(()));
    let same = "temper def f() -> int:\n    x = [1]\n    pair = (x, x)\n    a = pair[0]\n    b = pair[1]\n    a.append(3)\n    return len(b)\nprint(f())\n";
    assert_eq!(alias_err(same).map(|e| e.1), Some("a.append(3)".into()));
}

#[test]
fn an_exception_holding_a_list_leaves_aliasing_checked() {
    let escape = "temper def escape(owned xs: list[int]) -> list[int] raises ValueError:\n    if len(xs) == 0:\n        raise ValueError(xs)\n    return xs\nprint(escape([1]))\n";
    assert_eq!(analyzed(escape).precision, Precision::Precise);
    assert_eq!(
        alias_err(escape).map(|e| e.0),
        Some(
            "tempered `escape` returns its parameter `xs`; return a copy: `return list(xs)`".into()
        )
    );
    let shared = "temper def f(n: int) -> int raises ValueError:\n    a = [1]\n    b = a\n    b.append(n)\n    if n < 0:\n        raise ValueError(a)\n    return len(a)\nprint(f(1))\n";
    assert_eq!(alias_err(shared).map(|e| e.1), Some("b.append(n)".into()));
    let unwritten = "temper def f(xs: list[int]) -> int raises ValueError:\n    raise ValueError(xs)\nprint(1)\n";
    assert_eq!(
        mode_err(unwritten).0,
        "tempered `f` keeps `xs` after it returns; write `owned xs`"
    );
}

const ADD: &str = "temper def add(mut rows: list[list[int]], owned row: list[int]) -> None:\n    rows.append(row)\n";

/// The source text of the call `check` rejects in `ADD` plus `rest`.
fn shared_call(rest: &str) -> String {
    let (message, call) = mode_err(&format!("{ADD}{rest}"));
    assert_eq!(
        message,
        "tempered `add` keeps `row` after it returns, and this call may also pass that list as `rows`; pass a copy (`list(...)`)"
    );
    call
}

#[test]
fn passing_one_list_twice_to_a_tempered_mut_parameter_is_rejected() {
    let src = "temper def push(mut xs: list[int], ys: list[int]) -> None:\n    xs.append(len(ys))\na = [1]\npush(a, a)\n";
    assert_eq!(
        mode_err(src),
        (
            "tempered `push` changes `xs` in place, and this call may also pass that list as `ys`; pass a copy (`list(...)`)".into(),
            "push(a, a)".into()
        )
    );
    // The message names the parameter that changes, wherever it is.
    let late = "temper def push(ys: list[int], mut xs: list[int]) -> None:\n    xs.append(len(ys))\na = [1]\npush(a, a)\n";
    assert!(mode_err(late).0.starts_with("tempered `push` changes `xs`"));
    let fixed = src.replace("push(a, a)", "push(a, list(a))");
    assert_eq!(checked(&fixed), Ok(()));
}

#[test]
fn a_list_inside_another_argument_is_shared() {
    assert_eq!(shared_call("x = [1]\nadd([x], x)\n"), "add([x], x)");
    let tuple = "temper def f(mut xs: list[int], t: tuple[list[int], int]) -> None:\n    xs.append(t[1])\na = [1]\nf(a, (a, 2))\n";
    assert_eq!(mode_err(tuple).1, "f(a, (a, 2))");
    // Elements of one list are not told apart.
    let rows = "temper def push(mut xs: list[int], ys: list[int]) -> None:\n    xs.append(len(ys))\ndef g(rows: list[list[int]]):\n    push(rows[0], rows[1])\ng([[1], [2]])\n";
    assert_eq!(mode_err(rows).1, "push(rows[0], rows[1])");
}

#[test]
fn what_the_call_or_later_code_stores_does_not_count() {
    let src = "rows: list[list[int]] = []\nrow = [1]\nadd(rows, row)\nrow = [2]\nadd(rows, row)\n";
    assert_eq!(checked(&format!("{ADD}{src}")), Ok(()));
    let looped = "rows: list[list[int]] = []\nfor i in range(3):\n    row = [i]\n    add(rows, row)\n    rows.append([0])\n";
    assert_eq!(checked(&format!("{ADD}{looped}")), Ok(()));
}

#[test]
fn what_earlier_code_stores_counts() {
    let appended = "rows: list[list[int]] = []\nrow = [1]\nrows.append(row)\nadd(rows, row)\n";
    assert_eq!(shared_call(appended), "add(rows, row)");
    // The second iteration passes the list the first one kept.
    let looped = "rows: list[list[int]] = []\nrow = [1]\nfor i in range(2):\n    add(rows, row)\n";
    assert_eq!(shared_call(looped), "add(rows, row)");
    // A rebinding moves the name, not the list.
    let moved =
        "rows: list[list[int]] = []\nrow = [1]\nb = row\nrow = [2]\nadd(rows, b)\nadd(rows, row)\n";
    assert_eq!(checked(&format!("{ADD}{moved}")), Ok(()));
    assert_eq!(
        shared_call(&format!("{moved}add(rows, b)\n")),
        "add(rows, b)"
    );
}

#[test]
fn one_list_may_sit_at_different_depths() {
    // A list has one type, not one depth: `(xs, [xs])`.
    let src = "temper def f(mut pair: tuple[list[int], list[list[int]]]) -> int:\n    a = pair[0]\n    b = pair[1][0]\n    a.append(2)\n    return len(b)\nxs = [1]\nprint(f((xs, [xs])))\n";
    assert_eq!(alias_err(src).map(|e| e.1), Some("a.append(2)".into()));
    let globals = "xs = [1]\nrows = [xs]\ntemper def f() -> int:\n    a = xs\n    b = rows[0]\n    a.append(2)\n    return len(b)\nprint(f())\n";
    assert_eq!(alias_err(globals).map(|e| e.1), Some("a.append(2)".into()));
    // Lists of different types are never one list.
    let typed = "temper def f(mut pair: tuple[list[int], list[list[str]]]) -> int:\n    a = pair[0]\n    b = pair[1][0]\n    a.append(2)\n    return len(b)\nprint(f(([1], [[\"x\"]])))\n";
    assert_eq!(checked(typed), Ok(()));
}

#[test]
fn a_parameter_may_be_a_global() {
    // `f(saved)`: A5 sees one argument, so nothing else rules it out.
    let src = "saved = [1]\ntemper def f(mut xs: list[int]) -> int:\n    other = saved\n    xs.append(2)\n    return len(other)\nprint(f(saved))\n";
    assert_eq!(
        alias_err(src),
        Some((
            "tempered `f` changes `xs` while `other` may be the same list and is used later; copy instead of sharing it: `other = list(...)`".into(),
            "xs.append(2)".into()
        ))
    );
    let typed = src
        .replace("saved = [1]", "saved = [\"a\"]")
        .replace("print(f(saved))", "print(f([1]))");
    assert_eq!(checked(&typed), Ok(()));
    let returned = "cache = [1]\ntemper def f(xs: list[int]) -> list[int]:\n    return cache\nprint(f(cache))\n";
    assert_eq!(
        alias_err(returned).map(|e| e.0),
        Some("tempered `f` returns a list a global holds, which its parameter `xs` may hold too; return a copy made with `list(...)`".into())
    );
    let other_type = returned
        .replace("xs: list[int]", "xs: list[str]")
        .replace("f(cache)", "f([\"a\"])");
    assert_eq!(checked(&other_type), Ok(()));
}

#[test]
fn a_list_stored_in_a_global_may_be_inside_a_parameter() {
    let changed = "shelf: list[list[int]] = []\ntemper def f(mut rows: list[list[int]]) -> int:\n    b = [1]\n    shelf.append(b)\n    rows[0].append(2)\n    return len(b)\nprint(f(shelf))\n";
    // `rows` may be `shelf` itself, so changing `shelf` is the first error.
    assert_eq!(
        alias_err(changed).map(|e| e.1),
        Some("shelf.append(b)".into())
    );
    let returned = "shelf: list[list[int]] = []\ntemper def f(rows: list[list[int]]) -> list[int]:\n    b = [1]\n    shelf.append(b)\n    return b\nprint(f(shelf))\n";
    assert_eq!(
        alias_err(returned).map(|e| e.0),
        Some("tempered `f` returns a list a global holds, which its parameter `rows` may hold too; return a copy made with `list(...)`".into())
    );
}

#[test]
fn a_module_variable_read_after_a_change_is_used_later() {
    // Parameter changed, global read later: `f(saved)`.
    let src = "saved = [1]\ntemper def f(mut xs: list[int]) -> int:\n    xs.append(2)\n    return len(saved)\nprint(f(saved))\n";
    assert_eq!(
        alias_err(src),
        Some((
            "tempered `f` changes `xs` while module variable `saved` may be the same list and is used later; pass a copy instead, such as `list(saved)`".into(),
            "xs.append(2)".into()
        ))
    );
    // Through a callee that reads it.
    let callee = "saved = [1]\ntemper def peek() -> int:\n    return len(saved)\ntemper def f(mut xs: list[int]) -> int:\n    xs.append(2)\n    return peek()\nprint(f(saved))\n";
    assert_eq!(alias_err(callee).map(|e| e.1), Some("xs.append(2)".into()));
    // Global changed, another global of the type read later.
    let globals = "xs = [1]\nys = xs\ntemper def f() -> int:\n    xs.append(2)\n    return len(ys)\nprint(f())\n";
    assert_eq!(alias_err(globals).map(|e| e.1), Some("xs.append(2)".into()));
    // Changing a global and reading it again is one name.
    let same =
        "xs = [1]\ntemper def f() -> int:\n    xs.append(2)\n    return len(xs)\nprint(f())\n";
    assert_eq!(checked(same), Ok(()));
    // A read before the change, or of another type, is not used later.
    let before = "saved = [1]\ntemper def f(mut xs: list[int]) -> int:\n    n = len(saved)\n    xs.append(2)\n    return n\nprint(f(saved))\n";
    assert_eq!(checked(before), Ok(()));
    let typed = "saved = [\"a\"]\ntemper def f(mut xs: list[int]) -> int:\n    xs.append(2)\n    return len(saved)\nprint(f([1]))\n";
    assert_eq!(checked(typed), Ok(()));
}

#[test]
fn a_callee_changing_a_module_list_is_a_change() {
    // Zero arguments: the change is the callee's own, to `saved`.
    let looped = "saved = [1]\ntemper def touch() -> None:\n    saved.append(2)\ntemper def f() -> int:\n    for x in saved:\n        if x < 2:\n            touch()\n    return len(saved)\nprint(f())\n";
    assert_eq!(
        alias_err(looped),
        Some((
            "tempered `f` changes a list through `touch` while a `for` loop over it is running; loop over a copy: `for ... in list(...)`".into(),
            "touch()".into()
        ))
    );
    // Indirectly, and against a second name.
    let named = "saved = [1]\ntemper def touch() -> None:\n    saved.append(2)\ntemper def via() -> None:\n    touch()\ntemper def f() -> int:\n    a = saved\n    via()\n    return len(a)\nprint(f())\n";
    assert_eq!(alias_err(named).map(|e| e.1), Some("via()".into()));
    // A parameter may be `saved`.
    let param = "saved = [1]\ntemper def touch() -> None:\n    saved.append(2)\ntemper def f(xs: list[int]) -> int:\n    touch()\n    return len(xs)\nprint(f(saved))\n";
    assert_eq!(alias_err(param).map(|e| e.1), Some("touch()".into()));
    // Reading the changed module variable again is one name.
    let same = "saved = [1]\ntemper def touch() -> None:\n    saved.append(2)\ntemper def f() -> int:\n    touch()\n    return len(saved)\nprint(f())\n";
    assert_eq!(checked(same), Ok(()));
    // Helpers that only read, and lists of another type, are no change.
    let reads = "saved = [1]\ntemper def peek() -> int:\n    return len(saved)\ntemper def f() -> int:\n    t = 0\n    for x in saved:\n        t += peek()\n    return t\nprint(f())\n";
    assert_eq!(checked(reads), Ok(()));
    let typed = "saved = [\"a\"]\ntemper def touch() -> None:\n    saved.append(\"b\")\ntemper def f(xs: list[int]) -> int:\n    touch()\n    return len(xs)\nprint(f([1]))\n";
    assert_eq!(checked(typed), Ok(()));
}

#[test]
fn read_parameters_and_values_without_lists_may_share() {
    let read = "temper def both(xs: list[int], ys: list[int]) -> int:\n    return len(xs) + len(ys)\na = [1]\nprint(both(a, a))\n";
    assert_eq!(checked(read), Ok(()));
    let ints = "temper def push(mut xs: list[int], n: int) -> None:\n    xs.append(n)\na = [1]\npush(a, len(a))\n";
    assert_eq!(checked(ints), Ok(()));
    let distinct = "rows: list[list[int]] = []\nadd(rows, [1])\n";
    assert_eq!(checked(&format!("{ADD}{distinct}")), Ok(()));
}

#[test]
fn loose_callees_keep_python_aliasing() {
    // Even when the loose callee writes its modes.
    let src = "def add(mut rows, owned row):\n    rows.append(row)\nx = [1]\nadd([x], x)\ndef push(mut xs, ys):\n    xs.append(len(ys))\na = [1]\npush(a, a)\nprint(a)\n";
    assert_eq!(checked(src), Ok(()));
}

#[test]
fn calls_from_any_caller_are_checked() {
    let tempered = "temper def first(owned rows: list[list[int]]) -> None:\n    add(rows, rows[0])\nfirst([[1]])\n";
    assert_eq!(shared_call(tempered), "add(rows, rows[0])");
    // A tempered caller's own aliasing (D4) is reported first: the first
    // call may change `row` through `rows`, and `row` is used again.
    let twice = "temper def twice(mut rows: list[list[int]], owned row: list[int]) -> None:\n    add(rows, row)\n    add(rows, row)\ntwice([], [1])\n";
    assert_eq!(
        mode_err(&format!("{ADD}{twice}")),
        (
            "tempered `twice` changes a list while `row` may be the same list and is used later; copy instead of sharing it: `row = list(...)`".into(),
            "add(rows, row)".into()
        )
    );
    // Two module variables a function reads count as one list: the
    // module's code may have stored one in the other.
    let globals = "rows: list[list[int]] = []\nrow = [1]\ndef g():\n    add(rows, row)\ng()\n";
    assert_eq!(shared_call(globals), "add(rows, row)");
    let param = "rows: list[list[int]] = []\ndef g(row: list[int]):\n    add(rows, row)\ng([1])\n";
    assert_eq!(checked(&format!("{ADD}{param}")), Ok(()));
}

#[test]
fn a_conservative_analysis_rejects_no_call() {
    let src = "temper def push(mut xs: list[int], ys: list[int]) -> None:\n    xs.append(len(ys))\na = [1]\npush(a, a)\nx = print(7)\ndef print(n):\n    return 1\n";
    conservative(src);
    assert_eq!(checked(src), Ok(()));
}

const PUSH: &str =
    "temper def push(mut xs: list[int], ys: list[int]) -> None:\n    xs.append(len(ys))\n";

#[test]
fn a_loose_function_passing_its_parameters_on_takes_the_contract() {
    let forward =
        format!("{PUSH}def forward(xs: list[int], ys: list[int]) -> None:\n    push(xs, ys)\n");
    assert_eq!(
        mode_err(&format!("{forward}a = [1]\nforward(a, a)\n")),
        (
            "this call may pass one list as `xs` and `ys` of `forward`, and from there tempered `push` changes `xs` in place; pass a copy (`list(...)`)".into(),
            "forward(a, a)".into()
        )
    );
    let distinct = format!("{forward}a = [1]\nb = [2]\nforward(a, b)\nforward(a, list(a))\n");
    assert_eq!(checked(&distinct), Ok(()));
    // Through two loose functions, with the parameters swapped.
    let twice =
        format!("{forward}def swap(p, q):\n    forward(q, p)\na = [1]\nswap(a, [2])\nswap(a, a)\n");
    assert_eq!(mode_err(&twice).1, "swap(a, a)");
    // A list inside one argument, reaching the other parameter.
    let nested = format!("{PUSH}def nest(rows: list[list[int]], ys: list[int]) -> None:\n    push(rows[0], ys)\nr = [1]\nnest([[2]], r)\nnest([r], r)\n");
    assert_eq!(mode_err(&nested).1, "nest([r], r)");
}

#[test]
fn a_parameter_passed_on_beside_a_module_variable_must_not_be_it() {
    let useg =
        format!("{PUSH}g: list[int] = [7]\ndef useg(xs: list[int]) -> None:\n    push(xs, g)\n");
    assert_eq!(
        checked(&format!("{useg}useg([1])\na = [2]\nuseg(a)\n")),
        Ok(())
    );
    assert_eq!(
        mode_err(&format!("{useg}useg(g)\n")),
        (
            "this call may pass a list a module variable holds as `xs` of `useg`, and from there tempered `push` changes `xs` in place; pass a copy (`list(...)`)".into(),
            "useg(g)".into()
        )
    );
    // From a function too, where the module variable is one it reads.
    let inner = format!("{useg}def outer():\n    useg(g)\nouter()\n");
    assert_eq!(mode_err(&inner).1, "useg(g)");
    // A tempered caller takes the need as a loose one does.
    let tempered = format!("{PUSH}g: list[int] = [7]\ntemper def useg(mut xs: list[int]) -> None:\n    push(xs, g)\nuseg([1])\nuseg(g)\n");
    assert_eq!(mode_err(&tempered).1, "useg(g)");
}

#[test]
fn recursion_settles_needs() {
    let src = format!("{PUSH}def down(xs: list[int], ys: list[int], n: int) -> None:\n    if n == 0:\n        push(xs, ys)\n    else:\n        down(ys, xs, n - 1)\na = [1]\ndown(a, [2], 3)\ndown(a, a, 3)\n");
    assert_eq!(mode_err(&src).1, "down(a, a, 3)");
}

#[test]
fn a_store_through_one_parameter_may_show_through_another() {
    // The body stores `row` in `rows`, then hands `other[0]` and `row`
    // on: when `rows` is `other`, `push` gets `row` twice.
    let forward = format!("{PUSH}def forward(rows: list[list[int]], other: list[list[int]], row: list[int]) -> None:\n    rows.append(row)\n    push(other[0], row)\n");
    let disjoint =
        format!("{forward}shelf: list[list[int]] = []\nrow = [1]\nforward(shelf, [[2]], row)\n");
    assert_eq!(checked(&disjoint), Ok(()));
    assert_eq!(
        mode_err(&format!("{forward}shelf: list[list[int]] = []\nrow = [1]\nforward(shelf, shelf, row)\n")),
        (
            "this call may pass one list as `rows` and `other` of `forward`, and from there tempered `push` changes `xs` in place; pass a copy (`list(...)`)".into(),
            "forward(shelf, shelf, row)".into()
        )
    );
    // Also when what is stored is a list the body made, and only that
    // fresh list reaches the contract.
    let local = format!("{PUSH}def forward(rows: list[list[int]], other: list[list[int]]) -> None:\n    row = [1]\n    rows.append(row)\n    push(other[0], row)\n");
    let shelf = "shelf: list[list[int]] = []\n";
    assert_eq!(
        checked(&format!("{local}{shelf}forward(shelf, [[2]])\n")),
        Ok(())
    );
    assert_eq!(
        mode_err(&format!("{local}{shelf}forward(shelf, shelf)\n")).1,
        "forward(shelf, shelf)"
    );
    // The same through a module variable the body reads.
    let global = format!("{PUSH}g: list[list[int]] = [[0]]\ndef f(rows: list[list[int]], row: list[int]) -> None:\n    rows.append(row)\n    push(g[len(g) - 1], row)\nf([[1]], [5])\n");
    assert_eq!(checked(&global), Ok(()));
    let aliased = format!("{global}f(g, [6])\n");
    assert_eq!(mode_err(&aliased).1, "f(g, [6])");
    // A function that stores nothing may still take one list twice
    // where no contract pairs them.
    let reads = format!("{PUSH}def peek(a: list[int], b: list[int], c: list[int]) -> None:\n    push(c, [len(a) + len(b)])\nx = [1]\npeek(x, x, [2])\n");
    assert_eq!(checked(&reads), Ok(()));
    // Storing an int creates no sharing.
    let ints = format!("{PUSH}def bump(a: list[int], b: list[int], c: list[int]) -> None:\n    a.append(1)\n    push(c, b)\nx = [1]\nbump(x, x, [2])\n");
    assert_eq!(checked(&ints), Ok(()));
}
