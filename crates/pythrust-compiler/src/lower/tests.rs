use crate::diag::{DiagKind, Diagnostic};
use crate::exc::ExcKind;
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
fn a_range_value_holds_its_bounds_and_loops_over_them() {
    let p = lowered("r = range(3)\nfor x in r:\n    print(x)\nprint(len(r), r[0], 1 in r)\n");
    // Only `len`, for a range too long for an int.
    assert_eq!(fails(&p), 1);
    let rv = all_rvalues(&p);
    let n = |pred: fn(&Rvalue) -> bool| rv.iter().filter(|r| pred(r)).count();
    assert_eq!(
        n(|r| matches!(r, Rvalue::NewSeq(items) if items.len() == 3)),
        1
    );
    assert_eq!(n(|r| matches!(r, Rvalue::Field(_, _))), 3);
    assert_eq!(n(|r| matches!(r, Rvalue::Index(_, _))), 1);
    let p = lowered("s = 2\nr = range(0, 3, s)\nprint(r)\n");
    assert_eq!(fails(&p), 1);
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
fn and_or_wrap_the_operand_they_select() {
    let p = lowered("x: int | None = None\nprint(x or 7)\n");
    let wraps = all_rvalues(&p);
    let wraps = wraps
        .iter()
        .filter(|r| matches!(r, Rvalue::Wrap(_)))
        .count();
    // The `None` stored in `x`, and the 7.
    assert_eq!(wraps, 2);
}

#[test]
fn arguments_take_their_parameters_optional_fields() {
    let p = lowered(
        "def f(x: tuple[int | None, str]) -> str:\n    return str(x)\nprint(f((None, \"a\")))\n",
    );
    let f = p.funcs.iter().find(|f| f.name == "f").expect("f");
    let param = f.locals[f.params[0].0];
    let Some(Compound::Tuple(fields)) = p.compound(param) else {
        panic!("a tuple parameter");
    };
    assert!(matches!(fields[0], Ty::Opt(_)));
}

#[test]
fn a_tuple_is_rebuilt_before_wrapping_into_an_optional() {
    let p = lowered("t: tuple[int | None, str] | None = (None, \"a\")\nprint(t)\n");
    let rv = all_rvalues(&p);
    // The `None` field, then the rebuilt tuple.
    assert_eq!(
        rv.iter().filter(|r| matches!(r, Rvalue::Wrap(_))).count(),
        2
    );
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
fn list_unpacking_checks_every_short_length() {
    let p = lowered("a, b, c = [1, 2, 3]\nprint(a, b, c)\n");
    // One check for too many items, one per shorter length (0, 1, 2).
    assert_eq!(fails(&p), 4);
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
fn bools_meet_numbers_as_ints() {
    let p = lowered("t = True\nprint(t + 1, t == 1, 'a' * t, -t, int(t))\n");
    let rv = all_rvalues(&p);
    let widened = rv
        .iter()
        .filter(|r| matches!(r, Rvalue::BoolToInt(_)))
        .count();
    assert_eq!(widened, 5);
    // A literal folds to its int.
    let p = lowered("print(True + 1)\n");
    assert!(!all_rvalues(&p)
        .iter()
        .any(|r| matches!(r, Rvalue::BoolToInt(_))));
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
fn for_over_a_tuple_indexes_it() {
    let p = lowered("for x in (1, 2):\n    print(x)\n");
    let rv = rvalues(&p.funcs[p.main.0]);
    assert!(rv.iter().any(|r| matches!(r, Rvalue::Index(..))));
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
fn list_copies_lower_to_their_own_rvalue() {
    use crate::ir::Rvalue;
    let p = lowered("a = [1]\nb = list(a)\nprint(b)\n");
    let values = rvalues(&p.funcs[p.main.0]);
    assert!(values.iter().any(|v| matches!(v, Rvalue::CopyList(_))));
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

#[test]
fn an_empty_tuple_loop_binds_nothing() {
    let p = lowered("x = 7\nfor x in ():\n    pass\nelse:\n    print(x)\n");
    let main = &p.funcs[p.main.0];
    // Only `x = 7` writes `x`: no item is read and the body is not lowered.
    let rv = rvalues(main);
    assert!(!rv.iter().any(|r| matches!(r, Rvalue::Index(..))));
}

#[test]
fn an_open_result_type_comes_from_the_instance_body() {
    let p = lowered("def square(n):\n    return n * n\n\n\ndef twice(n):\n    return square(square(n))\n\n\nprint(twice(3), twice(1.5))\n");
    let squares = p.funcs.iter().filter(|f| f.name == "square");
    let mut rets: Vec<Ty> = squares.map(|f| f.ret).collect();
    rets.sort_by_key(|t| format!("{t:?}"));
    assert_eq!(rets, vec![Ty::Float, Ty::Int]);
}

#[test]
fn recursion_before_the_deciding_return_lowers_in_a_second_pass() {
    let src = "def rec(n, x):\n    if n > 0:\n        return rec(n - 1, x) + x\n    return x * x\n\n\ndef outer(x):\n    return rec(2, x) + 1\n\n\nprint(outer(3))\n";
    let p = lowered(src);
    let recs: Vec<&Func> = p.funcs.iter().filter(|f| f.name == "rec").collect();
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].ret, Ty::Int);
}

/// The result type of the one instance of `name` in `p`.
fn ret_of(p: &Program, name: &str) -> Ty {
    let mut funcs = p.funcs.iter().filter(|f| f.name == name);
    let ret = funcs.next().map(|f| f.ret);
    assert!(funcs.next().is_none(), "one instance of `{name}`");
    ret.unwrap_or_else(|| panic!("no instance of `{name}`"))
}

/// `Some(elem)` if `ty` is `list[elem] | None`.
fn optional_list_elem(p: &Program, ty: Ty) -> Option<Ty> {
    let Ty::Opt(id) = ty else { return None };
    let Some(Compound::Opt(Ty::List(list))) = p.types.get(id.0) else {
        return None;
    };
    match p.types.get(list.0) {
        Some(Compound::List(elem)) => Some(*elem),
        _ => None,
    }
}

#[test]
fn a_function_that_never_returns_settles_its_result_as_none() {
    let p = lowered(
        "def f(n):\n    return f(n - 1)\n\n\ndef g(x):\n    return f(x)\n\n\nprint(g(1))\n",
    );
    assert_eq!(ret_of(&p, "f"), Ty::None);
    assert_eq!(ret_of(&p, "g"), Ty::None);
    let p =
        lowered("def spin(n):\n    return spin(n + 1) if n else spin(n - 1)\n\n\nprint(spin(1))\n");
    assert_eq!(ret_of(&p, "spin"), Ty::None);
}

fn prints(f: &Func) -> usize {
    let insts = f.blocks.iter().flat_map(|b| &b.insts);
    insts.filter(|i| matches!(i, Inst::Print { .. })).count()
}

/// Whether a block of `f` calls and then ends, unreachable.
fn ends_after_a_call(f: &Func) -> bool {
    f.blocks.iter().any(|b| {
        let call = matches!(
            b.insts.last(),
            Some(Inst::Assign {
                value: Rvalue::Call(..),
                ..
            })
        );
        call && matches!(b.term, Term::Unreachable)
    })
}

#[test]
fn an_operation_on_a_never_returning_result_is_unreachable() {
    let p = lowered("def f(n):\n    return f(n - 1) + 1\n\n\nprint(f(1))\n");
    assert_eq!(ret_of(&p, "f"), Ty::None);
    let main = &p.funcs[p.main.0];
    assert!(ends_after_a_call(main));
    assert_eq!(prints(main), 0);
    let f = p.funcs.iter().find(|f| f.name == "f");
    assert!(f.is_some_and(ends_after_a_call));
}

#[test]
fn a_short_circuit_past_a_never_returning_call_continues() {
    let p = lowered(
        "def f(n):\n    return f(n - 1) * 2\n\n\nprint(0 > 1 < f(1))\nprint(1 if 0 else 2)\n",
    );
    let main = &p.funcs[p.main.0];
    assert!(ends_after_a_call(main));
    assert_eq!(prints(main), 2);
}

#[test]
fn mutual_recursion_through_ops_never_returns() {
    let p = lowered("def ping(n):\n    return pong(n) + 1\n\n\ndef pong(n):\n    return ping(n - 1) * 2\n\n\nprint(ping(3))\n");
    assert_eq!(prints(&p.funcs[p.main.0]), 0);
}

#[test]
fn a_stuck_elif_test_is_not_never_returning() {
    // `big` falls off (returns `None`) through the first branch.
    let p = lowered("def big(n) -> list | None:\n    if n > 5:\n        print(1)\n    elif big(n + 1):\n        print(2)\n\n\nprint(big(6))\n");
    let main = &p.funcs[p.main.0];
    assert!(!ends_after_a_call(main));
    assert_eq!(prints(main), 1);
}

/// The int literals that prints in blocks of `f` reachable from its
/// entry print first.
fn reachable_prints(f: &Func) -> Vec<i64> {
    let (mut seen, mut todo) = (vec![false; f.blocks.len()], vec![0]);
    let mut out = Vec::new();
    while let Some(b) = todo.pop() {
        if std::mem::replace(&mut seen[b], true) {
            continue;
        }
        let block = &f.blocks[b];
        for i in &block.insts {
            if let Inst::Print { args, .. } = i {
                if let Some(Operand::Int(n)) = args.first() {
                    out.push(*n);
                }
            }
        }
        match &block.term {
            Term::Goto(t) => todo.push(t.0),
            Term::Branch { then, orelse, .. } => todo.extend([then.0, orelse.0]),
            _ => {}
        }
    }
    out.sort();
    out
}

#[test]
fn a_never_returning_elif_test_keeps_the_join() {
    let p = lowered("def spin(n):\n    return spin(n - 1)\n\n\nif True:\n    print(1)\nelif spin(1):\n    print(2)\nprint(3)\n");
    assert_eq!(reachable_prints(&p.funcs[p.main.0]), vec![1, 3]);
}

#[test]
fn a_never_returning_branch_decides_nothing() {
    for ret in ["spin(n) if n else x * x", "x * x if not n else spin(n)"] {
        for (arg, ty) in [("3", Ty::Int), ("1.5", Ty::Float)] {
            let src = format!("def spin(n):\n    return spin(n - 1)\n\n\ndef h(n, x):\n    return {ret}\n\n\ndef o(x):\n    return h(0, x) + 1\n\n\nprint(o({arg}))\n");
            assert_eq!(ret_of(&lowered(&src), "h"), ty, "{ret} on {arg}");
        }
    }
}

#[test]
fn a_variable_only_a_never_returning_call_typed_takes_the_live_type() {
    let dead = "    if n:\n        y = spin(n)\n";
    for read in ["print(y)", "print(y * y)", "print(y * y + 1)"] {
        let read = format!("    if n:\n        {read}\n");
        for body in [format!("{dead}{read}"), format!("{read}{dead}")] {
            for (arg, ty) in [("3", Ty::Int), ("1.5", Ty::Float)] {
                let src = format!("def spin(n):\n    return spin(n - 1)\n\n\ndef h(n, x):\n{body}    y = x * x\n    return y\n\n\ndef o(x):\n    return h(0, x) + 1\n\n\nprint(o({arg}))\n");
                assert_eq!(ret_of(&lowered(&src), "h"), ty, "{body} on {arg}");
            }
        }
    }
}

#[test]
fn a_variable_with_an_open_element_waits_for_its_live_type() {
    for (arg, elem) in [("3", Ty::Int), ("1.5", Ty::Float)] {
        let src = format!("def spin(n):\n    return spin(n - 1)\n\n\ndef h(n, x):\n    if n:\n        y = spin(n)\n    i = 0\n    while i < 2:\n        if i:\n            print(y)\n        y = [x * x]\n        i += 1\n    return y\n\n\ndef o(x):\n    return len(h(0, x))\n\n\nprint(o({arg}))\n");
        let p = lowered(&src);
        let elem_of = match ret_of(&p, "h") {
            Ty::List(id) => p.types.get(id.0),
            _ => None,
        };
        assert_eq!(elem_of, Some(&Compound::List(elem)), "{arg}");
    }
}

#[test]
fn an_open_element_waits_for_the_live_branch() {
    for ret in ["spin(n) if n else [x * x]", "[x * x] or spin(n)"] {
        for (arg, elem) in [("3", Ty::Int), ("1.5", Ty::Float)] {
            let src = format!("def spin(n):\n    return spin(n - 1)\n\n\ndef h(n, x):\n    return {ret}\n\n\ndef o(x):\n    return len(h(0, x))\n\n\nprint(o({arg}))\n");
            let p = lowered(&src);
            let elem_of = match ret_of(&p, "h") {
                Ty::List(id) => p.types.get(id.0),
                _ => None,
            };
            assert_eq!(elem_of, Some(&Compound::List(elem)), "{ret} on {arg}");
        }
    }
}

#[test]
fn a_global_only_a_never_returning_call_assigns_fails_when_read() {
    let p = lowered("def spin(n):\n    return spin(n - 1)\n\n\nif 0:\n    y = spin(1)\n\n\ndef f():\n    return y\n\n\nprint(f())\n");
    let f = p.funcs.iter().find(|f| f.name == "f");
    let failing = f.map(|f| {
        f.blocks
            .iter()
            .filter(|b| matches!(b.term, Term::Fail(_)))
            .count()
    });
    assert_eq!(failing, Some(1));
    assert_eq!(prints(&p.funcs[p.main.0]), 0);
}

#[test]
fn globals_after_a_never_returning_call_fail_when_read() {
    let module = "def spin(n):\n    return spin(n - 1)\n\n\nif 0:\n    spin(1)\n    y = spin(1)\n    if 1:\n        a, b = (spin(1), 2)\n    for c in range(2):\n        d = spin(1)\ni = 0\nwhile i < 0:\n    spin(1)\n    e = spin(2)\n\n\n";
    for (name, read) in [("f", "y"), ("g", "a"), ("h", "d"), ("k", "e")] {
        let src = format!("{module}def {name}():\n    return {read}\n\n\nprint({name}())\n");
        let p = lowered(&src);
        let f = p.funcs.iter().find(|f| f.name == name);
        let failing = f.map(|f| f.blocks.iter().any(|b| matches!(b.term, Term::Fail(_))));
        assert_eq!(failing, Some(true), "{read}");
    }
}

#[test]
fn a_variable_only_a_never_returning_call_assigns_fails_when_read() {
    let p =
        lowered("def spin(n):\n    return spin(n - 1)\n\n\nif 0:\n    z = spin(1)\nprint(z * 2)\n");
    assert_eq!(fails(&p), 1);
    assert_eq!(prints(&p.funcs[p.main.0]), 0);
}

#[test]
fn never_returning_results_assigned_or_in_and_or_lower() {
    for src in [
        "def f(n):\n    return f(n - 1) + 1\n\n\nx = f(1)\nprint(x)\n",
        "def f(n):\n    return f(n - 1) or f(n + 1)\n\n\nprint(f(1))\n",
        "def f(n):\n    return f(n - 1) and f(n + 1)\n\n\nprint(f(1))\n",
    ] {
        let p = lowered(src);
        assert_eq!(prints(&p.funcs[p.main.0]), 0, "{src}");
    }
}

#[test]
fn returning_none_through_a_waiting_callee_is_not_never_returning() {
    // `fold` returns only through `unfold`, which waits on `fold` but
    // may return `None` itself.
    let p = lowered("def fold(n):\n    return unfold(n)\n\n\ndef unfold(n) -> list | None:\n    if n:\n        return fold(n - 1)\n    return None\n\n\nprint(fold(3))\n");
    assert_eq!(prints(&p.funcs[p.main.0]), 1);
}

#[test]
fn an_optional_only_none_settles_its_open_element() {
    for body in [
        "    return None\n",
        "    print(x)\n",
        "    if x:\n        return None\n",
    ] {
        let src = format!(
            "def f(x) -> list | None:\n{body}\n\ndef g(x):\n    return f(x)\n\n\nprint(g(1))\n"
        );
        let p = lowered(&src);
        assert_eq!(
            optional_list_elem(&p, ret_of(&p, "f")),
            Some(Ty::None),
            "{body}"
        );
        assert_eq!(
            optional_list_elem(&p, ret_of(&p, "g")),
            Some(Ty::None),
            "{body}"
        );
    }
}

#[test]
fn a_conditional_decides_from_its_nonrecursive_branch() {
    let p = lowered("def h(n, x):\n    return h(n - 1, x) if n else x * x\n\n\ndef o(x):\n    return h(2, x) + 1\n\n\nprint(o(1.5))\n");
    assert_eq!(ret_of(&p, "h"), Ty::Float);
    // Its other branch returns, so `h` is not taken for one that never
    // does: no call ends a path.
    assert!(!p.funcs.iter().any(ends_after_a_call));
}

#[test]
fn an_instance_waiting_on_its_caller_is_not_settled() {
    // `pong` alone decides nothing, but `ping`, which it calls back,
    // decides later: settling `pong` early would make it `None`.
    let src = "def ping(n, x):\n    if n > 0:\n        return pong(n - 1, x)\n    return x * x\n\n\ndef pong(n, x):\n    return ping(n, x)\n\n\ndef o(x):\n    return ping(2, x) + 1\n\n\nprint(o(3))\n";
    let p = lowered(src);
    assert_eq!(ret_of(&p, "ping"), Ty::Int);
    assert_eq!(ret_of(&p, "pong"), Ty::Int);
}

#[test]
fn open_display_items_decide_the_display_type() {
    let p = lowered("def squares(n):\n    return [n * n]\n\n\ndef count(n):\n    return len(squares(n))\n\n\nprint(count(3))\n");
    let squares = p.funcs.iter().find(|f| f.name == "squares");
    let elem = squares.map(|f| {
        p.types.get(match f.ret {
            Ty::List(id) => id.0,
            _ => usize::MAX,
        })
    });
    assert_eq!(elem, Some(Some(&crate::ir::Compound::List(Ty::Int))));
}

/// How many of the module's assignments compute rvalues `pick` takes.
fn assigns(p: &Program, pick: impl Fn(&Rvalue) -> bool) -> usize {
    count(
        &p.funcs[p.main.0],
        |i| matches!(i, Inst::Assign { value, .. } if pick(value)),
    )
}

#[test]
fn only_what_may_raise_is_tested_after() {
    let src = "def f(n):\n    if n < 0:\n        raise ValueError\n    return n\ndef g(n):\n    return n\nprint(f(1), g(2), int(\"3\"))\n";
    let p = lowered(src);
    let raised = |r: &Rvalue| matches!(r, Rvalue::Raised);
    // `f(1)` and `int("3")`, not `g(2)`.
    assert_eq!(assigns(&p, raised), 2);
    let uncaught = |f: &Func| f.blocks.iter().any(|b| b.term == Term::Uncaught);
    assert!(uncaught(&p.funcs[p.main.0]));
    // A function unwinds to its caller through its exit block.
    assert!(!p
        .funcs
        .iter()
        .filter(|f| f.name != "<module>")
        .any(uncaught));
}

#[test]
fn handlers_test_their_kinds_in_order() {
    let src = "try:\n    raise IndexError\nexcept ValueError:\n    pass\nexcept IndexError as e:\n    print(e)\n";
    let p = lowered(src);
    let tests = |kind| move |r: &Rvalue| *r == Rvalue::PendingIs(kind);
    assert_eq!(assigns(&p, tests(ExcKind::Value)), 1);
    assert_eq!(assigns(&p, tests(ExcKind::Index)), 1);
    assert_eq!(assigns(&p, |r| matches!(r, Rvalue::Catch)), 2);
    let new = |r: &Rvalue| matches!(r, Rvalue::NewExc(ExcKind::Index, None));
    assert_eq!(assigns(&p, new), 1);
}

#[test]
fn a_handler_name_is_unbound_after_its_handler() {
    // Bound in the entry block first, so only the unbinding makes the
    // later read check its flag.
    let src = "e = ValueError()\ntry:\n    raise e\nexcept ValueError as e:\n    pass\nprint(e)\n";
    let p = lowered(src);
    let name_errors = p.funcs[p.main.0].blocks.iter().filter(
        |b| matches!(&b.term, Term::Fail(d) if d.message.starts_with("NameError: name 'e'")),
    );
    assert_eq!(name_errors.count(), 1);
}

#[test]
fn an_exception_holding_a_list_lowers() {
    // The native backend rejects it (`codegen`); the analyses see it.
    let p = lowered("e = ValueError([1])\n");
    let insts = p.funcs[p.main.0].blocks.iter().flat_map(|b| &b.insts);
    let held = insts.filter(|i| {
        matches!(
            i,
            crate::ir::Inst::Assign {
                value: crate::ir::Rvalue::NewExc(_, Some(_)),
                ..
            }
        )
    });
    assert_eq!(held.count(), 1);
}

#[test]
fn a_parameter_a_handler_rebinds_is_checked_after_it() {
    let src = "def f(e: Exception) -> None:\n    try:\n        raise e\n    except ValueError as e:\n        pass\n    print(e)\nf(ValueError())\n";
    let p = lowered(src);
    let unbound = |b: &crate::ir::Block| matches!(&b.term, Term::Fail(d) if d.message.starts_with("UnboundLocalError"));
    let f = p.funcs.iter().find(|f| f.name == "f").map(|f| &f.blocks);
    // Both reads: a name a handler unbinds is never assumed bound.
    assert_eq!(f.into_iter().flatten().filter(|b| unbound(b)).count(), 2);
}
