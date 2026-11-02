use super::*;
use crate::ir::Ty;
use crate::span::Span;
use crate::{infer, lexer, lower, parser};

fn elided(src: &str) -> Program {
    let module = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
    let types = infer::infer(&module).expect("infer");
    elide(lower::lower(&module, &types).expect("lower"))
}

fn func<'p>(p: &'p Program, name: &str) -> &'p Func {
    p.funcs
        .iter()
        .find(|f| f.name == name)
        .expect("no such function")
}

fn count(f: &Func, pick: impl Fn(&Inst) -> bool) -> usize {
    f.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter(|i| pick(i))
        .count()
}

fn retains(f: &Func) -> usize {
    count(f, |i| matches!(i, Inst::Retain(_)))
}

fn releases(f: &Func) -> usize {
    count(f, |i| matches!(i, Inst::Release(_)))
}

fn copy(dest: usize, from: usize) -> Inst {
    Inst::Assign {
        dest: LocalId(dest),
        value: Rvalue::Use(Operand::Local(LocalId(from))),
        span: Span { start: 0, end: 0 },
    }
}

fn block(insts: Vec<Inst>) -> Block {
    Block {
        insts,
        term: Term::Unreachable,
    }
}

#[test]
fn a_parameter_only_read_is_borrowed_and_costs_no_refcount() {
    let p = elided("def f(xs: list[int], s: str) -> int:\n    return len(s) + xs[0]\nys = [1]\nprint(f(ys, 'ab'))\n");
    let f = func(&p, "f");
    assert_eq!(f.borrowed, f.params.iter().copied().collect());
    assert_eq!((retains(f), releases(f)), (0, 0));
    let main = &p.funcs[p.main.0];
    // `ys` is lent without a copy; the literal is released by the caller.
    assert_eq!(retains(main), 0);
}

#[test]
fn a_temporary_argument_to_a_borrowed_parameter_is_released_after_the_call() {
    let p = elided("def f(s: str) -> int:\n    return len(s)\nprint(f(str(7)))\n");
    let main = &p.funcs[p.main.0];
    let insts: Vec<&Inst> = main.blocks.iter().flat_map(|b| &b.insts).collect();
    let call = insts
        .iter()
        .position(|i| call_args(i).is_some())
        .expect("a call");
    let Some((_, [Operand::Local(arg)])) = call_args(insts[call]) else {
        panic!("one local argument");
    };
    assert_eq!(
        next_mention(&main.blocks[0].insts[call + 1..], *arg),
        Some(&Inst::Release(*arg))
    );
}

#[test]
fn a_reassigned_or_unbound_parameter_is_not_borrowed() {
    let p = elided("def f(s: str) -> int:\n    s = s + 'x'\n    return len(s)\nprint(f('a'))\n");
    assert!(func(&p, "f").borrowed.is_empty());
    let src = "def g(e):\n    try:\n        raise e\n    except ValueError as e:\n        pass\n    return 1\nprint(g(ValueError('a')))\n";
    assert!(func(&elided(src), "g").borrowed.is_empty());
}

#[test]
fn a_call_that_keeps_its_argument_needs_the_parameter_owned() {
    // A call shape `release_after_calls` cannot rewrite keeps the
    // parameter owned.
    let mut p = elided("def f(s: str) -> int:\n    return len(s)\nprint(f('a'))\n");
    let main = p.main.0;
    let callee = p
        .funcs
        .iter()
        .position(|f| f.name == "f")
        .map(FuncId)
        .expect("f");
    let b = &mut p.funcs[main].blocks[0];
    let call = b
        .insts
        .iter()
        .position(|i| call_args(i).is_some())
        .expect("a call");
    b.insts.truncate(call + 1);
    assert!(!moved_in(&p.funcs, callee, 0));
}

#[test]
fn a_copy_kept_past_its_statement_keeps_its_reference_until_its_last_use() {
    let p = elided("def f(s: str) -> str:\n    t = s\n    print(t)\n    return t\nprint(f('a'))\n");
    // `t = s` owns a reference; returning `t`, its last use, takes it.
    assert_eq!(retains(func(&p, "f")), 1);
}

