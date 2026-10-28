//! Interpreter tests over hand-built ASTs. Expected output is CPython 3.12's
//! except where `docs/grammar.md` lists a deviation.

use super::run;
use crate::ast::*;
use crate::diag::{DiagKind, Diagnostic};
use crate::span::Span;

// ---- AST builders -------------------------------------------------------

const Z: Span = Span { start: 0, end: 0 };

fn sp(start: usize, end: usize) -> Span {
    Span { start, end }
}

fn ex(kind: ExprKind) -> Expr {
    Expr { kind, span: Z }
}

fn at(span: Span, e: Expr) -> Expr {
    Expr { span, ..e }
}

fn int(i: i64) -> Expr {
    ex(ExprKind::Int(i))
}

fn float(f: f64) -> Expr {
    ex(ExprKind::Float(f))
}

fn st(s: &str) -> Expr {
    ex(ExprKind::Str(s.to_string()))
}

fn boolean(b: bool) -> Expr {
    ex(ExprKind::Bool(b))
}

fn none() -> Expr {
    ex(ExprKind::None)
}

fn name(n: &str) -> Expr {
    ex(ExprKind::Name(n.to_string()))
}

fn list(items: Vec<Expr>) -> Expr {
    ex(ExprKind::List(items))
}

fn tuple(items: Vec<Expr>) -> Expr {
    ex(ExprKind::Tuple(items))
}

fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
    ex(ExprKind::Binary {
        op,
        left: Box::new(l),
        right: Box::new(r),
    })
}

fn un(op: UnaryOp, x: Expr) -> Expr {
    ex(ExprKind::Unary {
        op,
        operand: Box::new(x),
    })
}

fn logical(op: LogicalOp, l: Expr, r: Expr) -> Expr {
    ex(ExprKind::Logical {
        op,
        left: Box::new(l),
        right: Box::new(r),
    })
}

fn cmp(left: Expr, rest: Vec<(CmpOp, Expr)>) -> Expr {
    ex(ExprKind::Compare {
        left: Box::new(left),
        rest,
    })
}

fn if_exp(test: Expr, body: Expr, orelse: Expr) -> Expr {
    ex(ExprKind::IfExp {
        test: Box::new(test),
        body: Box::new(body),
        orelse: Box::new(orelse),
    })
}

fn ident(n: &str) -> Ident {
    Ident {
        name: n.to_string(),
        span: Z,
    }
}

fn arg(value: Expr) -> Arg {
    Arg {
        name: None,
        value,
        span: Z,
    }
}

fn kwarg(n: &str, value: Expr) -> Arg {
    Arg {
        name: Some(ident(n)),
        value,
        span: Z,
    }
}

fn call_args(func: Expr, args: Vec<Arg>) -> Expr {
    ex(ExprKind::Call {
        func: Box::new(func),
        args,
    })
}

fn call(f: &str, args: Vec<Expr>) -> Expr {
    call_args(name(f), args.into_iter().map(arg).collect())
}

fn sub(value: Expr, index: Expr) -> Expr {
    ex(ExprKind::Subscript {
        value: Box::new(value),
        index: Box::new(index),
    })
}

fn attr(value: Expr, a: &str) -> Expr {
    ex(ExprKind::Attr {
        value: Box::new(value),
        attr: ident(a),
    })
}

fn append(list: &str, v: Expr) -> Stmt {
    expr(call_args(attr(name(list), "append"), vec![arg(v)]))
}

fn stmt(kind: StmtKind) -> Stmt {
    Stmt { kind, span: Z }
}

fn expr(e: Expr) -> Stmt {
    stmt(StmtKind::Expr(e))
}

fn print(args: Vec<Expr>) -> Stmt {
    expr(call("print", args))
}

fn tname(n: &str) -> Target {
    Target {
        kind: TargetKind::Name(n.to_string()),
        span: Z,
    }
}

fn ttuple(targets: Vec<Target>) -> Target {
    Target {
        kind: TargetKind::Tuple(targets),
        span: Z,
    }
}

fn tsub(value: Expr, index: Expr) -> Target {
    Target {
        kind: TargetKind::Subscript {
            value: Box::new(value),
            index: Box::new(index),
        },
        span: Z,
    }
}

fn assign_to(target: Target, value: Expr) -> Stmt {
    stmt(StmtKind::Assign { target, value })
}

fn assign(n: &str, value: Expr) -> Stmt {
    assign_to(tname(n), value)
}

fn aug(target: Target, op: BinOp, value: Expr) -> Stmt {
    stmt(StmtKind::AugAssign { target, op, value })
}

fn branch(test: Expr, body: Vec<Stmt>) -> Branch {
    Branch {
        test,
        body,
        span: Z,
    }
}

fn if_(test: Expr, body: Vec<Stmt>, elifs: Vec<Branch>, orelse: Vec<Stmt>) -> Stmt {
    stmt(StmtKind::If {
        branch: branch(test, body),
        elifs,
        orelse,
    })
}

fn while_(test: Expr, body: Vec<Stmt>, orelse: Vec<Stmt>) -> Stmt {
    stmt(StmtKind::While {
        branch: branch(test, body),
        orelse,
    })
}

fn for_(target: Target, iter: Expr, body: Vec<Stmt>, orelse: Vec<Stmt>) -> Stmt {
    stmt(StmtKind::For {
        target,
        iter,
        body,
        orelse,
    })
}

fn ret(value: Expr) -> Stmt {
    stmt(StmtKind::Return(Some(value)))
}

fn def(n: &str, params: Vec<(&str, Option<Expr>)>, body: Vec<Stmt>) -> Stmt {
    let params = params
        .into_iter()
        .map(|(p, default)| Param {
            mode: ParamMode::Read,
            name: ident(p),
            annotation: None,
            default,
            span: Z,
        })
        .collect();
    stmt(StmtKind::FuncDef(FuncDef {
        mode: Mode::Loose,
        name: ident(n),
        params,
        returns: None,
        raises: Vec::new(),
        body,
        span: Z,
    }))
}

fn module(body: Vec<Stmt>) -> Module {
    Module { body, span: Z }
}

// ---- runners ------------------------------------------------------------

