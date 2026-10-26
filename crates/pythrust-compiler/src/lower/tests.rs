use crate::diag::{DiagKind, Diagnostic};
use crate::ir::{Func, Inst, LocalId, Program, Term, Ty};
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
    let (d, _) = rejected("x = 1.5\n");
    assert!(d.message.contains("floats"));
    let (d, _) = rejected("xs = [1]\n");
    assert!(d.message.contains("lists"), "{}", d.message);
    let (d, _) = rejected("def f(xs):\n    return 1\nprint(f([]))\n");
    assert!(d.message.contains("lists"), "{}", d.message);
}

#[test]
fn unsupported_operations_name_the_construct() {
    let (_, text) = rejected("print(1 / 2)\n");
    assert_eq!(text, "1 / 2");
    let (_, text) = rejected("print(2 ** 3)\n");
    assert_eq!(text, "2 ** 3");
    let (d, _) = rejected("print(1, end='')\n");
    assert!(d.message.contains("keyword arguments"));
    let (d, _) = rejected("print(len([1]))\n");
    assert!(
        d.message.contains("lists") || d.message.contains("list[int]"),
        "{}",
        d.message
    );
}

#[test]
fn unsupported_scoping_and_calls_name_the_construct() {
    let (d, text) = rejected("N = 3\ndef f():\n    return N\nprint(f())\n");
    assert!(d.message.contains("global"), "{}", d.message);
    assert_eq!(text, "N");
    let (d, text) = rejected("def f(x=1 // 0):\n    pass\nprint(2)\n");
    assert!(d.message.contains("default arguments"));
    assert_eq!(text, "1 // 0");
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
    assert!(d.message.contains("non-int"), "{}", d.message);
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
    assert!(d.message.contains("non-int"), "{}", d.message);
    let (d, _) = rejected("for x in [1]:\n    print(x)\n");
    assert!(d.message.contains("lists"), "{}", d.message);
}