/// A function of string locals whose blocks are `blocks`, after
/// [`copies`].
fn after_copies(blocks: Vec<Block>) -> Func {
    let mut f = Func {
        name: "f".to_string(),
        params: Vec::new(),
        ret: Ty::None,
        locals: vec![Ty::Str; 4],
        names: Default::default(),
        borrowed: BTreeSet::new(),
        blocks,
        regions: Vec::new(),
        tasks: Vec::new(),
    };
    copies(&mut f, &[BTreeSet::new()]);
    f
}

fn print(l: usize) -> Inst {
    Inst::Print {
        args: vec![Operand::Local(LocalId(l))],
        sep: None,
        end: None,
    }
}

#[test]
fn a_copy_released_while_its_source_lives_borrows_it() {
    let f = after_copies(vec![block(vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        print(1),
        Inst::Release(LocalId(1)),
        print(0),
    ])]);
    let want = vec![copy(1, 0), print(1), Inst::Forget(LocalId(1)), print(0)];
    assert_eq!(f.blocks[0].insts, want);
}

#[test]
fn a_copy_borrows_across_blocks_as_a_for_snapshot_does() {
    // b0 copies, b1 tests, b2 reads and loops, b3 releases: the shape of
    // `for v in xs` over a variable.
    let goto = |t| Term::Goto(crate::ir::BlockId(t));
    let f = after_copies(vec![
        Block {
            insts: vec![copy(1, 0), Inst::Retain(LocalId(1))],
            term: goto(1),
        },
        Block {
            insts: Vec::new(),
            term: Term::Branch {
                cond: Operand::Bool(true),
                then: crate::ir::BlockId(2),
                orelse: crate::ir::BlockId(3),
            },
        },
        Block {
            insts: vec![print(1)],
            term: goto(1),
        },
        Block {
            insts: vec![Inst::Release(LocalId(1)), print(0)],
            term: Term::Unreachable,
        },
    ]);
    assert_eq!(f.blocks[0].insts, vec![copy(1, 0)]);
    assert_eq!(f.blocks[3].insts[0], Inst::Forget(LocalId(1)));
}

#[test]
fn a_copy_of_a_source_never_read_again_takes_its_reference() {
    let f = after_copies(vec![block(vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        copy(2, 1),
        Inst::Forget(LocalId(1)),
        Inst::Release(LocalId(0)),
    ])]);
    assert_eq!(f.blocks[0].insts[1], Inst::Forget(LocalId(0)));
    // Read after its source is dropped, a copy takes the reference too.
    let f = after_copies(vec![block(vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        Inst::Release(LocalId(0)),
        print(1),
        Inst::Release(LocalId(1)),
    ])]);
    assert_eq!(f.blocks[0].insts[1], Inst::Forget(LocalId(0)));
}

#[test]
fn a_copy_whose_source_is_read_again_and_that_moves_on_stays() {
    let moved_and_read = vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        copy(2, 1),
        Inst::Forget(LocalId(1)),
        print(0),
    ];
    // Copied on, `a` could be read through the copy after `x` ends.
    let copied_then_source_ends = vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        copy(2, 1),
        Inst::Retain(LocalId(2)),
        // A call taking `x` ends it, and reads it.
        Inst::Assign {
            dest: LocalId(3),
            value: Rvalue::Call(FuncId(0), vec![Operand::Local(LocalId(0))]),
            span: Span { start: 0, end: 0 },
        },
        print(2),
        Inst::Release(LocalId(2)),
        Inst::Release(LocalId(1)),
    ];
    let not_a_copy = vec![Inst::Retain(LocalId(1)), Inst::Release(LocalId(1))];
    for insts in [moved_and_read, not_a_copy] {
        assert_eq!(
            after_copies(vec![block(insts.clone())]).blocks[0].insts,
            insts
        );
    }
    // The call reads `x`, so nothing may take its reference, and `a` is
    // copied, so `a` cannot borrow past `x`.
    let f = after_copies(vec![block(copied_then_source_ends)]);
    assert!(f.blocks[0].insts.contains(&Inst::Retain(LocalId(1))));
}

#[test]
fn a_returning_path_that_never_releases_the_copy_keeps_it() {
    let f = after_copies(vec![Block {
        insts: vec![copy(1, 0), Inst::Retain(LocalId(1)), print(0)],
        term: Term::Return(Operand::None),
    }]);
    assert!(f.blocks[0].insts.contains(&Inst::Retain(LocalId(1))));
}