/// Runs and returns stdout and the error, if any.
fn exec(body: Vec<Stmt>) -> (String, Option<Diagnostic>) {
    let mut out = Vec::new();
    let result = run(&module(body), &mut out);
    (String::from_utf8(out).expect("utf-8 output"), result.err())
}

fn ok(body: Vec<Stmt>) -> String {
    let (out, err) = exec(body);
    assert_eq!(err, None, "unexpected error; output so far: {out}");
    out
}

/// Runs a program that must fail; returns its error message.
fn fails(body: Vec<Stmt>) -> String {
    let (_, err) = exec(body);
    let err = err.expect("program should fail");
    assert_eq!(err.kind, DiagKind::Runtime);
    err.message
}

/// `print(e)` output, without the newline.
fn show(e: Expr) -> String {
    ok(vec![print(vec![e])]).trim_end_matches('\n').to_string()
}

fn show_err(e: Expr) -> String {
    fails(vec![print(vec![e])])
}

use BinOp::*;

// ---- printing -----------------------------------------------------------

#[test]
fn prints_scalars_as_python_does() {
    assert_eq!(show(int(-42)), "-42");
    assert_eq!(show(boolean(true)), "True");
    assert_eq!(show(none()), "None");
    assert_eq!(show(st("hi")), "hi");
    let out = ok(vec![
        print(vec![int(1), st("a"), float(2.5)]),
        print(vec![]),
    ]);
    assert_eq!(out, "1 a 2.5\n\n");
}

#[test]
fn float_repr_matches_python() {
    let cases = [
        (1.0, "1.0"),
        (0.1, "0.1"),
        (-0.0, "-0.0"),
        (1.5, "1.5"),
        (123456789.0, "123456789.0"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e+16"),
        (1.5e300, "1.5e+300"),
        (0.0001, "0.0001"),
        (0.00001, "1e-05"),
        (-1.25e-7, "-1.25e-07"),
        (f64::INFINITY, "inf"),
        (f64::NAN, "nan"),
    ];
    for (f, want) in cases {
        assert_eq!(show(float(f)), want, "repr of {f:?}");
    }
}

#[test]
fn containers_use_repr_of_elements() {
    assert_eq!(
        show(list(vec![int(1), st("a"), list(vec![])])),
        "[1, 'a', []]"
    );
    assert_eq!(show(tuple(vec![int(1)])), "(1,)");
    assert_eq!(show(tuple(vec![])), "()");
    assert_eq!(show(tuple(vec![int(1), float(2.0)])), "(1, 2.0)");
    assert_eq!(show(list(vec![st("it's")])), "[\"it's\"]");
    assert_eq!(show(list(vec![st("a\n\\'\"")])), "['a\\n\\\\\\'\"']");
    assert_eq!(show(list(vec![st("\u{7}")])), "['\\x07']");
}

#[test]
fn print_honors_sep_and_end() {
    let p = call_args(
        name("print"),
        vec![
            arg(int(1)),
            arg(int(2)),
            kwarg("sep", st("-")),
            kwarg("end", st("!")),
        ],
    );
    assert_eq!(ok(vec![expr(p)]), "1-2!");
    let p = call_args(name("print"), vec![arg(int(1)), kwarg("end", none())]);
    assert_eq!(ok(vec![expr(p)]), "1\n");
}

#[test]
fn print_rejects_bad_keywords() {
    let p = call_args(name("print"), vec![kwarg("file", int(1))]);
    assert!(fails(vec![expr(p)]).contains("'file' is an invalid keyword"));
    let p = call_args(name("print"), vec![kwarg("sep", int(1))]);
    assert!(fails(vec![expr(p)]).contains("sep must be None or a string"));
}

#[test]
fn output_before_an_error_is_kept() {
    let (out, err) = exec(vec![
        print(vec![st("before")]),
        print(vec![name("nope")]),
        print(vec![st("after")]),
    ]);
    assert_eq!(out, "before\n");
    assert!(err.is_some_and(|e| e.message.contains("NameError")));
}

#[test]
fn write_failure_is_a_runtime_error() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let err = run(&module(vec![print(vec![int(1)])]), &mut Broken).expect_err("write fails");
    assert!(err.message.contains("OSError: cannot write output: closed"));
}

// ---- arithmetic ---------------------------------------------------------

#[test]
fn division_is_true_and_floor() {
    assert_eq!(show(bin(Div, int(7), int(2))), "3.5");
    assert_eq!(show(bin(Div, int(6), int(3))), "2.0");
    assert_eq!(show(bin(FloorDiv, int(-7), int(2))), "-4");
    assert_eq!(show(bin(FloorDiv, int(7), int(-2))), "-4");
    assert_eq!(show(bin(FloorDiv, int(6), int(-2))), "-3");
    assert_eq!(show(bin(Mod, int(-7), int(3))), "2");
    assert_eq!(show(bin(Mod, int(7), int(-3))), "-2");
    assert_eq!(show(bin(FloorDiv, float(-7.5), int(2))), "-4.0");
    assert_eq!(show(bin(Mod, float(-7.5), int(2))), "0.5");
    assert_eq!(show(bin(Mod, float(7.5), float(-2.0))), "-0.5");
}

#[test]
fn mixed_and_bool_arithmetic_promotes() {
    assert_eq!(show(bin(Add, int(1), float(0.5))), "1.5");
    assert_eq!(show(bin(Add, boolean(true), int(1))), "2");
    assert_eq!(show(bin(Mul, boolean(true), float(2.5))), "2.5");
}

#[test]
fn powers() {
    assert_eq!(show(bin(Pow, int(2), int(10))), "1024");
    assert_eq!(show(bin(Pow, int(2), int(0))), "1");
    assert_eq!(show(bin(Pow, float(2.0), int(-1))), "0.5");
    assert_eq!(
        show_err(bin(Pow, int(2), int(-1))),
        "ValueError: negative exponent in int ** int; use a float base, e.g. 2.0 ** -1"
    );
    assert_eq!(show(bin(Pow, int(-1), int(i64::MAX))), "-1");
    assert_eq!(show(bin(Pow, float(4.0), float(0.5))), "2.0");
    assert!(show_err(bin(Pow, int(2), int(63))).contains("OverflowError"));
    assert!(show_err(bin(Pow, int(2), int(i64::MAX))).contains("OverflowError"));
    assert!(show_err(bin(Pow, int(0), int(-1))).contains("negative exponent"));
    assert!(show_err(bin(Pow, int(-2), int(-2))).contains("use a float base, e.g. (-2.0) ** -2"));
    assert!(show_err(bin(Pow, float(0.0), int(-1))).contains("ZeroDivisionError"));
    assert!(show_err(bin(Pow, float(10.0), float(400.0))).contains("OverflowError"));
    assert!(show_err(bin(Pow, float(-8.0), float(0.5))).contains("ValueError"));
}

