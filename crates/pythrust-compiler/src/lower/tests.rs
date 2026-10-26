use crate::diag::{DiagKind, Diagnostic};
use crate::ir::{Compound, Func, Inst, LocalId, Operand, Program, Rvalue, Term, Ty};
use crate::{infer, lexer, parser};

fn lower_src(src: &str) -> crate::diag::Result<Program> {
    let module = parser::parse(&lexer::lex(src)?)?;
    let types = infer::infer(&module)?;
    super::lower(&module, &types)
}

fn lowered(src: &str) -> Program {
    match lower_src(src) {
        Ok(p) => p,
        Err(d) => panic!("lowering failed: {d}"),
    }
}

/// The backend error for `src`, and the source text its span covers.
fn rejected(src: &str) -> (Diagnostic, &str) {
    match lower_src(src) {
        Ok(_) => panic!("lowered, expected a backend error"),
        Err(d) => {
            assert_eq!(d.kind, DiagKind::Backend, "{d}");
            let text = &src[d.span.start..d.span.end];
            (d, text)
        }
    }
}

fn fails(p: &Program) -> usize {
    let blocks = p.funcs.iter().flat_map(|f| &f.blocks);
    blocks.filter(|b| matches!(b.term, Term::Fail(_))).count()
}

#[test]
fn main_is_the_module_and_calls_create_instances() {
    let p = lowered("def fib(n):\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(10))\n");
    assert_eq!(p.funcs.len(), 2);
    assert_eq!(p.funcs[p.main.0].name, "<module>");
    assert_eq!(p.funcs[p.main.0].ret, Ty::None);
    let fib = &p.funcs[1];
    assert_eq!((fib.name.as_str(), fib.ret), ("fib", Ty::Int));
    assert_eq!(fib.params.len(), 1);
}

#[test]
fn generic_functions_get_one_instance_per_type() {
    let p = lowered("def same(x):\n    return x\nprint(same(1), same(True), same(2))\n");
    let rets: Vec<Ty> = p.funcs.iter().skip(1).map(|f| f.ret).collect();
    assert_eq!(rets.len(), 2);
    assert!(rets.contains(&Ty::Int) && rets.contains(&Ty::Bool));
}

#[test]
fn unreached_functions_are_not_lowered() {
    let p = lowered("def unused(s):\n    return s + 'x'\nprint(1)\n");
    assert_eq!(p.funcs.len(), 1);
}

#[test]
fn range_checks_a_zero_step_only_when_given_one() {
    assert_eq!(fails(&lowered("for i in range(3):\n    print(i)\n")), 0);
    assert_eq!(fails(&lowered("for i in range(0, 3):\n    print(i)\n")), 0);
    let p = lowered("s = 1\nfor i in range(0, 3, s):\n    print(i)\n");
    assert_eq!(fails(&p), 1);
}

#[test]
fn code_after_return_and_break_lowers() {
    let src = "def f(n):\n    while True:\n        break\n        print(n)\n    return n\n    print(n)\nprint(f(1))\n";
    assert_eq!(lowered(src).funcs.len(), 2);
}

#[test]
fn unsupported_values_name_the_construct() {
    let (d, _) = rejected("r = range(3)\nprint(len(r))\n");
    assert!(d.message.contains("range"), "{}", d.message);
}

/// Every rvalue of every function of `p`.
fn all_rvalues(p: &Program) -> Vec<&Rvalue> {
    p.funcs.iter().flat_map(rvalues).collect()
}

#[test]
fn optionals_wrap_on_assignment_and_unwrap_where_narrowed() {
    let p = lowered("x: int | None = None\nx = 2\nif x is not None:\n    print(x + 1)\n");
    let rv = all_rvalues(&p);
    let count = |f: fn(&Rvalue) -> bool| rv.iter().filter(|r| f(r)).count();
    assert_eq!(count(|r| matches!(r, Rvalue::Wrap(_))), 2);
    assert_eq!(count(|r| matches!(r, Rvalue::IsNone(_))), 1);
    assert_eq!(count(|r| matches!(r, Rvalue::Unwrap(_))), 1);
    assert_eq!(p.types, vec![Compound::Opt(Ty::Int)]);
}

#[test]
fn comparing_an_optional_with_none_needs_no_unwrap() {
    let p = lowered("x: str | None = None\nprint(x == None, x != \"a\")\n");
    let rv = all_rvalues(&p);
    let unwraps = rv.iter().filter(|r| matches!(r, Rvalue::Unwrap(_))).count();
    // Only `x != "a"` reads the present value.
    assert_eq!(unwraps, 1);
}

