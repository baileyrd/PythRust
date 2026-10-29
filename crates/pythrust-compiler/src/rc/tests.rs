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
fn a_copy_kept_past_its_statement_keeps_its_reference() {
    let p = elided("def f(s: str) -> str:\n    t = s\n    print(t)\n    return t\nprint(f('a'))\n");
    // `t = s` and returning `t` each own a reference.
    assert_eq!(retains(func(&p, "f")), 2);
}

#[test]
fn a_copy_released_while_its_source_lives_borrows_it() {
    let mut b = block(vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        Inst::Print {
            args: vec![Operand::Local(LocalId(1))],
            sep: None,
            end: None,
        },
        Inst::Release(LocalId(1)),
    ]);
    cancel(&mut b);
    assert_eq!(b.insts[0], copy(1, 0));
    assert_eq!(b.insts.len(), 3);
    assert_eq!(b.insts[2], Inst::Forget(LocalId(1)));
}

#[test]
fn a_copy_outliving_its_source_or_moved_on_stays() {
    let outlives = vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        Inst::Release(LocalId(0)),
        Inst::Release(LocalId(1)),
    ];
    let moved = vec![
        copy(1, 0),
        Inst::Retain(LocalId(1)),
        copy(2, 1),
        Inst::Forget(LocalId(1)),
        Inst::Release(LocalId(1)),
    ];
    let not_a_copy = vec![Inst::Retain(LocalId(1)), Inst::Release(LocalId(1))];
    for insts in [outlives, moved, not_a_copy] {
        let mut b = block(insts.clone());
        cancel(&mut b);
        assert_eq!(b.insts, insts);
    }
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
    };
    skip_null(&mut f);
    assert!(f.blocks[0].insts.is_empty());
    assert_eq!(f.blocks[2].insts, vec![Inst::Release(LocalId(1))]);
}