#[test]
fn int_overflow_is_an_error_not_a_wrap() {
    assert!(show_err(bin(Add, int(i64::MAX), int(1))).contains("OverflowError"));
    assert!(show_err(bin(Sub, int(i64::MIN), int(1))).contains("OverflowError"));
    assert!(show_err(bin(Mul, int(i64::MAX), int(2))).contains("OverflowError"));
    assert!(show_err(bin(FloorDiv, int(i64::MIN), int(-1))).contains("OverflowError"));
    assert!(show_err(un(UnaryOp::Neg, int(i64::MIN))).contains("OverflowError"));
    assert_eq!(show(bin(Mod, int(i64::MIN), int(-1))), "0");
    assert_eq!(
        show(bin(Add, int(i64::MAX - 1), int(1))),
        i64::MAX.to_string()
    );
}

#[test]
fn division_by_zero() {
    assert!(show_err(bin(Div, int(1), int(0))).contains("ZeroDivisionError: division by zero"));
    assert!(show_err(bin(FloorDiv, int(1), int(0))).contains("ZeroDivisionError"));
    assert!(show_err(bin(Mod, int(1), int(0))).contains("ZeroDivisionError"));
    assert!(show_err(bin(Div, float(1.0), float(0.0))).contains("ZeroDivisionError"));
    assert!(show_err(bin(Mod, float(1.0), float(-0.0))).contains("ZeroDivisionError"));
}

#[test]
fn error_span_is_the_failing_expression() {
    let (_, err) = exec(vec![print(vec![at(sp(6, 11), bin(Div, int(1), int(0)))])]);
    assert_eq!(err.map(|e| e.span), Some(sp(6, 11)));
}

#[test]
fn unary_operators() {
    assert_eq!(show(un(UnaryOp::Neg, float(1.5))), "-1.5");
    assert_eq!(show(un(UnaryOp::Pos, boolean(true))), "1");
    assert_eq!(show(un(UnaryOp::Not, list(vec![]))), "True");
    assert!(show_err(un(UnaryOp::Neg, st("a"))).contains("bad operand type for unary -: 'str'"));
}

#[test]
fn sequence_concat_and_repeat() {
    assert_eq!(show(bin(Add, st("ab"), st("cd"))), "abcd");
    assert_eq!(show(bin(Mul, st("ab"), int(3))), "ababab");
    assert_eq!(show(bin(Mul, int(2), list(vec![int(0)]))), "[0, 0]");
    assert_eq!(show(bin(Mul, tuple(vec![int(1)]), int(-5))), "()");
    assert_eq!(
        show(bin(Add, tuple(vec![int(1)]), tuple(vec![int(2)]))),
        "(1, 2)"
    );
    assert!(show_err(bin(Mul, st("x"), int(i64::MAX))).contains("MemoryError"));
    assert!(show_err(bin(Add, st("a"), int(1)))
        .contains("unsupported operand type(s) for +: 'str' and 'int'"));
    assert!(show_err(bin(Mul, st("a"), st("b"))).contains("unsupported operand"));
}

// ---- logic and comparison ----------------------------------------------

/// `def f(x): print("f"); return x`, to observe evaluation.
fn tracer() -> Stmt {
    def(
        "f",
        vec![("x", None)],
        vec![print(vec![st("f")]), ret(name("x"))],
    )
}

#[test]
fn and_or_short_circuit_and_return_operands() {
    assert_eq!(show(logical(LogicalOp::Or, int(0), st("x"))), "x");
    assert_eq!(show(logical(LogicalOp::And, int(1), int(0))), "0");
    assert_eq!(show(logical(LogicalOp::And, list(vec![]), int(1))), "[]");
    let skipped = logical(LogicalOp::And, boolean(false), call("f", vec![int(1)]));
    assert_eq!(ok(vec![tracer(), print(vec![skipped])]), "False\n");
}

#[test]
fn comparison_chain_evaluates_each_operand_once() {
    let chain = cmp(
        int(1),
        vec![(CmpOp::Lt, call("f", vec![int(2)])), (CmpOp::Lt, int(3))],
    );
    assert_eq!(ok(vec![tracer(), print(vec![chain])]), "f\nTrue\n");
    let short = cmp(
        int(3),
        vec![(CmpOp::Lt, int(1)), (CmpOp::Lt, call("f", vec![int(9)]))],
    );
    assert_eq!(ok(vec![tracer(), print(vec![short])]), "False\n");
}

#[test]
fn equality_and_ordering() {
    let c = |l: Expr, op: CmpOp, r: Expr| show(cmp(l, vec![(op, r)]));
    assert_eq!(c(int(1), CmpOp::Eq, float(1.0)), "True");
    assert_eq!(c(boolean(true), CmpOp::Eq, int(1)), "True");
    assert_eq!(c(int(1), CmpOp::Eq, st("1")), "False");
    assert_eq!(c(st("a"), CmpOp::Lt, st("b")), "True");
    assert_eq!(
        c(
            list(vec![int(1), int(2)]),
            CmpOp::Lt,
            list(vec![int(1), int(3)])
        ),
        "True"
    );
    assert_eq!(
        c(
            tuple(vec![int(1), int(2)]),
            CmpOp::Lt,
            tuple(vec![int(1), int(2), int(0)])
        ),
        "True"
    );
    assert_eq!(
        c(tuple(vec![int(1)]), CmpOp::GtEq, tuple(vec![int(1)])),
        "True"
    );
    assert_eq!(c(float(f64::NAN), CmpOp::Eq, float(f64::NAN)), "False");
    assert_eq!(c(float(f64::NAN), CmpOp::LtEq, float(f64::NAN)), "False");
    assert_eq!(c(float(f64::NAN), CmpOp::NotEq, float(f64::NAN)), "True");
    assert_eq!(
        c(
            call("range", vec![int(0)]),
            CmpOp::Eq,
            call("range", vec![int(5), int(2)])
        ),
        "True"
    );
    assert!(show_err(cmp(int(1), vec![(CmpOp::Lt, st("a"))]))
        .contains("'<' not supported between instances of 'int' and 'str'"));
}