#[test]
fn releases_of_locals_null_on_every_path_go() {
    let p = elided("x = 'a' + 'b'\nprint(x)\n");
    let main = &p.funcs[p.main.0];
    // The two literals, and `x` at the end; the moved-out sum is null.
    assert_eq!(releases(main), 3);
}

#[test]
fn a_local_set_on_some_path_keeps_its_release() {
    let mut f = Func {
        name: "f".to_string(),
        params: Vec::new(),
        ret: Ty::None,
        locals: vec![Ty::Bool, Ty::Str],
        names: Default::default(),
        borrowed: BTreeSet::new(),
        blocks: vec![
            Block {
                insts: vec![Inst::Release(LocalId(1))],
                term: Term::Branch {
                    cond: Operand::Local(LocalId(0)),
                    then: crate::ir::BlockId(1),
                    orelse: crate::ir::BlockId(2),
                },
            },
            Block {
                insts: vec![Inst::Assign {
                    dest: LocalId(1),
                    value: Rvalue::StrLit("a".to_string()),
                    span: Span { start: 0, end: 0 },
                }],
                term: Term::Goto(crate::ir::BlockId(2)),
            },
            Block {
                insts: vec![Inst::Release(LocalId(1))],
                term: Term::Return(Operand::None),
            },
        ],
        regions: Vec::new(),
        tasks: Vec::new(),
    };
    skip_null(&mut f);
    assert!(f.blocks[0].insts.is_empty());
    assert_eq!(f.blocks[2].insts, vec![Inst::Release(LocalId(1))]);
}

/// The elided program for `def f(s: str) -> T` with `body`, called once.
fn kept_by(sig: &str, body: &str, call: &str) -> Program {
    elided(&format!("def f({sig}:\n{body}\n{call}\n"))
}

#[test]
fn a_callee_storing_its_parameter_in_a_list_adds_its_own_reference() {
    let p = kept_by(
        "rows: list[str], s: str) -> None",
        "    rows.append(s)",
        "rows = []\nf(rows, 'a')\nprint(rows)",
    );
    let f = func(&p, "f");
    // The list takes a reference of its own, so `f` keeps nothing.
    assert_eq!(f.borrowed.len(), 2);
    assert_eq!(count(f, |i| matches!(i, Inst::Append { .. })), 1);
}

#[test]
fn a_callee_returning_or_boxing_its_parameter_adds_its_own_reference() {
    let returned = kept_by("s: str) -> str", "    return s", "print(f('a'))");
    let f = func(&returned, "f");
    assert_eq!(f.borrowed.len(), 1);
    // The result is a copy with its own `Retain`.
    assert_eq!(retains(f), 1);
    let boxed = kept_by(
        "xs: list[int]) -> list[list[int]]",
        "    return [xs, xs]",
        "print(f([1]))",
    );
    let f = func(&boxed, "f");
    assert_eq!(f.borrowed.len(), 1);
    assert_eq!(
        count(f, |i| matches!(
            i,
            Inst::Assign {
                value: Rvalue::NewSeq(_),
                ..
            }
        )),
        1
    );
}

#[test]
fn a_callee_raising_its_parameter_adds_its_own_reference() {
    let src = "def f(s: str) -> int:\n    raise ValueError(s)\ntry:\n    f('a')\nexcept ValueError as e:\n    print(e)\n";
    let p = elided(src);
    let f = func(&p, "f");
    assert_eq!(f.borrowed.len(), 1);
    assert_eq!(count(f, |i| matches!(i, Inst::Raise { .. })), 1);
}

#[test]
fn a_parameter_passed_on_directly_is_retained() {
    // Lowering always passes a copy; a parameter handed to a call as it
    // is would be moved, so it counts as kept.
    let p = elided("def g(s: str) -> int:\n    return len(s)\ndef f(s: str) -> int:\n    return g(s)\nprint(f('a'))\n");
    let mut f = func(&p, "f").clone();
    let s = Operand::Local(f.params[0]);
    let id = p
        .funcs
        .iter()
        .position(|g| g.name == "g")
        .map(FuncId)
        .expect("g");
    f.blocks[0].insts.push(Inst::Assign {
        dest: LocalId(f.locals.len() - 1),
        value: Rvalue::Call(id, vec![s]),
        span: Span { start: 0, end: 0 },
    });
    assert_eq!(super::retains(&f), BTreeSet::from([0]));
}