#[test]
fn falling_off_an_optional_function_returns_its_null_slot() {
    let p = lowered("def f(n: int) -> int | None:\n    if n > 0:\n        return n\nprint(f(1))\n");
    let f = p.funcs.iter().find(|f| f.name == "f").expect("f");
    assert!(matches!(f.ret, Ty::Opt(_)));
    assert!(matches!(f.blocks[1].term, Term::Return(Operand::Local(_))));
}

#[test]
fn optional_tuples_are_rebuilt_to_the_declared_type() {
    let p = lowered("t: tuple[int | None, str] = (None, \"a\")\nprint(t)\n");
    let rv = all_rvalues(&p);
    assert!(rv.iter().any(|r| matches!(r, Rvalue::Field(_, 1))));
    assert_eq!(
        rv.iter().filter(|r| matches!(r, Rvalue::NewSeq(_))).count(),
        2
    );
}

#[test]
fn unsupported_operations_name_the_construct() {
    let (_, text) = rejected("print(int(True))\n");
    assert_eq!(text, "int(True)");
    let (d, text) = rejected("a, b = [1, 2]\n");
    assert!(d.message.contains("unpacking"), "{}", d.message);
    assert_eq!(text, "a, b");
}

#[test]
fn only_module_variables_functions_read_are_mirrored() {
    let p = lowered("n = 1\nm = 2\ndef f() -> int:\n    return n\nn = 3\nprint(f(), m)\n");
    let main = &p.funcs[p.main.0];
    let sets = main.blocks.iter().flat_map(|b| &b.insts);
    let sets = sets.filter(|i| matches!(i, Inst::SetGlobal { .. })).count();
    // Two writes of `n`, each storing the value and the bound flag.
    assert_eq!(sets, 4);
    assert_eq!(p.globals, vec![Ty::Int, Ty::Bool]);
}

#[test]
fn a_function_reading_a_module_variable_checks_it_is_bound() {
    let p = lowered("def f() -> str:\n    return s\ns = 'x'\nprint(f())\n");
    assert_eq!(fails(&p), 1, "NameError before the module assigns it");
    let main = &p.funcs[p.main.0];
    let drops = main.blocks[1].insts.iter();
    let drops = drops.filter(|i| matches!(i, Inst::DropGlobal(_))).count();
    assert_eq!(drops, 1, "the module's end drops the string global");
}

#[test]
fn defaults_are_stored_when_the_def_runs() {
    let src = "def f(a: int, b: str = 'd') -> str:\n    return b\nprint(f(1), f(2, 'e'))\n";
    let p = lowered(src);
    let main = &p.funcs[p.main.0];
    let insts = || main.blocks.iter().flat_map(|b| &b.insts);
    let stored = insts()
        .filter(|i| matches!(i, Inst::SetGlobal { .. }))
        .count();
    let loaded = insts().filter(|i| {
        matches!(
            i,
            Inst::Assign {
                value: crate::ir::Rvalue::Global(_),
                ..
            }
        )
    });
    assert_eq!(
        (stored, loaded.count()),
        (1, 1),
        "one default, one call omitting it"
    );
}

#[test]
fn unsupported_calls_name_the_construct() {
    let src =
        "def twice(n):\n    return sq(sq(n))\ndef sq(n):\n    return n * n\nprint(twice(3))\n";
    let (d, text) = rejected(src);
    assert!(d.message.contains("generic code"), "{}", d.message);
    assert_eq!(text, "sq(n)");
}

#[test]
fn calls_before_a_def_fail_or_are_rejected() {
    let p = lowered("print(f())\ndef f():\n    return 7\n");
    assert_eq!(fails(&p), 1, "a direct early call raises NameError");
    let src = "def g():\n    return f()\nprint(g())\ndef f():\n    return 7\n";
    let (d, text) = rejected(src);
    assert!(d.message.contains("`f`"), "{}", d.message);
    assert_eq!(text, "g");
    assert_eq!(fails(&lowered("def f():\n    return 7\nprint(f())\n")), 0);
}

#[test]
fn builtins_stay_callable_until_a_shadowing_def_runs() {
    let p = lowered("print(7)\ndef print(x):\n    pass\nprint(8)\n");
    assert_eq!(fails(&p), 0);
    // The builtin call before the def, then the user's `print` after it.
    assert_eq!(p.funcs.len(), 2);
    // Typed by the later `def` as int, but the builtin gives None.
    let (d, text) = rejected("x = print(7)\ndef print(n):\n    return 1\n");
    assert!(d.message.contains("shadows"), "{}", d.message);
    assert_eq!(text, "print(7)");
}