#[test]
fn membership() {
    let c = |l: Expr, op: CmpOp, r: Expr| show(cmp(l, vec![(op, r)]));
    assert_eq!(c(int(2), CmpOp::In, list(vec![int(1), int(2)])), "True");
    assert_eq!(c(st("ell"), CmpOp::In, st("hello")), "True");
    assert_eq!(
        c(
            int(9),
            CmpOp::In,
            call("range", vec![int(0), int(10), int(3)])
        ),
        "True"
    );
    assert_eq!(
        c(
            int(4),
            CmpOp::NotIn,
            call("range", vec![int(0), int(10), int(3)])
        ),
        "True"
    );
    assert_eq!(
        c(float(3.0), CmpOp::In, call("range", vec![int(5)])),
        "True"
    );
    assert!(show_err(cmp(int(1), vec![(CmpOp::In, int(5))]))
        .contains("argument of type 'int' is not iterable"));
    assert!(show_err(cmp(int(1), vec![(CmpOp::In, st("1"))]))
        .contains("requires string as left operand"));
}

#[test]
fn conditional_expression_evaluates_one_side() {
    let e = if_exp(boolean(false), call("f", vec![int(1)]), st("no"));
    assert_eq!(ok(vec![tracer(), print(vec![e])]), "no\n");
}

// ---- assignment ---------------------------------------------------------

#[test]
fn tuple_unpacking_and_swap() {
    let out = ok(vec![
        assign_to(
            ttuple(vec![tname("a"), tname("b")]),
            tuple(vec![int(1), int(2)]),
        ),
        assign_to(
            ttuple(vec![tname("a"), tname("b")]),
            tuple(vec![name("b"), name("a")]),
        ),
        print(vec![name("a"), name("b")]),
        assign_to(
            ttuple(vec![ttuple(vec![tname("x"), tname("y")]), tname("z")]),
            list(vec![st("pq"), int(3)]),
        ),
        print(vec![name("x"), name("y"), name("z")]),
    ]);
    assert_eq!(out, "2 1\np q 3\n");
}

#[test]
fn unpacking_arity_mismatch() {
    let two = || ttuple(vec![tname("a"), tname("b")]);
    let msg = fails(vec![assign_to(two(), tuple(vec![int(1), int(2), int(3)]))]);
    assert_eq!(msg, "ValueError: too many values to unpack (expected 2)");
    let msg = fails(vec![assign_to(two(), list(vec![int(1)]))]);
    assert_eq!(
        msg,
        "ValueError: not enough values to unpack (expected 2, got 1)"
    );
    assert!(fails(vec![assign_to(two(), int(1))]).contains("'int' object is not iterable"));
}

#[test]
fn subscripts() {
    let xs = || list(vec![int(10), int(20), int(30)]);
    assert_eq!(show(sub(xs(), int(-1))), "30");
    assert_eq!(show(sub(st("héllo"), int(1))), "é");
    assert_eq!(show(sub(tuple(vec![int(1), int(2)]), boolean(true))), "2");
    assert_eq!(
        show_err(sub(xs(), int(3))),
        "IndexError: list index out of range"
    );
    assert_eq!(
        show_err(sub(xs(), int(-4))),
        "IndexError: list index out of range"
    );
    assert!(show_err(sub(xs(), st("0"))).contains("list indices must be integers, not str"));
    assert!(show_err(sub(int(1), int(0))).contains("'int' object is not subscriptable"));
    let r = || call("range", vec![int(10), int(0), int(-3)]);
    assert_eq!(show(sub(r(), int(-1))), "1");
    assert_eq!(
        show_err(sub(r(), int(4))),
        "IndexError: range object index out of range"
    );
}

#[test]
fn subscript_assignment() {
    let out = ok(vec![
        assign("xs", list(vec![int(1), int(2)])),
        assign_to(tsub(name("xs"), int(-1)), int(9)),
        aug(tsub(name("xs"), int(0)), Add, int(5)),
        print(vec![name("xs")]),
    ]);
    assert_eq!(out, "[6, 9]\n");
    let msg = fails(vec![assign_to(tsub(tuple(vec![int(1)]), int(0)), int(2))]);
    assert_eq!(
        msg,
        "TypeError: 'tuple' object does not support item assignment"
    );
    let msg = fails(vec![
        assign("xs", list(vec![])),
        assign_to(tsub(name("xs"), int(0)), int(1)),
    ]);
    assert_eq!(msg, "IndexError: list index out of range");
}

#[test]
fn lists_alias_through_append_and_plus_equals() {
    let out = ok(vec![
        assign("a", list(vec![int(1)])),
        assign("b", name("a")),
        append("b", int(2)),
        aug(tname("b"), Add, list(vec![int(3)])),
        aug(tname("a"), Add, name("a")),
        print(vec![name("b")]),
        aug(tname("a"), Mul, int(2)),
        print(vec![call("len", vec![name("b")])]),
        assign("s", st("x")),
        assign("t", name("s")),
        aug(tname("t"), Add, st("y")),
        print(vec![name("s"), name("t")]),
    ]);
    assert_eq!(out, "[1, 2, 3, 1, 2, 3]\n12\nx xy\n");
}

#[test]
fn bad_attribute_and_bad_call() {
    assert!(show_err(attr(int(1), "append"))
        .contains("AttributeError: 'int' object has no attribute 'append'"));
    assert!(show_err(call_args(int(1), vec![])).contains("'int' object is not callable"));
    let two_args = call_args(attr(list(vec![]), "append"), vec![arg(int(1)), arg(int(2))]);
    assert!(show_err(two_args).contains("append() takes exactly one argument (2 given)"));
}

// ---- functions and scope ------------------------------------------------