#[test]
fn every_instruction_but_a_read_or_the_exit_release_keeps() {
    let p = LocalId(0);
    let assign = |value| Inst::Assign {
        dest: LocalId(1),
        value,
        span: Span { start: 0, end: 0 },
    };
    assert!(keeps(&Inst::Release(p), p, false));
    assert!(!keeps(&Inst::Release(p), p, true));
    assert!(keeps(&Inst::Forget(p), p, true));
    assert!(keeps(&copy(0, 1), p, false));
    assert!(!keeps(&copy(1, 0), p, false));
    assert!(!keeps(&assign(Rvalue::Len(Operand::Local(p))), p, false));
}

/// Element reads of `f`'s held loops: (borrowed, counted).
fn item_reads(f: &Func) -> (usize, usize) {
    let held =
        |op: &Operand| matches!(op, Operand::Local(l) if matches!(f.locals[l.0], Ty::Held(_)));
    let borrowed = count(f, |i| {
        matches!(
            i,
            Inst::Assign {
                value: Rvalue::Item(..),
                ..
            }
        )
    });
    let counted = count(
        f,
        |i| matches!(i, Inst::Assign { value: Rvalue::Index(s, _), .. } if held(s)),
    );
    (borrowed, counted)
}

const READ_ONLY: &str = "def total(rows: list[str]) -> int:\n    n = 0\n    for s in rows:\n        n += len(s)\n    return n\n";

#[test]
fn a_held_loop_reads_an_element_only_read_without_counting_it() {
    let p = elided(&format!("{READ_ONLY}print(total(['a', 'bc']))\n"));
    let f = func(&p, "total");
    assert_eq!(item_reads(f), (1, 0));
    let releases_of_items = count(
        f,
        |i| matches!(i, Inst::Release(l) if f.locals[l.0] == Ty::Str),
    );
    assert_eq!(releases_of_items, 0);
}

/// `f`'s element reads of held loops, asserting it holds one.
fn held_item_reads(src: &str) -> (usize, usize) {
    let p = elided(src);
    let f = func(&p, "f");
    assert!(f.locals.iter().any(|t| matches!(t, Ty::Held(_))), "{src}");
    item_reads(f)
}

#[test]
fn an_element_read_after_the_hold_or_through_another_name_is_counted() {
    let bodies = [
        // Read after the loop, when the hold has ended.
        (
            "s = ''\n    for s in rows:\n        pass\n    return len(s)\n",
            "int",
        ),
        // Copied to another name.
        (
            "t = ''\n    for s in rows:\n        t = s\n    return len(t)\n",
            "int",
        ),
        // Returned.
        ("for s in rows:\n        return s\n    return ''\n", "str"),
    ];
    for (body, ret) in bodies {
        let src = format!("def f(rows: list[str]) -> {ret}:\n    {body}print(f(['a']))\n");
        assert_eq!(held_item_reads(&src), (0, 1), "{body}");
    }
}

/// A store takes its own count of what it stores, so an element stored
/// in a value of another type (a tuple in a `list[tuple[str, int]]`)
/// is still read without one, as `rc::borrows` lets a copy be.
#[test]
fn an_element_stored_elsewhere_is_read_without_a_count() {
    let src = concat!(
        "def f(rows: list[str]) -> int:\n",
        "    out: list[tuple[str, int]] = []\n",
        "    for s in rows:\n        out.append((s, 1))\n",
        "    return len(out)\n",
        "print(f(['a']))\n",
    );
    assert_eq!(held_item_reads(src), (1, 0));
}

/// Pins which element reads of the `for_items` fixture go uncounted, so
/// its native run tests the paths it names.
#[test]
fn the_for_items_fixture_reads_without_counts_where_it_says() {
    let p = elided(include_str!("../../../../examples/for_items.pyr"));
    let want = [
        ("total", (1, 0)),
        ("longest_sorted", (1, 0)),
        ("pairs", (2, 0)),
        ("tagged", (1, 0)),
        ("first_of", (1, 0)),
        ("checked", (1, 0)),
    ];
    for (name, reads) in want {
        assert_eq!(item_reads(func(&p, name)), reads, "{name}");
    }
}
