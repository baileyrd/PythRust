use crate::diag::{DiagKind, Diagnostic};
use crate::ir::{Program, Term, Ty};
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
    let (d, text) = rejected("print('hi')\n");
    assert!(d.message.contains("strings"), "{}", d.message);
    assert_eq!(text, "'hi'");
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
    let (d, _) = rejected("def f(x=1):\n    return x\nprint(f())\n");
    assert!(d.message.contains("default arguments"));
}

#[test]
fn bool_int_mixing_is_rejected_not_miscompiled() {
    let (d, _) = rejected("print(True + 1)\n");
    assert!(d.message.contains("non-int"), "{}", d.message);
    let (d, _) = rejected("print(True == 1)\n");
    assert!(d.message.contains("different types"), "{}", d.message);
}