#[test]
fn recursion_fib() {
    let fib = def(
        "fib",
        vec![("n", None)],
        vec![
            if_(
                cmp(name("n"), vec![(CmpOp::Lt, int(2))]),
                vec![ret(name("n"))],
                vec![],
                vec![],
            ),
            ret(bin(
                Add,
                call("fib", vec![bin(Sub, name("n"), int(1))]),
                call("fib", vec![bin(Sub, name("n"), int(2))]),
            )),
        ],
    );
    assert_eq!(
        ok(vec![fib, print(vec![call("fib", vec![int(20)])])]),
        "6765\n"
    );
}

#[test]
fn implicit_return_is_none() {
    assert_eq!(
        ok(vec![
            def("f", vec![], vec![stmt(StmtKind::Pass)]),
            print(vec![call("f", vec![])])
        ]),
        "None\n"
    );
}

#[test]
fn defaults_are_evaluated_once_at_def() {
    let out = ok(vec![
        assign("d", int(1)),
        def("f", vec![("x", Some(name("d")))], vec![ret(name("x"))]),
        assign("d", int(2)),
        print(vec![call("f", vec![])]),
        def(
            "g",
            vec![("acc", Some(list(vec![])))],
            vec![append("acc", int(0)), ret(name("acc"))],
        ),
        expr(call("g", vec![])),
        print(vec![call("g", vec![])]),
    ]);
    assert_eq!(out, "1\n[0, 0]\n");
}

#[test]
fn keyword_binding() {
    let f = || {
        def(
            "f",
            vec![("a", None), ("b", Some(int(2)))],
            vec![ret(tuple(vec![name("a"), name("b")]))],
        )
    };
    let call_f = |args: Vec<Arg>| call_args(name("f"), args);
    let out = ok(vec![
        f(),
        print(vec![call_f(vec![kwarg("b", int(5)), kwarg("a", int(4))])]),
        print(vec![call_f(vec![arg(int(1))])]),
    ]);
    assert_eq!(out, "(4, 5)\n(1, 2)\n");
    let err = |args: Vec<Arg>| fails(vec![f(), expr(call_f(args))]);
    assert!(err(vec![]).contains("f() missing required argument: 'a'"));
    assert!(err(vec![arg(int(1)), arg(int(2)), arg(int(3))])
        .contains("takes 2 positional arguments but 3 were given"));
    assert!(err(vec![kwarg("c", int(1))]).contains("unexpected keyword argument 'c'"));
    assert!(err(vec![arg(int(1)), kwarg("a", int(1))]).contains("multiple values for argument 'a'"));
}

#[test]
fn functions_read_globals_late() {
    let out = ok(vec![
        def("f", vec![], vec![ret(name("g"))]),
        assign("g", int(7)),
        print(vec![call("f", vec![])]),
    ]);
    assert_eq!(out, "7\n");
}

#[test]
fn assigned_names_are_local_throughout() {
    let msg = fails(vec![
        assign("x", int(1)),
        def(
            "f",
            vec![],
            vec![print(vec![name("x")]), assign("x", int(2))],
        ),
        expr(call("f", vec![])),
    ]);
    assert!(msg.starts_with("UnboundLocalError: cannot access local variable 'x'"));
    let out = ok(vec![
        assign("x", int(1)),
        def("f", vec![], vec![assign("x", int(2)), ret(name("x"))]),
        print(vec![call("f", vec![]), name("x")]),
    ]);
    assert_eq!(out, "2 1\n");
}

#[test]
fn undefined_name() {
    assert_eq!(
        show_err(name("nope")),
        "NameError: name 'nope' is not defined"
    );
}

#[test]
fn list_copies_only_lists() {
    assert_eq!(
        show_err(call("list", vec![int(1)])),
        "TypeError: 'int' object is not iterable"
    );
}

#[test]
fn recursion_limit_is_an_error() {
    let forever = def("f", vec![], vec![ret(call("f", vec![]))]);
    let msg = fails(vec![forever, expr(call("f", vec![]))]);
    assert_eq!(msg, "RecursionError: maximum recursion depth exceeded");
}

#[test]
fn nested_def_is_rejected() {
    let inner = def("g", vec![], vec![stmt(StmtKind::Pass)]);
    let msg = fails(vec![def("f", vec![], vec![inner]), expr(call("f", vec![]))]);
    assert!(msg.contains("only allowed at module top level"));
}

#[test]
fn stray_control_flow_is_an_error() {
    assert!(fails(vec![stmt(StmtKind::Break)]).contains("'break' outside loop"));
    assert!(fails(vec![stmt(StmtKind::Continue)]).contains("'continue' not properly in loop"));
    assert!(fails(vec![ret(int(1))]).contains("'return' outside function"));
    let f = def("f", vec![], vec![stmt(StmtKind::Break)]);
    assert!(fails(vec![f, expr(call("f", vec![]))]).contains("'break' outside loop"));
}

// ---- control flow -------------------------------------------------------

#[test]
fn if_elif_else() {
    let classify = |n: i64| {
        ok(vec![
            assign("n", int(n)),
            if_(
                cmp(name("n"), vec![(CmpOp::Lt, int(0))]),
                vec![print(vec![st("neg")])],
                vec![branch(
                    cmp(name("n"), vec![(CmpOp::Eq, int(0))]),
                    vec![print(vec![st("zero")])],
                )],
                vec![print(vec![st("pos")])],
            ),
        ])
    };
    assert_eq!(classify(-1), "neg\n");
    assert_eq!(classify(0), "zero\n");
    assert_eq!(classify(1), "pos\n");
}

#[test]
fn while_with_break_continue_and_else() {
    let lt = |n: &str, k: i64| cmp(name(n), vec![(CmpOp::Lt, int(k))]);
    let out = ok(vec![
        assign("i", int(0)),
        while_(
            lt("i", 10),
            vec![
                aug(tname("i"), Add, int(1)),
                if_(
                    cmp(bin(Mod, name("i"), int(2)), vec![(CmpOp::Eq, int(0))]),
                    vec![stmt(StmtKind::Continue)],
                    vec![],
                    vec![],
                ),
                if_(
                    cmp(name("i"), vec![(CmpOp::Gt, int(6))]),
                    vec![stmt(StmtKind::Break)],
                    vec![],
                    vec![],
                ),
                print(vec![name("i")]),
            ],
            vec![print(vec![st("else")])],
        ),
        while_(boolean(false), vec![], vec![print(vec![st("ran else")])]),
    ]);
    assert_eq!(out, "1\n3\n5\nran else\n");
}