#[test]
fn reads_check_boundness_unless_known() {
    // Assigned in the entry block: never checked.
    assert_eq!(
        fails(&lowered("x = 1\nwhile x < 3:\n    x += 1\nprint(x)\n")),
        0
    );
    // Maybe unbound after the branch: checked once.
    assert_eq!(fails(&lowered("if True:\n    y = 1\nprint(y)\n")), 1);
}

#[test]
fn bool_int_mixing_is_rejected_not_miscompiled() {
    let (d, _) = rejected("print(True + 1)\n");
    assert!(d.message.contains("non-number"), "{}", d.message);
    let (d, _) = rejected("print(True + 1.5)\n");
    assert!(d.message.contains("non-number"), "{}", d.message);
    let (d, _) = rejected("print(True == 1)\n");
    assert!(d.message.contains("different types"), "{}", d.message);
}

/// The function named `name`.
fn func<'p>(p: &'p Program, name: &str) -> &'p Func {
    match p.funcs.iter().find(|f| f.name == name) {
        Some(f) => f,
        None => panic!("no function `{name}`"),
    }
}

fn count(f: &Func, pred: impl Fn(&Inst) -> bool) -> usize {
    f.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter(|i| pred(i))
        .count()
}

#[test]
fn returns_release_every_string_but_the_result() {
    let p = lowered("def f(s):\n    t = s + 'x'\n    return t\nprint(f('a'))\n");
    let f = func(&p, "f");
    let exits: Vec<&crate::ir::Block> = f
        .blocks
        .iter()
        .filter(|b| matches!(b.term, Term::Return(_)))
        .collect();
    assert_eq!(exits.len(), 1, "every return goes through one exit");
    let strs = f.locals.iter().filter(|t| **t == Ty::Str).count();
    let released = exits[0]
        .insts
        .iter()
        .filter(|i| matches!(i, Inst::Release(_)))
        .count();
    assert_eq!(released, strs - 1, "all but the result slot");
    // The parameter is the caller's to give: the call moves an owned copy.
    let main = &p.funcs[p.main.0];
    assert!(count(main, |i| matches!(i, Inst::Forget(_))) >= 1);
}

#[test]
fn writes_own_the_new_string_before_releasing_the_old() {
    let p = lowered("x = 'a'\nx = x\nprint(x)\n");
    let main = &p.funcs[p.main.0];
    let insts: Vec<&Inst> = main.blocks.iter().flat_map(|b| &b.insts).collect();
    // `x` is local 1 (after the literal's temporary); the copy is local 3.
    let (x, copy) = (LocalId(1), LocalId(3));
    let moved = insts.windows(4).any(|w| match w {
        [Inst::Retain(t), Inst::Release(r), Inst::Assign { dest, value, .. }, Inst::Forget(f)] => {
            (*t, *r, *dest, *f) == (copy, x, x, copy)
                && *value == crate::ir::Rvalue::Use(crate::ir::Operand::Local(copy))
        }
        _ => false,
    });
    assert!(moved, "retain the copy, release `x`, move: {insts:?}");
}

#[test]
fn string_operations_outside_the_subset_are_rejected() {
    let (d, _) = rejected("print('a' * True)\n");
    assert!(d.message.contains("non-number"), "{}", d.message);
    let (d, text) = rejected("for x in (1, 2):\n    print(x)\n");
    assert!(d.message.contains("`for` over"), "{}", d.message);
    assert_eq!(text, "(1, 2)");
}

#[test]
fn leaving_a_loop_releases_the_statements_it_leaves() {
    // `continue` from the inner `for`'s `else` leaves that `for`
    // statement, whose snapshot is the literal's temporary: released on
    // that edge, at the statement's end, and at exit.
    let src = "for i in range(2):\n    for ch in 'ab':\n        pass\n    else:\n        continue\n    print(i)\n";
    let p = lowered(src);
    let main = &p.funcs[p.main.0];
    let insts = || main.blocks.iter().flat_map(|b| &b.insts);
    let lit = insts().find_map(|i| match i {
        Inst::Assign {
            dest,
            value: crate::ir::Rvalue::StrLit(_),
            ..
        } => Some(*dest),
        _ => None,
    });
    let Some(lit) = lit else {
        panic!("no literal");
    };
    assert_eq!(insts().filter(|i| **i == Inst::Release(lit)).count(), 3);
}