#[test]
fn for_over_each_iterable_kind() {
    let out = ok(vec![
        for_(tname("c"), st("ab"), vec![print(vec![name("c")])], vec![]),
        for_(
            tname("i"),
            call("range", vec![int(5), int(0), int(-2)]),
            vec![print(vec![name("i")])],
            vec![print(vec![st("done")])],
        ),
        for_(
            ttuple(vec![tname("k"), tname("v")]),
            list(vec![tuple(vec![int(1), st("x")])]),
            vec![print(vec![name("k"), name("v")])],
            vec![],
        ),
    ]);
    assert_eq!(out, "a\nb\n5\n3\n1\ndone\n1 x\n");
}

#[test]
fn for_break_skips_else() {
    let out = ok(vec![for_(
        tname("i"),
        call("range", vec![int(3)]),
        vec![stmt(StmtKind::Break)],
        vec![print(vec![st("else")])],
    )]);
    assert_eq!(out, "");
}

#[test]
fn appending_while_iterating_sees_new_items() {
    let out = ok(vec![
        assign("xs", list(vec![int(1)])),
        for_(
            tname("x"),
            name("xs"),
            vec![
                print(vec![name("x")]),
                if_(
                    cmp(name("x"), vec![(CmpOp::Lt, int(3))]),
                    vec![append("xs", bin(Add, name("x"), int(1)))],
                    vec![],
                    vec![],
                ),
            ],
            vec![],
        ),
    ]);
    assert_eq!(out, "1\n2\n3\n");
}

#[test]
fn range_at_i64_limits_terminates() {
    let r = call("range", vec![int(i64::MAX - 1), int(i64::MAX)]);
    assert_eq!(
        ok(vec![for_(
            tname("i"),
            r,
            vec![print(vec![name("i")])],
            vec![]
        )]),
        format!("{}\n", i64::MAX - 1)
    );
    let whole = call("range", vec![int(i64::MIN), int(i64::MAX)]);
    assert!(show_err(call("len", vec![whole])).contains("OverflowError"));
}

#[test]
fn fizzbuzz() {
    let divisible = |k: i64| cmp(bin(Mod, name("i"), int(k)), vec![(CmpOp::Eq, int(0))]);
    let program = vec![for_(
        tname("i"),
        call("range", vec![int(1), int(16)]),
        vec![if_(
            divisible(15),
            vec![print(vec![st("FizzBuzz")])],
            vec![
                branch(divisible(3), vec![print(vec![st("Fizz")])]),
                branch(divisible(5), vec![print(vec![st("Buzz")])]),
            ],
            vec![print(vec![name("i")])],
        )],
        vec![],
    )];
    let want = "1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz\n";
    assert_eq!(ok(program), want);
}

// ---- builtins -----------------------------------------------------------

#[test]
fn len_and_range() {
    assert_eq!(show(call("len", vec![st("héllo")])), "5");
    assert_eq!(show(call("len", vec![list(vec![int(1), int(2)])])), "2");
    assert_eq!(
        show(call(
            "len",
            vec![call("range", vec![int(0), int(10), int(3)])]
        )),
        "4"
    );
    assert_eq!(
        show(call("len", vec![call("range", vec![int(5), int(0)])])),
        "0"
    );
    assert_eq!(show(call("range", vec![int(3)])), "range(0, 3)");
    assert_eq!(
        show(call("range", vec![int(1), int(9), int(2)])),
        "range(1, 9, 2)"
    );
    assert!(show_err(call("len", vec![int(1)])).contains("object of type 'int' has no len()"));
    assert!(show_err(call("len", vec![])).contains("len() takes exactly 1 arguments (0 given)"));
    assert!(show_err(call("range", vec![int(1), int(2), int(0)])).contains("must not be zero"));
    assert!(show_err(call("range", vec![float(1.0)]))
        .contains("'float' object cannot be interpreted as an integer"));
}

#[test]
fn conversions() {
    assert_eq!(show(call("int", vec![st("  -42 ")])), "-42");
    assert_eq!(show(call("int", vec![st("1_000")])), "1000");
    assert_eq!(show(call("int", vec![float(-3.9)])), "-3");
    assert_eq!(show(call("int", vec![boolean(true)])), "1");
    assert_eq!(show(call("int", vec![])), "0");
    assert_eq!(show(call("float", vec![st("1e3")])), "1000.0");
    assert_eq!(show(call("float", vec![int(2)])), "2.0");
    assert_eq!(show(call("float", vec![st("-inf")])), "-inf");
    assert_eq!(show(call("str", vec![float(1.5)])), "1.5");
    assert_eq!(show(list(vec![call("str", vec![int(1)])])), "['1']");
    assert_eq!(show(call("bool", vec![list(vec![])])), "False");
    assert_eq!(show(call("bool", vec![st("0")])), "True");
    assert_eq!(show(call("bool", vec![])), "False");
}

#[test]
fn conversion_failures() {
    let err = |f: &str, v: Expr| show_err(call(f, vec![v]));
    assert_eq!(
        err("int", st("abc")),
        "ValueError: invalid literal for int() with base 10: 'abc'"
    );
    assert!(err("int", st("1__0")).contains("invalid literal"));
    assert!(err("int", st("_1")).contains("invalid literal"));
    assert!(err("int", st("")).contains("invalid literal"));
    assert!(err("int", st("99999999999999999999")).contains("OverflowError"));
    assert!(err("int", float(f64::NAN)).contains("cannot convert float NaN"));
    assert!(err("int", float(9.3e18)).contains("OverflowError"));
    assert!(err("int", list(vec![]))
        .contains("int() argument must be a string or a number, not 'list'"));
    assert!(err("float", st("x")).contains("could not convert string to float: 'x'"));
    let kw = call_args(name("len"), vec![kwarg("x", int(1))]);
    assert!(show_err(kw).contains("len() takes no keyword arguments"));
}