/// The rvalues assigned anywhere in `f`.
fn rvalues(f: &Func) -> Vec<&crate::ir::Rvalue> {
    let insts = f.blocks.iter().flat_map(|b| &b.insts);
    insts
        .filter_map(|i| match i {
            Inst::Assign { value, .. } => Some(value),
            _ => None,
        })
        .collect()
}

#[test]
fn ints_meeting_floats_compute_in_floats() {
    use crate::ir::{FloatOp, Operand, Rvalue};
    let p = lowered("n = 3\nprint(n / 2, n * 1.5, 2 < 2.5, n ** 2)\n");
    let main = &p.funcs[p.main.0];
    let values = rvalues(main);
    // An int constant converts at compile time, a variable at run time.
    let div = values.iter().find_map(|v| match v {
        Rvalue::Float(FloatOp::Div, _, b) => Some(*b),
        _ => None,
    });
    assert_eq!(div, Some(Operand::Float(2.0)));
    assert!(values.iter().any(|v| matches!(v, Rvalue::ToFloat(_))));
    assert!(values.iter().any(|v| matches!(v, Rvalue::IntPow(_, _))));
    let cmp = values.iter().any(|v| {
        matches!(v, Rvalue::Cmp(_, Operand::Float(a), Operand::Float(b)) if (*a, *b) == (2.0, 2.5))
    });
    assert!(cmp, "{values:?}");
}

#[test]
fn conversions_pick_their_rvalue_by_type() {
    use crate::ir::Rvalue;
    let p = lowered("s = '2'\nf = 2.5\nprint(int(f), int(s), float(s), str(f), float(f))\n");
    let values = rvalues(&p.funcs[p.main.0]);
    let has = |pred: fn(&Rvalue) -> bool| values.iter().any(|v| pred(v));
    assert!(has(|v| matches!(v, Rvalue::FloatToInt(_))));
    assert!(has(|v| matches!(v, Rvalue::ParseInt(_))));
    assert!(has(|v| matches!(v, Rvalue::ParseFloat(_))));
    assert!(has(|v| matches!(v, Rvalue::ToStr(_))));
}

#[test]
fn loop_bindings_release_what_they_create_each_iteration() {
    // The item and the inner tuple's field are temporaries of the
    // per-iteration binding: each is released there and once more at
    // exit, never left for the next iteration to overwrite.
    let p = lowered("for (a, b), c in [((1, 'x'), 3)]:\n    print(a, b, c)\n");
    let main = &p.funcs[p.main.0];
    let insts = || main.blocks.iter().flat_map(|b| &b.insts);
    let made: Vec<LocalId> = insts()
        .filter_map(|i| match i {
            Inst::Assign {
                dest,
                value: crate::ir::Rvalue::Index(..) | crate::ir::Rvalue::Field(_, 0),
                ..
            } if main.locals[dest.0].is_rc() => Some(*dest),
            _ => None,
        })
        .collect();
    assert_eq!(made.len(), 2, "the item and the inner tuple");
    for t in made {
        assert_eq!(insts().filter(|i| **i == Inst::Release(t)).count(), 2);
    }
}

#[test]
fn an_unconstrained_empty_list_lowers() {
    let p = lowered("print(not [], [])\n");
    assert_eq!(p.types, vec![crate::ir::Compound::List(Ty::None)]);
}

#[test]
fn keyword_arguments_bind_in_parameter_order() {
    let p = lowered("def f(a: int, b: str) -> int:\n    return a\nprint(f(b='x', a=2))\n");
    let main = &p.funcs[p.main.0];
    let call = main
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find_map(|i| match i {
            Inst::Assign {
                value: crate::ir::Rvalue::Call(_, args),
                ..
            } => Some(args.clone()),
            _ => None,
        });
    let Some(args) = call else {
        panic!("no call");
    };
    let tys: Vec<Ty> = args.iter().map(|a| main.ty(*a)).collect();
    assert_eq!(tys, vec![Ty::Int, Ty::Str]);
}

#[test]
fn print_keeps_sep_and_end_apart_from_its_values() {
    let p = lowered("print(1, 2, end='!', sep='-')\n");
    let main = &p.funcs[p.main.0];
    let print = main
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find_map(|i| match i {
            Inst::Print { args, sep, end } => Some((args.len(), sep.is_some(), end.is_some())),
            _ => None,
        });
    assert_eq!(print, Some((2, true, true)));
}