#[test]
fn builtins_can_be_shadowed() {
    let out = ok(vec![assign("len", int(3)), print(vec![name("len")])]);
    assert_eq!(out, "3\n");
}

// ---- review fixes: bounds and cycles ------------------------------------

#[test]
fn unpacking_a_huge_range_stops_early() {
    let two = || ttuple(vec![tname("a"), tname("b")]);
    let huge = call("range", vec![int(i64::MAX)]);
    let msg = fails(vec![assign_to(two(), huge)]);
    assert_eq!(msg, "ValueError: too many values to unpack (expected 2)");
    let nested = ttuple(vec![two(), tname("c")]);
    let value = list(vec![call("range", vec![int(i64::MAX)]), int(1)]);
    assert_eq!(fails(vec![assign_to(nested, value)]), msg);
    let exact = assign_to(two(), call("range", vec![int(2)]));
    assert_eq!(ok(vec![exact, print(vec![name("a"), name("b")])]), "0 1\n");
}

/// `xs = []; xs.append(xs)`
fn self_list(n: &str) -> Vec<Stmt> {
    vec![assign(n, list(vec![])), append(n, name(n))]
}

#[test]
fn cyclic_lists_render_as_python_does() {
    let mut program = self_list("xs");
    program.push(print(vec![name("xs"), tuple(vec![name("xs")])]));
    program.extend([
        assign("a", list(vec![])),
        assign("b", list(vec![name("a")])),
        append("a", name("b")),
        print(vec![name("a"), call("str", vec![name("b")])]),
    ]);
    assert_eq!(ok(program), "[[...]] ([[...]],)\n[[[...]]] [[[...]]]\n");
    let tuple_cycle = vec![
        assign("a", list(vec![])),
        assign("t", tuple(vec![name("a")])),
        append("a", name("t")),
        print(vec![name("t"), name("a")]),
    ];
    assert_eq!(ok(tuple_cycle), "([(...)],) [([...],)]\n");
}

#[test]
fn cyclic_lists_compare_by_identity_first() {
    let mut program = self_list("xs");
    program.push(print(vec![
        cmp(name("xs"), vec![(CmpOp::Eq, name("xs"))]),
        cmp(name("xs"), vec![(CmpOp::In, name("xs"))]),
        cmp(name("xs"), vec![(CmpOp::Lt, name("xs"))]),
    ]));
    assert_eq!(ok(program), "True True False\n");
    let mut distinct = self_list("p");
    distinct.extend(self_list("q"));
    distinct.push(print(vec![cmp(name("p"), vec![(CmpOp::Eq, name("q"))])]));
    assert_eq!(
        fails(distinct),
        "RecursionError: maximum recursion depth exceeded in comparison"
    );
}

#[test]
fn deep_nesting_is_a_recursion_error() {
    // x = []; i = 0; while i < 20000: x = [x]; i += 1
    let mut program = vec![
        assign("x", list(vec![])),
        assign("y", list(vec![])),
        assign("i", int(0)),
        while_(
            cmp(name("i"), vec![(CmpOp::Lt, int(20_000))]),
            vec![
                assign("x", list(vec![name("x")])),
                assign("y", list(vec![name("y")])),
                aug(tname("i"), Add, int(1)),
            ],
            vec![],
        ),
    ];
    let compare = print(vec![cmp(name("x"), vec![(CmpOp::Eq, name("y"))])]);
    assert!(fails([program.clone(), vec![compare]].concat()).contains("in comparison"));
    program.push(print(vec![name("x")]));
    assert!(fails(program).contains("while getting the repr of an object"));
}

#[test]
fn repeating_empty_sequences_is_immediate() {
    assert_eq!(show(tuple(vec![bin(Mul, st(""), int(i64::MAX))])), "('',)");
    assert_eq!(show(bin(Mul, list(vec![]), int(i64::MAX))), "[]");
    assert_eq!(show(bin(Mul, int(i64::MAX), tuple(vec![]))), "()");
}

#[test]
fn is_compares_only_with_none() {
    let c = |l: Expr, op: CmpOp, r: Expr| show(cmp(l, vec![(op, r)]));
    assert_eq!(c(none(), CmpOp::Is, none()), "True");
    assert_eq!(c(int(1), CmpOp::Is, none()), "False");
    assert_eq!(c(none(), CmpOp::IsNot, int(1)), "True");
    assert!(
        show_err(cmp(int(1), vec![(CmpOp::Is, int(1))])).contains("`is` compares only with None")
    );
}

// ---- exceptions (D6, D7) --------------------------------------------------

/// Lexes, parses and runs `src` (unchecked, as `run` takes any AST).
fn exec_src(src: &str) -> (String, Option<Diagnostic>) {
    let tokens = crate::lexer::lex(src).expect("lexes");
    let parsed = crate::parser::parse(&tokens).expect("parses");
    let mut out = Vec::new();
    let result = run(&parsed, &mut out);
    (String::from_utf8(out).expect("utf-8 output"), result.err())
}

fn ok_src(src: &str) -> String {
    let (out, err) = exec_src(src);
    assert_eq!(err, None, "unexpected error; output so far: {out}");
    out
}

/// The runtime error `src` stops with.
fn fails_src(src: &str) -> String {
    let (_, err) = exec_src(src);
    let err = err.expect("program should fail");
    assert_eq!(err.kind, DiagKind::Runtime);
    err.message
}

#[test]
fn raise_is_caught_by_the_first_matching_handler_across_calls() {
    let src =
        "def check(n):\n    if n < 0:\n        raise ValueError(\"negative\")\n    return n\n\
               try:\n    check(1)\n    check(-1)\n    print(\"unreachable\")\n\
               except IndexError:\n    print(\"index\")\n\
               except ValueError as e:\n    print(\"caught\", e)\n\
               except ValueError:\n    print(\"second\")\n";
    assert_eq!(ok_src(src), "caught negative\n");
}

#[test]
fn an_unmatched_exception_reaches_an_outer_try() {
    let src = "try:\n    try:\n        raise IndexError(3)\n    except ValueError:\n        print(\"inner\")\n    print(\"skipped\")\n\
               except IndexError as e:\n    print(\"outer\", e)\n";
    assert_eq!(ok_src(src), "outer 3\n");
}

#[test]
fn uncaught_exceptions_report_as_cpython_does() {
    assert_eq!(fails_src("raise ValueError(\"bad\")\n"), "ValueError: bad");
    assert_eq!(fails_src("raise IndexError(7)\n"), "IndexError: 7");
    assert_eq!(
        fails_src("raise ZeroDivisionError()\n"),
        "ZeroDivisionError"
    );
    assert_eq!(fails_src("raise OverflowError\n"), "OverflowError");
    assert_eq!(fails_src("raise RecursionError(\"\")\n"), "RecursionError");
    let reraise = "try:\n    raise ValueError(\"a\")\nexcept ValueError as e:\n    raise e\n";
    assert_eq!(fails_src(reraise), "ValueError: a");
}

#[test]
fn uncaught_exception_points_at_its_raise() {
    let src = "def f():\n    raise ValueError(\"x\")\nf()\n";
    let (_, err) = exec_src(src);
    let span = err.expect("fails").span;
    assert_eq!(&src[span.start..span.end], "raise ValueError(\"x\")");
}

#[test]
fn parse_builtins_raise_catchable_value_errors() {
    let src = "for s in [\"12\", \"x\", \"1.5\"]:\n    try:\n        print(int(s))\n    except ValueError as e:\n        print(e)\n\
               try:\n    float(\"nope\")\nexcept ValueError as e:\n    print(e)\n";
    let expected = "12\ninvalid literal for int() with base 10: 'x'\n\
                    invalid literal for int() with base 10: '1.5'\n\
                    could not convert string to float: 'nope'\n";
    assert_eq!(ok_src(src), expected);
    let uncaught = fails_src("int(\"x\")\n");
    assert_eq!(
        uncaught,
        "ValueError: invalid literal for int() with base 10: 'x'"
    );
}

#[test]
fn faults_are_never_caught() {
    let faults = [
        ("[1][5]", "IndexError: list index out of range"),
        ("1 // 0", "ZeroDivisionError"),
        ("9223372036854775807 + 1", "OverflowError"),
        ("int(\"99999999999999999999\")", "OverflowError"),
        (
            "int(float(\"nan\"))",
            "ValueError: cannot convert float NaN",
        ),
    ];
    for (expr, msg) in faults {
        let kind = msg.split(':').next().unwrap_or(msg);
        let src = format!("try:\n    print({expr})\nexcept {kind}:\n    print(\"caught\")\n");
        assert!(fails_src(&src).starts_with(msg), "{expr}");
    }
    let unpack = "try:\n    a, b = [1]\nexcept ValueError:\n    print(\"caught\")\n";
    assert!(fails_src(unpack).starts_with("ValueError: not enough values"));
    let deep = "def f(n):\n    return f(n + 1)\ntry:\n    f(0)\nexcept RecursionError:\n    print(\"caught\")\n";
    assert!(fails_src(deep).starts_with("RecursionError"));
}

#[test]
fn handler_unbinds_its_name_afterwards() {
    let module = "try:\n    raise ValueError(1)\nexcept ValueError as e:\n    print(e)\nprint(e)\n";
    assert_eq!(fails_src(module), "NameError: name 'e' is not defined");
    let local = "def f():\n    try:\n        raise ValueError(1)\n    except ValueError as e:\n        pass\n    return e\nf()\n";
    assert!(fails_src(local).starts_with("UnboundLocalError"));
}

#[test]
fn flow_passes_through_try() {
    let src = "def f():\n    try:\n        return 1\n    except ValueError:\n        return 2\n\
               for i in range(3):\n    try:\n        if i == 1:\n            continue\n        if i == 2:\n            break\n        print(i)\n    except ValueError:\n        pass\n\
               print(f())\n";
    assert_eq!(ok_src(src), "0\n1\n");
}

#[test]
fn exception_values_print_as_python_does() {
    let src = "e = ValueError(\"bad\")\nprint(e, str(e), [e], (IndexError(),), e == e, ValueError(\"bad\") == e)\nprint(ValueError)\n";
    let expected = "bad bad [ValueError('bad')] (IndexError(),) True False\n<class 'ValueError'>\n";
    assert_eq!(ok_src(src), expected);
}

#[test]
fn raising_or_catching_a_non_exception_is_a_type_error() {
    let raise = fails_src("raise 5\n");
    assert_eq!(
        raise,
        "TypeError: exceptions must derive from BaseException"
    );
    let shadowed = "ValueError = 1\ntry:\n    raise IndexError()\nexcept ValueError:\n    pass\n";
    assert!(fails_src(shadowed).starts_with("TypeError: catching classes"));
}

#[test]
fn nested_exception_arguments_are_bounded() {
    let chain = "e = ValueError(\"leaf\")\nfor i in range(10001):\n    e = ValueError(e)\n";
    assert_eq!(ok_src(&format!("{chain}print(e, str(e))\n")), "leaf leaf\n");
    assert_eq!(fails_src(&format!("{chain}raise e\n")), "ValueError: leaf");
    let repr = fails_src(&format!("{chain}print([e])\n"));
    assert!(repr.starts_with("RecursionError"), "{repr}");
    let shallow = "print([ValueError(IndexError([1]))], str(ValueError(IndexError(2))))\n";
    assert_eq!(ok_src(shallow), "[ValueError(IndexError([1]))] 2\n");
}

#[test]
fn list_copy_composes_with_exceptions() {
    let src = "def parse_all(xs: list[str]) -> list[int]:\n    out = []\n    for s in list(xs):\n        try:\n            out.append(int(s))\n        except ValueError as e:\n            xs.append(\"seen\")\n            print(e)\n    return list(out)\n\
               words = [\"1\", \"x\"]\nprint(parse_all(words), words)\n\
               try:\n    list(ValueError(1))\nexcept ValueError:\n    print(\"caught\")\n";
    let (out, err) = exec_src(src);
    assert_eq!(
        out,
        "invalid literal for int() with base 10: 'x'\n[1] ['1', 'x', 'seen']\n"
    );
    let err = err.expect("list() of a non-list is a fault");
    assert!(err.message.starts_with("TypeError"), "{}", err.message);
}
