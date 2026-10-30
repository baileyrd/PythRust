//! Inference tests on hand-built ASTs. Each builder call takes a fresh
//! span, so every node is distinct, as it is in parsed source.

use std::cell::Cell;

use crate::ast::*;
use crate::span::Span;

use super::{infer, Type, TypeVar, Types};

#[derive(Default)]
struct B {
    next: Cell<usize>,
}

impl B {
    fn sp(&self) -> Span {
        let n = self.next.get();
        self.next.set(n + 1);
        Span {
            start: n * 10,
            end: n * 10 + 5,
        }
    }

    fn id(&self, name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: self.sp(),
        }
    }

    // Expressions.

    fn e(&self, kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: self.sp(),
        }
    }
    fn n(&self, name: &str) -> Expr {
        self.e(ExprKind::Name(name.to_string()))
    }
    fn i(&self, v: i64) -> Expr {
        self.e(ExprKind::Int(v))
    }
    fn fl(&self, v: f64) -> Expr {
        self.e(ExprKind::Float(v))
    }
    fn s(&self, v: &str) -> Expr {
        self.e(ExprKind::Str(v.to_string()))
    }
    fn bl(&self, v: bool) -> Expr {
        self.e(ExprKind::Bool(v))
    }
    fn bin(&self, op: BinOp, l: Expr, r: Expr) -> Expr {
        self.e(ExprKind::Binary {
            op,
            left: Box::new(l),
            right: Box::new(r),
        })
    }
    fn neg(&self, x: Expr) -> Expr {
        self.e(ExprKind::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(x),
        })
    }
    fn logic(&self, op: LogicalOp, l: Expr, r: Expr) -> Expr {
        self.e(ExprKind::Logical {
            op,
            left: Box::new(l),
            right: Box::new(r),
        })
    }
    fn cmp(&self, l: Expr, rest: Vec<(CmpOp, Expr)>) -> Expr {
        self.e(ExprKind::Compare {
            left: Box::new(l),
            rest,
        })
    }
    fn ifexp(&self, test: Expr, body: Expr, orelse: Expr) -> Expr {
        self.e(ExprKind::IfExp {
            test: Box::new(test),
            body: Box::new(body),
            orelse: Box::new(orelse),
        })
    }
    fn list(&self, items: Vec<Expr>) -> Expr {
        self.e(ExprKind::List(items))
    }
    fn tuple(&self, items: Vec<Expr>) -> Expr {
        self.e(ExprKind::Tuple(items))
    }
    fn idx(&self, v: Expr, i: Expr) -> Expr {
        self.e(ExprKind::Subscript {
            value: Box::new(v),
            index: Box::new(i),
        })
    }
    fn arg(&self, value: Expr) -> Arg {
        Arg {
            name: None,
            value,
            span: self.sp(),
        }
    }
    fn kw(&self, name: &str, value: Expr) -> Arg {
        Arg {
            name: Some(self.id(name)),
            value,
            span: self.sp(),
        }
    }
    fn call(&self, name: &str, args: Vec<Expr>) -> Expr {
        let args = args.into_iter().map(|a| self.arg(a)).collect();
        self.call_args(name, args)
    }
    fn call_args(&self, name: &str, args: Vec<Arg>) -> Expr {
        let func = Box::new(self.n(name));
        self.e(ExprKind::Call { func, args })
    }
    fn method(&self, value: Expr, name: &str, args: Vec<Expr>) -> Expr {
        let func = self.e(ExprKind::Attr {
            value: Box::new(value),
            attr: self.id(name),
        });
        let args = args.into_iter().map(|a| self.arg(a)).collect();
        self.e(ExprKind::Call {
            func: Box::new(func),
            args,
        })
    }

    // Targets and statements.

    fn st(&self, kind: StmtKind) -> Stmt {
        Stmt {
            kind,
            span: self.sp(),
        }
    }
    fn t(&self, name: &str) -> Target {
        Target {
            kind: TargetKind::Name(name.to_string()),
            span: self.sp(),
        }
    }
    fn ttuple(&self, items: Vec<Target>) -> Target {
        Target {
            kind: TargetKind::Tuple(items),
            span: self.sp(),
        }
    }
    fn tidx(&self, v: Expr, i: Expr) -> Target {
        Target {
            kind: TargetKind::Subscript {
                value: Box::new(v),
                index: Box::new(i),
            },
            span: self.sp(),
        }
    }
    fn assign(&self, name: &str, value: Expr) -> Stmt {
        self.assign_to(self.t(name), value)
    }
    fn assign_to(&self, target: Target, value: Expr) -> Stmt {
        self.st(StmtKind::Assign { target, value })
    }
    fn ann(&self, name: &str, ty: TypeExpr, value: Expr) -> Stmt {
        self.st(StmtKind::AnnAssign {
            target: self.id(name),
            annotation: ty,
            value,
        })
    }
    fn aug(&self, name: &str, op: BinOp, value: Expr) -> Stmt {
        self.st(StmtKind::AugAssign {
            target: self.t(name),
            op,
            value,
        })
    }
    fn ret(&self, value: Expr) -> Stmt {
        self.st(StmtKind::Return(Some(value)))
    }
    fn ex(&self, e: Expr) -> Stmt {
        self.st(StmtKind::Expr(e))
    }
    fn branch(&self, test: Expr, body: Block) -> Branch {
        Branch {
            test,
            body,
            span: self.sp(),
        }
    }
    fn if_(&self, test: Expr, body: Block, orelse: Block) -> Stmt {
        self.st(StmtKind::If {
            branch: self.branch(test, body),
            elifs: vec![],
            orelse,
        })
    }
    fn while_(&self, test: Expr, body: Block) -> Stmt {
        self.st(StmtKind::While {
            branch: self.branch(test, body),
            orelse: vec![],
        })
    }
    fn for_(&self, target: Target, iter: Expr, body: Block) -> Stmt {
        self.st(StmtKind::For {
            target,
            iter,
            body,
            orelse: vec![],
        })
    }
    fn ty(&self, name: &str, args: Vec<TypeExpr>) -> TypeExpr {
        TypeExpr {
            name: self.id(name),
            args,
            optional: false,
            span: self.sp(),
        }
    }
    fn opt(&self, name: &str, args: Vec<TypeExpr>) -> TypeExpr {
        TypeExpr {
            optional: true,
            ..self.ty(name, args)
        }
    }
    fn param(&self, name: &str, annotation: Option<TypeExpr>, default: Option<Expr>) -> Param {
        Param {
            mode: ParamMode::Read,
            name: self.id(name),
            annotation,
            default,
            span: self.sp(),
        }
    }
    fn def(&self, name: &str, params: &[&str], body: Block) -> Stmt {
        let params = params.iter().map(|p| self.param(p, None, None)).collect();
        self.def_full(name, params, None, body)
    }
    fn def_full(
        &self,
        name: &str,
        params: Vec<Param>,
        returns: Option<TypeExpr>,
        body: Block,
    ) -> Stmt {
        self.st(StmtKind::FuncDef(FuncDef {
            mode: Mode::Loose,
            name: self.id(name),
            params,
            returns,
            raises: Vec::new(),
            body,
            span: self.sp(),
        }))
    }
}

fn module(body: Vec<Stmt>) -> Module {
    Module {
        body,
        span: Span { start: 0, end: 0 },
    }
}

fn ok(body: Vec<Stmt>) -> Types {
    match infer(&module(body)) {
        Ok(types) => types,
        Err(d) => panic!("unexpected error: {d}"),
    }
}

fn err(body: Vec<Stmt>) -> String {
    match infer(&module(body)) {
        Ok(_) => panic!("expected a type error"),
        Err(d) => d.message,
    }
}

fn list_of(t: Type) -> Type {
    Type::List(Box::new(t))
}

/// `x = <value>` at module level, returning the type of `x`.
fn type_of_value(b: &B, value: Expr) -> Type {
    let target = b.t("x");
    let span = target.span;
    let types = ok(vec![b.assign_to(target, value)]);
    types.at(span).cloned().unwrap_or(Type::None)
}

// Arithmetic and operators.

#[test]
fn arithmetic_follows_the_numeric_tower() {
    let b = B::default();
    let e = b.bin(BinOp::Add, b.i(1), b.fl(2.5));
    assert_eq!(type_of_value(&b, e), Type::Float);
    let e = b.bin(BinOp::Div, b.i(1), b.i(2));
    assert_eq!(type_of_value(&b, e), Type::Float);
    let e = b.bin(BinOp::FloorDiv, b.i(7), b.i(2));
    assert_eq!(type_of_value(&b, e), Type::Int);
    let e = b.bin(BinOp::Pow, b.i(2), b.i(10));
    assert_eq!(type_of_value(&b, e), Type::Int);
    let e = b.bin(BinOp::Add, b.bl(true), b.i(1));
    assert_eq!(type_of_value(&b, e), Type::Int);
}

#[test]
fn sequences_concatenate_and_repeat() {
    let b = B::default();
    let e = b.bin(BinOp::Mul, b.s("ab"), b.i(3));
    assert_eq!(type_of_value(&b, e), Type::Str);
    let e = b.bin(BinOp::Mul, b.i(3), b.list(vec![b.i(1)]));
    assert_eq!(type_of_value(&b, e), list_of(Type::Int));
    let e = b.bin(BinOp::Add, b.tuple(vec![b.i(1)]), b.tuple(vec![b.s("a")]));
    assert_eq!(
        type_of_value(&b, e),
        Type::Tuple(vec![Type::Int, Type::Str])
    );
}

#[test]
fn str_plus_int_is_an_error() {
    let b = B::default();
    let e = b.bin(BinOp::Add, b.s("a"), b.i(1));
    let msg = err(vec![b.assign("x", e)]);
    assert_eq!(msg, "unsupported operand types for +: str and int");
}

#[test]
fn unary_minus_on_str_is_an_error() {
    let b = B::default();
    let msg = err(vec![b.ex(b.neg(b.s("a")))]);
    assert_eq!(msg, "bad operand type for unary -: str");
}

#[test]
fn logical_operands_share_one_type() {
    let b = B::default();
    let e = b.logic(LogicalOp::Or, b.i(0), b.i(5));
    assert_eq!(type_of_value(&b, e), Type::Int);
    let e = b.logic(LogicalOp::Or, b.i(0), b.s("none"));
    let msg = err(vec![b.ex(e)]);
    assert!(
        msg.starts_with("both sides of `or` must share one type"),
        "{msg}"
    );
}

#[test]
fn conditional_branches_share_one_type() {
    let b = B::default();
    let e = b.ifexp(b.bl(true), b.i(1), b.i(2));
    assert_eq!(type_of_value(&b, e), Type::Int);
    let e = b.ifexp(b.bl(true), b.i(1), b.s("a"));
    assert!(err(vec![b.ex(e)]).starts_with("both branches"));
}

#[test]
fn chained_and_mixed_numeric_comparisons() {
    let b = B::default();
    let e = b.cmp(b.i(1), vec![(CmpOp::Lt, b.fl(2.0)), (CmpOp::LtEq, b.i(3))]);
    assert_eq!(type_of_value(&b, e), Type::Bool);
    let e = b.cmp(b.s("a"), vec![(CmpOp::Lt, b.i(1))]);
    assert_eq!(err(vec![b.ex(e)]), "cannot order str against int");
}

#[test]
fn membership_checks_element_type() {
    let b = B::default();
    let e = b.cmp(b.i(1), vec![(CmpOp::In, b.list(vec![b.i(1), b.i(2)]))]);
    assert_eq!(type_of_value(&b, e), Type::Bool);
    let e = b.cmp(b.s("a"), vec![(CmpOp::NotIn, b.list(vec![b.i(1)]))]);
    let msg = err(vec![b.ex(e)]);
    assert_eq!(msg, "cannot compare str with int: they are never equal");
}

// Variables and lists.

#[test]
fn a_variable_keeps_one_type() {
    let b = B::default();
    let msg = err(vec![b.assign("x", b.i(1)), b.assign("x", b.s("a"))]);
    assert_eq!(
        msg,
        "cannot change the type of `x`: expected int, found str"
    );
}

#[test]
fn int_accumulator_cannot_widen_to_float() {
    let b = B::default();
    let body = vec![
        b.assign("total", b.i(0)),
        b.aug("total", BinOp::Add, b.fl(0.5)),
    ];
    assert_eq!(
        err(body),
        "`total` is int here, but `+=` makes it float; start `total` as a float, such as `0.0`"
    );
}

#[test]
fn lists_are_homogeneous() {
    let b = B::default();
    let msg = err(vec![b.ex(b.list(vec![b.i(1), b.s("a")]))]);
    assert_eq!(
        msg,
        "list elements must share one type: expected int, found str"
    );
}

#[test]
fn append_fills_an_empty_list() {
    let b = B::default();
    let target = b.t("xs");
    let span = target.span;
    let types = ok(vec![
        b.assign_to(target, b.list(vec![])),
        b.ex(b.method(b.n("xs"), "append", vec![b.i(1)])),
    ]);
    assert_eq!(types.at(span), Some(&list_of(Type::Int)));
}

#[test]
fn appending_a_list_to_itself_is_infinite() {
    let b = B::default();
    let msg = err(vec![
        b.assign("xs", b.list(vec![])),
        b.ex(b.method(b.n("xs"), "append", vec![b.n("xs")])),
    ]);
    assert!(msg.contains("contain itself"), "{msg}");
}

#[test]
fn unknown_method_is_an_error() {
    let b = B::default();
    let msg = err(vec![
        b.assign("xs", b.list(vec![])),
        b.ex(b.method(b.n("xs"), "pop", vec![])),
    ]);
    assert!(msg.starts_with("unknown method `.pop`"), "{msg}");
}

#[test]
fn operations_on_never_built_values_are_accepted() {
    let b = B::default();
    let sum = b.bin(
        BinOp::Add,
        b.idx(b.n("xs"), b.i(0)),
        b.idx(b.n("xs"), b.i(1)),
    );
    ok(vec![b.assign("xs", b.list(vec![])), b.ex(sum)]);
}

#[test]
fn generic_sort_accepts_an_empty_list() {
    let b = B::default();
    let body = vec![
        b.if_(
            b.cmp(
                b.idx(b.n("xs"), b.i(0)),
                vec![(CmpOp::Gt, b.idx(b.n("xs"), b.i(1)))],
            ),
            vec![b.st(StmtKind::Pass)],
            vec![],
        ),
        b.ret(b.n("xs")),
    ];
    ok(vec![
        b.def("sort", &["xs"], body),
        b.ex(b.call("sort", vec![b.list(vec![])])),
        b.ex(b.call("sort", vec![b.list(vec![b.i(1)])])),
    ]);
}

#[test]
fn subscript_assignment_checks_element_type() {
    let b = B::default();
    let msg = err(vec![
        b.assign("xs", b.list(vec![b.i(1)])),
        b.assign_to(b.tidx(b.n("xs"), b.i(0)), b.s("a")),
    ]);
    assert_eq!(msg, "cannot assign str into list[int]");
}

// Tuples.

#[test]
fn tuples_unpack_positionally() {
    let b = B::default();
    let (ta, tb) = (b.t("a"), b.t("b"));
    let (sa, sb) = (ta.span, tb.span);
    let types =
        ok(vec![b.assign_to(
            b.ttuple(vec![ta, tb]),
            b.tuple(vec![b.i(1), b.s("x")]),
        )]);
    assert_eq!(types.at(sa), Some(&Type::Int));
    assert_eq!(types.at(sb), Some(&Type::Str));
}

#[test]
fn unpacking_arity_mismatch_is_an_error() {
    let b = B::default();
    let target = b.ttuple(vec![b.t("a"), b.t("b"), b.t("c")]);
    let msg = err(vec![b.assign_to(target, b.tuple(vec![b.i(1), b.i(2)]))]);
    assert_eq!(msg, "cannot unpack 2 values into 3 targets");
}

#[test]
fn tuple_index_must_be_an_in_range_literal() {
    let b = B::default();
    let pair = || b.tuple(vec![b.i(1), b.s("x")]);
    assert_eq!(type_of_value(&b, b.idx(pair(), b.i(1))), Type::Str);
    assert_eq!(type_of_value(&b, b.idx(pair(), b.neg(b.i(2)))), Type::Int);
    let msg = err(vec![b.ex(b.idx(pair(), b.i(2)))]);
    assert_eq!(msg, "tuple index 2 out of range for 2 elements");
    let msg = err(vec![b.assign("i", b.i(0)), b.ex(b.idx(pair(), b.n("i")))]);
    assert_eq!(msg, "a tuple index must be an integer literal");
}

#[test]
fn tuple_item_assignment_is_an_error() {
    let b = B::default();
    let msg = err(vec![
        b.assign("t", b.tuple(vec![b.i(1)])),
        b.assign_to(b.tidx(b.n("t"), b.i(0)), b.i(2)),
    ]);
    assert_eq!(msg, "tuple does not support item assignment");
}

// Loops.

#[test]
fn for_over_range_and_list_of_tuples() {
    let b = B::default();
    let total = b.t("total");
    let (tk, tv) = (b.t("k"), b.t("v"));
    let (s_total, s_v) = (total.span, tv.span);
    let pairs = b.list(vec![b.tuple(vec![b.i(1), b.s("a")])]);
    let types = ok(vec![
        b.assign_to(total, b.i(0)),
        b.for_(
            b.t("i"),
            b.call("range", vec![b.i(10)]),
            vec![b.aug("total", BinOp::Add, b.n("i"))],
        ),
        b.for_(b.ttuple(vec![tk, tv]), pairs, vec![b.ex(b.n("k"))]),
    ]);
    assert_eq!(types.at(s_total), Some(&Type::Int));
    assert_eq!(types.at(s_v), Some(&Type::Str));
}

#[test]
fn iterating_an_int_is_an_error() {
    let b = B::default();
    let msg = err(vec![b.for_(b.t("i"), b.i(3), vec![b.ex(b.n("i"))])]);
    assert_eq!(msg, "int is not iterable");
}

// Functions.

#[test]
fn identity_is_generic_across_calls() {
    let b = B::default();
    let (ta, tb) = (b.t("a"), b.t("b"));
    let (sa, sb) = (ta.span, tb.span);
    let types = ok(vec![
        b.def("ident", &["x"], vec![b.ret(b.n("x"))]),
        b.assign_to(ta, b.call("ident", vec![b.i(1)])),
        b.assign_to(tb, b.call("ident", vec![b.s("s")])),
    ]);
    assert_eq!(types.at(sa), Some(&Type::Int));
    assert_eq!(types.at(sb), Some(&Type::Str));
    let sig = types
        .func("ident")
        .map(|f| (f.params[0].ty.clone(), f.ret.clone()));
    assert!(matches!(sig, Some((Type::Var(a), Type::Var(r))) if a == r));
}

#[test]
fn generic_constraints_are_checked_per_call() {
    let b = B::default();
    let add = || {
        b.def(
            "add",
            &["a", "b"],
            vec![b.ret(b.bin(BinOp::Add, b.n("a"), b.n("b")))],
        )
    };
    ok(vec![
        add(),
        b.ex(b.call("add", vec![b.i(1), b.i(2)])),
        b.ex(b.call("add", vec![b.s("a"), b.s("b")])),
    ]);
    let msg = err(vec![add(), b.ex(b.call("add", vec![b.s("a"), b.i(1)]))]);
    assert_eq!(
        msg,
        "in call to `add`: unsupported operand types for +: str and int"
    );
}

#[test]
fn uncalled_generic_function_is_accepted() {
    let b = B::default();
    let body = vec![b.ret(b.bin(BinOp::Add, b.n("x"), b.i(1)))];
    ok(vec![b.def("inc", &["x"], body)]);
}

#[test]
fn recursive_fibonacci() {
    let b = B::default();
    let fib = |n: Expr| b.call("fib", vec![n]);
    let body = vec![
        b.if_(
            b.cmp(b.n("n"), vec![(CmpOp::Lt, b.i(2))]),
            vec![b.ret(b.n("n"))],
            vec![],
        ),
        b.ret(b.bin(
            BinOp::Add,
            fib(b.bin(BinOp::Sub, b.n("n"), b.i(1))),
            fib(b.bin(BinOp::Sub, b.n("n"), b.i(2))),
        )),
    ];
    let target = b.t("r");
    let span = target.span;
    let types = ok(vec![
        b.def("fib", &["n"], body),
        b.assign_to(target, fib(b.i(10))),
    ]);
    assert_eq!(types.at(span), Some(&Type::Int));
}

#[test]
fn mutual_recursion_is_inferred_together() {
    let b = B::default();
    let step = |name: &str, other: &str| {
        let zero = b.cmp(b.n("n"), vec![(CmpOp::Eq, b.i(0))]);
        let rec = b.call(other, vec![b.bin(BinOp::Sub, b.n("n"), b.i(1))]);
        b.def(name, &["n"], vec![b.ret(b.ifexp(zero, b.bl(true), rec))])
    };
    let target = b.t("r");
    let span = target.span;
    let types = ok(vec![
        step("even", "odd"),
        step("odd", "even"),
        b.assign_to(target, b.call("even", vec![b.i(4)])),
    ]);
    assert_eq!(types.at(span), Some(&Type::Bool));
}

#[test]
fn functions_read_module_globals() {
    let b = B::default();
    let types = ok(vec![
        b.assign("n", b.i(10)),
        b.def(
            "twice",
            &[],
            vec![b.ret(b.bin(BinOp::Mul, b.n("n"), b.i(2)))],
        ),
        b.ex(b.call("print", vec![b.call("twice", vec![])])),
    ]);
    assert_eq!(types.func("twice").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn annotations_must_agree() {
    let b = B::default();
    let msg = err(vec![b.ann("x", b.ty("int", vec![]), b.s("a"))]);
    assert_eq!(
        msg,
        "value does not match its annotation: expected int, found str"
    );

    let p = b.param("x", Some(b.ty("int", vec![])), None);
    let msg = err(vec![
        b.def_full("f", vec![p], None, vec![b.ret(b.n("x"))]),
        b.ex(b.call("f", vec![b.s("a")])),
    ]);
    assert_eq!(msg, "argument `x` of `f`: expected int, found str");

    let ret = Some(b.ty("str", vec![]));
    let msg = err(vec![b.def_full("g", vec![], ret, vec![b.ret(b.i(1))])]);
    assert_eq!(msg, "return type mismatch: expected str, found int");
}

#[test]
fn annotation_shapes_are_checked() {
    let b = B::default();
    let list_int = b.ty("list", vec![b.ty("int", vec![])]);
    let target = b.id("xs");
    ok(vec![b.st(StmtKind::AnnAssign {
        target,
        annotation: list_int,
        value: b.list(vec![]),
    })]);
    let bad = b.ty("list", vec![b.ty("int", vec![]), b.ty("str", vec![])]);
    let msg = err(vec![b.ann("xs", bad, b.list(vec![]))]);
    assert_eq!(msg, "`list` takes one type argument");
    let msg = err(vec![b.ann("x", b.ty("dict", vec![]), b.i(1))]);
    assert_eq!(msg, "unknown type `dict`");
    let msg = err(vec![b.ann("x", b.ty("tuple", vec![]), b.i(1))]);
    assert!(msg.starts_with("`tuple` needs its element types"), "{msg}");
}

#[test]
fn falling_off_the_end_returns_none() {
    let b = B::default();
    let body = vec![b.if_(b.n("x"), vec![b.ret(b.i(1))], vec![])];
    let msg = err(vec![b.def("f", &["x"], body)]);
    assert_eq!(
        msg,
        "`f` returns int, but can reach its end, which returns None; \
         annotate `-> int | None` if that is intended"
    );
}

#[test]
fn while_true_without_break_does_not_fall_through() {
    let b = B::default();
    let body = vec![b.while_(b.bl(true), vec![b.ret(b.i(1))])];
    let types = ok(vec![b.def("f", &[], body)]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn if_else_that_always_returns_does_not_fall_through() {
    let b = B::default();
    let body = vec![b.if_(b.n("x"), vec![b.ret(b.i(1))], vec![b.ret(b.i(2))])];
    ok(vec![b.def("f", &["x"], body)]);
}

#[test]
fn argument_binding_follows_python() {
    let b = B::default();
    let def = || {
        let params = vec![b.param("a", None, None), b.param("c", None, Some(b.i(2)))];
        b.def_full("f", params, None, vec![b.ret(b.n("a"))])
    };
    ok(vec![
        def(),
        b.ex(b.call_args("f", vec![b.arg(b.i(1)), b.kw("c", b.i(3))])),
    ]);
    let cases: Vec<(Vec<Arg>, &str)> = vec![
        (vec![b.kw("c", b.i(1))], "`f` is missing argument `a`"),
        (
            vec![b.arg(b.i(1)), b.kw("z", b.i(1))],
            "`f` got an unexpected keyword argument `z`",
        ),
        (
            vec![b.arg(b.i(1)), b.kw("a", b.i(1))],
            "`f` got multiple values for argument `a`",
        ),
        (
            vec![b.arg(b.i(1)), b.arg(b.i(2)), b.arg(b.i(3))],
            "`f` takes 2 arguments but 3 were given",
        ),
        (
            vec![b.kw("a", b.i(1)), b.arg(b.i(2))],
            "positional argument follows keyword argument",
        ),
    ];
    for (args, expected) in cases {
        assert_eq!(err(vec![def(), b.ex(b.call_args("f", args))]), expected);
    }
}

#[test]
fn mutable_default_is_not_generic() {
    let b = B::default();
    let p = b.param("xs", None, Some(b.list(vec![])));
    let body = vec![
        b.ex(b.method(b.n("xs"), "append", vec![b.i(1)])),
        b.ret(b.n("xs")),
    ];
    let types = ok(vec![b.def_full("f", vec![p], None, body)]);
    assert_eq!(
        types.func("f").map(|f| f.ret.clone()),
        Some(list_of(Type::Int))
    );
}

// Builtins and names.

#[test]
fn builtins_are_typed() {
    let b = B::default();
    assert_eq!(type_of_value(&b, b.call("len", vec![b.s("ab")])), Type::Int);
    assert_eq!(type_of_value(&b, b.call("str", vec![b.i(1)])), Type::Str);
    assert_eq!(type_of_value(&b, b.call("int", vec![b.s("3")])), Type::Int);
    assert_eq!(
        type_of_value(&b, b.call("float", vec![b.i(3)])),
        Type::Float
    );
    assert_eq!(
        type_of_value(&b, b.call("range", vec![b.i(1), b.i(9)])),
        Type::Range
    );
    assert_eq!(
        type_of_value(&b, b.call("list", vec![b.list(vec![b.s("a")])])),
        list_of(Type::Str)
    );
    let print = b.call_args("print", vec![b.arg(b.i(1)), b.kw("end", b.s(""))]);
    assert_eq!(type_of_value(&b, print), Type::None);
}

#[test]
fn list_copy_shares_its_element_type() {
    let b = B::default();
    let body = vec![
        b.assign("a", b.list(vec![b.i(1)])),
        b.assign("c", b.call("list", vec![b.n("a")])),
        b.ex(b.method(b.n("c"), "append", vec![b.s("x")])),
    ];
    assert!(err(body).contains("expected int, found str"));
}

#[test]
fn builtin_misuse_is_an_error() {
    let b = B::default();
    let cases: Vec<(Expr, &str)> = vec![
        (
            b.call("len", vec![b.i(5)]),
            "object of type int has no len()",
        ),
        (
            b.call("len", vec![]),
            "`len` takes 1 arguments but 0 were given",
        ),
        (
            b.call("int", vec![b.list(vec![])]),
            "int() argument must be int, float, str or bool, not list[T0]",
        ),
        (
            b.call_args("print", vec![b.kw("end", b.i(1))]),
            "`end` of `print`: expected str, found int",
        ),
        (
            b.call("range", vec![b.s("a")]),
            "`range` takes ints: expected int, found str",
        ),
        (
            b.call("list", vec![b.s("ab")]),
            "`list` copies a list: expected list[T0], found str",
        ),
        (
            b.call("list", vec![]),
            "`list` takes 1 arguments but 0 were given",
        ),
    ];
    for (e, expected) in cases {
        assert_eq!(err(vec![b.ex(e)]), expected);
    }
}

#[test]
fn name_errors() {
    let b = B::default();
    assert_eq!(err(vec![b.ex(b.n("nope"))]), "name `nope` is not defined");
    assert_eq!(
        err(vec![b.ex(b.call("nope", vec![]))]),
        "name `nope` is not defined"
    );
    let msg = err(vec![b.def("f", &[], vec![]), b.assign("g", b.n("f"))]);
    assert_eq!(msg, "`f` is a function; functions are not values yet");
    let msg = err(vec![
        b.assign("len", b.i(3)),
        b.ex(b.call("len", vec![b.s("a")])),
    ]);
    assert_eq!(msg, "`len` is int, not a function");
}

#[test]
fn structural_errors() {
    let b = B::default();
    let msg = err(vec![b.def("f", &[], vec![]), b.def("f", &[], vec![])]);
    assert_eq!(msg, "`f` is defined twice");
    let msg = err(vec![b.def("f", &[], vec![]), b.assign("f", b.i(1))]);
    assert_eq!(msg, "`f` is both a function and a variable");
    let msg = err(vec![b.ret(b.i(1))]);
    assert_eq!(msg, "`return` outside a function");
    let inner = b.def("g", &[], vec![]);
    let msg = err(vec![b.def("f", &[], vec![inner])]);
    assert_eq!(msg, "`def` is only allowed at module top level");
}

#[test]
fn errors_carry_the_span_of_the_offending_node() {
    let b = B::default();
    let bad = b.s("a");
    let span = bad.span;
    let result = infer(&module(vec![b.ex(b.list(vec![b.i(1), bad]))]));
    assert_eq!(result.err().map(|d| d.span), Some(span));
}

#[test]
fn default_aliased_through_another_function_stays_fixed() {
    let b = B::default();
    let p = b.param("xs", None, Some(b.list(vec![])));
    let msg = err(vec![
        b.def_full("shared", vec![p], None, vec![b.ret(b.n("xs"))]),
        b.def("get", &[], vec![b.ret(b.call("shared", vec![]))]),
        b.ex(b.method(b.call("get", vec![]), "append", vec![b.i(1)])),
        b.ex(b.method(b.call("get", vec![]), "append", vec![b.s("s")])),
    ]);
    assert_eq!(
        msg,
        "appended value does not match the list: expected int, found str"
    );
}

#[test]
fn default_bound_after_its_def_stays_fixed() {
    let b = B::default();
    let p = b.param("xs", None, Some(b.list(vec![])));
    let body = vec![
        b.ex(b.method(b.n("xs"), "append", vec![b.list(vec![])])),
        b.ret(b.n("xs")),
    ];
    let inner = |v: Expr| b.method(b.idx(b.call("f", vec![]), b.i(0)), "append", vec![v]);
    let msg = err(vec![
        b.def_full("f", vec![p], None, body),
        b.ex(inner(b.i(1))),
        b.ex(inner(b.s("s"))),
    ]);
    assert!(
        msg.contains("list[int]") && msg.contains("list[str]"),
        "{msg}"
    );
}

#[test]
fn break_in_nested_loop_else_exits_the_outer_loop() {
    let b = B::default();
    let inner = b.st(StmtKind::For {
        target: b.t("x"),
        iter: b.list(vec![b.i(1)]),
        body: vec![b.st(StmtKind::Pass)],
        orelse: vec![b.st(StmtKind::Break)],
    });
    let body = vec![b.while_(b.bl(true), vec![inner, b.ret(b.i(1))])];
    let msg = err(vec![b.def("f", &[], body)]);
    assert_eq!(
        msg,
        "`f` returns int, but can reach its end, which returns None; \
         annotate `-> int | None` if that is intended"
    );
}

#[test]
fn break_in_nested_loop_body_does_not_exit_the_outer_loop() {
    let b = B::default();
    let inner = b.for_(b.t("x"), b.list(vec![b.i(1)]), vec![b.st(StmtKind::Break)]);
    let body = vec![b.while_(b.bl(true), vec![inner, b.ret(b.i(1))])];
    let types = ok(vec![b.def("f", &[], body)]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn negative_literal_exponent_on_int_base_is_an_error() {
    let b = B::default();
    let e = b.bin(BinOp::Pow, b.i(2), b.i(-1));
    let msg = err(vec![b.ex(e)]);
    assert!(msg.contains("use a float base"), "{msg}");
    let e = b.bin(BinOp::Pow, b.i(2), b.neg(b.i(1)));
    assert!(err(vec![b.ex(e)]).contains("use a float base"));
}

#[test]
fn negative_exponent_on_float_base_or_unknown_sign_is_accepted() {
    let b = B::default();
    let e = b.bin(BinOp::Pow, b.fl(2.0), b.i(-1));
    assert_eq!(type_of_value(&b, e), Type::Float);
    let target = b.t("x");
    let span = target.span;
    let types = ok(vec![
        b.assign("n", b.i(-1)),
        b.assign_to(target, b.bin(BinOp::Pow, b.i(2), b.n("n"))),
    ]);
    assert_eq!(types.at(span), Some(&Type::Int));
}

// Optional (`T | None`).

fn int_or_none() -> Type {
    Type::Optional(Box::new(Type::Int))
}

#[test]
fn optional_variable_holds_none_or_its_type() {
    let b = B::default();
    let target = b.t("x");
    let span = target.span;
    let types = ok(vec![
        b.ann("x", b.opt("int", vec![]), b.e(ExprKind::None)),
        b.assign_to(target, b.i(5)),
        b.assign("x", b.e(ExprKind::None)),
    ]);
    assert_eq!(types.at(span), Some(&int_or_none()));
}

#[test]
fn none_first_without_annotation_asks_for_one() {
    let b = B::default();
    let msg = err(vec![
        b.assign("x", b.e(ExprKind::None)),
        b.assign("x", b.i(5)),
    ]);
    assert_eq!(
        msg,
        "cannot change the type of `x`: expected None, found int; \
         to allow None, annotate it, as in `x: int | None = None`"
    );
}

#[test]
fn optional_is_not_its_inner_type() {
    let b = B::default();
    let msg = err(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1))),
    ]);
    assert_eq!(msg, "unsupported operand types for +: int | None and int");
    let msg = err(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.ann("y", b.ty("int", vec![]), b.n("x")),
    ]);
    assert!(msg.ends_with("expected int, found int | None"), "{msg}");
}

#[test]
fn optional_compares_with_none_and_its_type() {
    let b = B::default();
    ok(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.ex(b.cmp(b.n("x"), vec![(CmpOp::Eq, b.e(ExprKind::None))])),
        b.ex(b.cmp(b.n("x"), vec![(CmpOp::NotEq, b.i(2))])),
    ]);
    let msg = err(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.ex(b.cmp(b.n("x"), vec![(CmpOp::Lt, b.i(2))])),
    ]);
    assert_eq!(msg, "cannot order int | None against int");
}

#[test]
fn list_of_optional_takes_mixed_literal_and_appends() {
    let b = B::default();
    let annotation = b.ty("list", vec![b.opt("int", vec![])]);
    let target = b.t("ys");
    let span = target.span;
    let types = ok(vec![
        b.ann("xs", annotation, b.list(vec![b.e(ExprKind::None), b.i(1)])),
        b.ex(b.method(b.n("xs"), "append", vec![b.e(ExprKind::None)])),
        b.ex(b.method(b.n("xs"), "append", vec![b.i(2)])),
        b.assign_to(target, b.n("xs")),
    ]);
    assert_eq!(types.at(span), Some(&list_of(int_or_none())));
}

#[test]
fn optional_return_may_fall_through() {
    let b = B::default();
    let body = vec![b.if_(b.n("x"), vec![b.ret(b.i(1))], vec![])];
    let p = b.param("x", None, None);
    let types = ok(vec![b.def_full(
        "f",
        vec![p],
        Some(b.opt("int", vec![])),
        body,
    )]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(int_or_none()));
    let body = vec![b.if_(b.n("x"), vec![b.ret(b.i(1))], vec![])];
    let msg = err(vec![b.def("g", &["x"], body)]);
    assert!(
        msg.ends_with("annotate `-> int | None` if that is intended"),
        "{msg}"
    );
}

#[test]
fn optional_parameter_with_none_default() {
    let b = B::default();
    let p = b.param("x", Some(b.opt("int", vec![])), Some(b.e(ExprKind::None)));
    ok(vec![
        b.def_full("f", vec![p], None, vec![b.ret(b.n("x"))]),
        b.ex(b.call("f", vec![])),
        b.ex(b.call("f", vec![b.i(3)])),
    ]);
}

#[test]
fn none_or_none_annotation_is_none() {
    let b = B::default();
    let target = b.id("x");
    let span = target.span;
    let types = ok(vec![b.st(StmtKind::AnnAssign {
        target,
        annotation: b.opt("None", vec![]),
        value: b.e(ExprKind::None),
    })]);
    assert_eq!(types.at(span), Some(&Type::None));
}

// Narrowing with `is None` / `is not None`.

fn is_none(b: &B, name: &str, op: CmpOp) -> Expr {
    b.cmp(b.n(name), vec![(op, b.e(ExprKind::None))])
}

#[test]
fn is_not_none_narrows_the_branch() {
    let b = B::default();
    let target = b.t("y");
    let span = target.span;
    let body = vec![b.assign_to(target, b.bin(BinOp::Add, b.n("x"), b.i(1)))];
    let types = ok(vec![
        b.ann("x", b.opt("int", vec![]), b.e(ExprKind::None)),
        b.if_(is_none(&b, "x", CmpOp::IsNot), body, vec![]),
    ]);
    assert_eq!(types.at(span), Some(&Type::Int));
}

#[test]
fn is_none_narrows_the_else_branch() {
    let b = B::default();
    let orelse = vec![b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1)))];
    ok(vec![
        b.ann("x", b.opt("int", vec![]), b.e(ExprKind::None)),
        b.if_(
            is_none(&b, "x", CmpOp::Is),
            vec![b.st(StmtKind::Pass)],
            orelse,
        ),
    ]);
}

#[test]
fn narrowing_does_not_leak_or_survive_assignment() {
    let b = B::default();
    let msg = err(vec![
        b.ann("x", b.opt("int", vec![]), b.e(ExprKind::None)),
        b.if_(
            is_none(&b, "x", CmpOp::IsNot),
            vec![b.st(StmtKind::Pass)],
            vec![],
        ),
        b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1))),
    ]);
    assert_eq!(msg, "unsupported operand types for +: int | None and int");
    let body = vec![
        b.assign("x", b.e(ExprKind::None)),
        b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1))),
    ];
    let msg = err(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.if_(is_none(&b, "x", CmpOp::IsNot), body, vec![]),
    ]);
    assert_eq!(msg, "unsupported operand types for +: int | None and int");
}

#[test]
fn narrowed_optional_parameter_returns_its_type() {
    let b = B::default();
    let p = b.param("x", Some(b.opt("int", vec![])), Some(b.e(ExprKind::None)));
    let body = vec![
        b.if_(
            is_none(&b, "x", CmpOp::IsNot),
            vec![b.ret(b.bin(BinOp::Mul, b.n("x"), b.i(2)))],
            vec![],
        ),
        b.ret(b.i(0)),
    ];
    let types = ok(vec![b.def_full("f", vec![p], None, body)]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn while_is_not_none_narrows_its_body() {
    let b = B::default();
    let body = vec![
        b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1))),
        b.st(StmtKind::Break),
    ];
    ok(vec![
        b.ann("x", b.opt("int", vec![]), b.i(1)),
        b.while_(is_none(&b, "x", CmpOp::IsNot), body),
    ]);
}

// Narrowing inside `and`, `or`, `not` and `x if c else y`.

const UNNARROWED: &str = "unsupported operand types for +: int | None and int";

/// `x: int | None = None` and `y: int | None = None`.
fn two_optionals(b: &B) -> Vec<Stmt> {
    ["x", "y"]
        .iter()
        .map(|n| b.ann(n, b.opt("int", vec![]), b.e(ExprKind::None)))
        .collect()
}

fn plus_one(b: &B, name: &str) -> Expr {
    b.bin(BinOp::Add, b.n(name), b.i(1))
}

fn positive(b: &B, name: &str) -> Expr {
    b.cmp(plus_one(b, name), vec![(CmpOp::Gt, b.i(0))])
}

fn with_optionals(b: &B, rest: Vec<Stmt>) -> Vec<Stmt> {
    two_optionals(b).into_iter().chain(rest).collect()
}

#[test]
fn short_circuit_narrows_the_right_operand() {
    let b = B::default();
    for (op, test) in [(LogicalOp::And, CmpOp::IsNot), (LogicalOp::Or, CmpOp::Is)] {
        let e = b.logic(op, is_none(&b, "x", test), positive(&b, "x"));
        ok(with_optionals(&b, vec![b.ex(e)]));
    }
    for (op, test) in [(LogicalOp::And, CmpOp::Is), (LogicalOp::Or, CmpOp::IsNot)] {
        let e = b.logic(op, is_none(&b, "x", test), positive(&b, "x"));
        assert_eq!(err(with_optionals(&b, vec![b.ex(e)])), UNNARROWED);
    }
}

#[test]
fn short_circuit_narrowing_does_not_leak() {
    let b = B::default();
    let e = b.logic(
        LogicalOp::And,
        is_none(&b, "x", CmpOp::IsNot),
        positive(&b, "x"),
    );
    let body = with_optionals(&b, vec![b.ex(e), b.ex(plus_one(&b, "x"))]);
    assert_eq!(err(body), UNNARROWED);
    let left = b.logic(
        LogicalOp::And,
        positive(&b, "x"),
        is_none(&b, "x", CmpOp::IsNot),
    );
    assert_eq!(err(with_optionals(&b, vec![b.ex(left)])), UNNARROWED);
}

#[test]
fn conditional_expression_narrows_each_branch() {
    let b = B::default();
    let e = b.ifexp(is_none(&b, "x", CmpOp::IsNot), plus_one(&b, "x"), b.i(0));
    let target = b.t("z");
    let span = target.span;
    let types = ok(with_optionals(&b, vec![b.assign_to(target, e)]));
    assert_eq!(types.at(span), Some(&Type::Int));
    let e = b.ifexp(is_none(&b, "x", CmpOp::Is), b.i(0), plus_one(&b, "x"));
    ok(with_optionals(&b, vec![b.ex(e)]));
    let e = b.ifexp(is_none(&b, "x", CmpOp::Is), plus_one(&b, "x"), b.i(0));
    assert_eq!(err(with_optionals(&b, vec![b.ex(e)])), UNNARROWED);
}

#[test]
fn compound_tests_narrow_if_clauses() {
    let b = B::default();
    let both = || {
        let (x, y) = (
            is_none(&b, "x", CmpOp::IsNot),
            is_none(&b, "y", CmpOp::IsNot),
        );
        b.logic(LogicalOp::And, x, y)
    };
    let sum = || b.ex(b.bin(BinOp::Add, b.n("x"), b.n("y")));
    ok(with_optionals(&b, vec![b.if_(both(), vec![sum()], vec![])]));
    let either = || {
        let (x, y) = (is_none(&b, "x", CmpOp::Is), is_none(&b, "y", CmpOp::Is));
        b.logic(LogicalOp::Or, x, y)
    };
    let pass = || vec![b.st(StmtKind::Pass)];
    ok(with_optionals(
        &b,
        vec![b.if_(either(), pass(), vec![sum()])],
    ));
    // `and` failing proves neither side; `or` holding proves neither.
    let neither = b.if_(both(), pass(), vec![b.ex(plus_one(&b, "x"))]);
    assert_eq!(err(with_optionals(&b, vec![neither])), UNNARROWED);
    let neither = b.if_(either(), vec![b.ex(plus_one(&b, "x"))], vec![]);
    assert_eq!(err(with_optionals(&b, vec![neither])), UNNARROWED);
}

#[test]
fn or_test_narrows_later_clauses_and_guards() {
    let b = B::default();
    let best_test = || {
        b.logic(
            LogicalOp::Or,
            is_none(&b, "x", CmpOp::Is),
            positive(&b, "x"),
        )
    };
    let chain = b.st(StmtKind::If {
        branch: b.branch(best_test(), vec![b.st(StmtKind::Pass)]),
        elifs: vec![b.branch(b.bl(true), vec![b.ex(plus_one(&b, "x"))])],
        orelse: vec![b.ex(plus_one(&b, "x"))],
    });
    ok(with_optionals(&b, vec![chain]));
    let either = b.logic(
        LogicalOp::Or,
        is_none(&b, "x", CmpOp::Is),
        is_none(&b, "y", CmpOp::Is),
    );
    let body = vec![
        b.if_(either, vec![b.ret(b.i(0))], vec![]),
        b.ret(b.bin(BinOp::Add, b.n("x"), b.n("y"))),
    ];
    let p = |n: &str| b.param(n, Some(b.opt("int", vec![])), Some(b.e(ExprKind::None)));
    let types = ok(vec![b.def_full("f", vec![p("x"), p("y")], None, body)]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn not_flips_what_a_test_proves() {
    let b = B::default();
    let not = |e: Expr| {
        b.e(ExprKind::Unary {
            op: UnaryOp::Not,
            operand: Box::new(e),
        })
    };
    let test = not(is_none(&b, "x", CmpOp::Is));
    ok(with_optionals(
        &b,
        vec![b.if_(test, vec![b.ex(plus_one(&b, "x"))], vec![])],
    ));
    let test = not(is_none(&b, "x", CmpOp::IsNot));
    let body = vec![b.if_(test, vec![b.ex(plus_one(&b, "x"))], vec![])];
    assert_eq!(err(with_optionals(&b, body)), UNNARROWED);
}

/// The type error in `body`, after `x: int | None` and `y: int | None`.
/// Parsed, so that spans nest as the truth-test hint needs.
fn optionals_err(body: &str) -> String {
    let src = format!("x: int | None = None\ny: int | None = None\n{body}");
    let parsed = crate::lexer::lex(&src).and_then(|t| crate::parser::parse(&t));
    match parsed.map(|m| infer(&m)) {
        Ok(Err(e)) => e.message,
        Ok(Ok(_)) => "no error".to_string(),
        Err(e) => format!("does not parse: {}", e.message),
    }
}

#[test]
fn truth_tests_do_not_narrow_but_suggest_is_not_none() {
    let hint = |n: &str| {
        format!("{UNNARROWED}; a truth test does not narrow `{n}`: write `{n} is not None`")
    };
    let cases = [
        "if x:\n    print(x + 1)\n",
        "while x:\n    print(x + 1)\n",
        "if x:\n    if x:\n        print(x + 1)\n",
        "if x:\n    if y:\n        print(x + 1)\n",
        "if y and x:\n    print(x + 1)\n",
    ];
    for body in cases {
        assert_eq!(optionals_err(body), hint("x"), "{body}");
    }
    let fix = "; a truth test does not narrow `x`: write `x is not None`";
    for body in ["1 + x", "-x", "x > 0", "0 < x"] {
        let msg = optionals_err(&format!("if x:\n    print({body})\n"));
        assert!(msg.ends_with(fix), "{body}: {msg}");
    }
}

#[test]
fn truth_test_hint_names_only_a_fixable_failing_use() {
    let cases = [
        // Another name, even one the body reads harmlessly.
        "if y:\n    print(y)\n    print(x + 1)\n",
        // `x is not None` would not narrow an `x` the body assigns.
        "if x:\n    x = None\n    print(x + 1)\n",
        // After the truth test's body.
        "if x:\n    pass\nprint(x + 1)\n",
    ];
    for body in cases {
        assert_eq!(optionals_err(body), UNNARROWED, "{body}");
    }
    // The failing operand is not the truth-tested read, or narrowing it
    // would not fix the error.
    let unfixable = [
        (
            "if x:\n    print(print(x) + 1)\n",
            "unsupported operand types for +: None and int",
        ),
        (
            "if y:\n    print(bool(y) + x)\n",
            "unsupported operand types for +: bool and int | None",
        ),
        (
            "if x:\n    print(x + \"a\")\n",
            "unsupported operand types for +: int | None and str",
        ),
    ];
    for (body, msg) in unfixable {
        assert_eq!(optionals_err(body), msg, "{body}");
    }
    let narrowed = "if x:\n    if x is not None:\n        print(x + 1)\n";
    assert_eq!(optionals_err(narrowed), "no error");
}

#[test]
fn is_with_a_non_none_operand_is_an_error() {
    let b = B::default();
    let msg = err(vec![b.ex(b.cmp(b.i(1), vec![(CmpOp::Is, b.i(1))]))]);
    assert_eq!(
        msg,
        "`is` compares only with None in phase 2; use `==` for values"
    );
}

// Values flowing into `T | None`.

#[test]
fn computed_values_flow_into_optional() {
    let b = B::default();
    let target = b.t("x");
    let sum = b.bin(BinOp::Add, b.i(1), b.i(2));
    let sum_span = sum.span;
    let types = ok(vec![
        b.ann("x", b.opt("int", vec![]), sum),
        b.assign_to(target, b.bin(BinOp::Mul, b.i(3), b.i(4))),
        b.assign("x", b.e(ExprKind::None)),
    ]);
    assert_eq!(types.at(sum_span), Some(&Type::Int));
    let msg = err(vec![b.ann(
        "x",
        b.opt("int", vec![]),
        b.bin(BinOp::Add, b.s("a"), b.s("b")),
    )]);
    assert_eq!(
        msg,
        "value does not match its annotation: expected int | None, found str"
    );
}

#[test]
fn computed_returns_arguments_and_appends_flow_into_optional() {
    let b = B::default();
    let ret = b.ret(b.bin(BinOp::Add, b.i(1), b.i(2)));
    let p = b.param("y", Some(b.opt("int", vec![])), None);
    ok(vec![
        b.def_full("f", vec![], Some(b.opt("int", vec![])), vec![ret]),
        b.def_full("g", vec![p], None, vec![b.ret(b.i(0))]),
        b.ex(b.call("g", vec![b.bin(BinOp::Add, b.i(1), b.i(2))])),
        b.ann(
            "xs",
            b.ty("list", vec![b.opt("int", vec![])]),
            b.list(vec![]),
        ),
        b.ex(b.method(b.n("xs"), "append", vec![b.bin(BinOp::Add, b.i(1), b.i(2))])),
    ]);
}

#[test]
fn optional_return_of_a_parameter_is_checked_per_call() {
    let b = B::default();
    let def = b.def_full(
        "f",
        vec![b.param("a", None, None)],
        Some(b.opt("int", vec![])),
        vec![b.ret(b.n("a"))],
    );
    ok(vec![
        def.clone(),
        b.ex(b.call("f", vec![b.i(1)])),
        b.ex(b.call("f", vec![b.e(ExprKind::None)])),
    ]);
    let msg = err(vec![def, b.ex(b.call("f", vec![b.s("s")]))]);
    assert_eq!(
        msg,
        "in call to `f`: return type mismatch: expected int | None, found str"
    );
}

#[test]
fn stores_check_against_the_element_type() {
    let b = B::default();
    let opt_list = || b.ty("list", vec![b.opt("int", vec![])]);
    let store = |v: Expr| b.assign_to(b.tidx(b.n("xs"), b.i(0)), v);
    ok(vec![
        b.ann("xs", opt_list(), b.list(vec![b.e(ExprKind::None)])),
        store(b.i(1)),
        store(b.e(ExprKind::None)),
        store(b.bin(BinOp::Add, b.i(1), b.i(2))),
    ]);
    let msg = err(vec![
        b.ann("xs", opt_list(), b.list(vec![b.e(ExprKind::None)])),
        store(b.s("a")),
    ]);
    assert_eq!(msg, "cannot assign str into list[int | None]");
}

#[test]
fn unconstrained_value_takes_the_optional_target() {
    let b = B::default();
    let target = b.t("xs");
    let span = target.span;
    let types = ok(vec![
        b.assign_to(target, b.list(vec![])),
        b.ann("x", b.opt("int", vec![]), b.idx(b.n("xs"), b.i(0))),
    ]);
    assert_eq!(types.at(span), Some(&list_of(int_or_none())));
}

#[test]
fn literals_take_the_expected_type_at_calls_returns_and_reassignment() {
    let b = B::default();
    let opt_list = || b.ty("list", vec![b.opt("int", vec![])]);
    let mixed = || b.list(vec![b.e(ExprKind::None), b.i(1)]);
    let nested = b.param("xss", Some(b.ty("list", vec![opt_list()])), None);
    ok(vec![
        b.def_full(
            "f",
            vec![b.param("xs", Some(opt_list()), None)],
            None,
            vec![b.ret(b.i(0))],
        ),
        b.def_full("g", vec![], Some(opt_list()), vec![b.ret(mixed())]),
        b.def_full("h", vec![nested], None, vec![b.ret(b.i(0))]),
        b.ex(b.call("f", vec![mixed()])),
        b.ex(b.call(
            "h",
            vec![b.list(vec![b.list(vec![b.e(ExprKind::None)]), mixed()])],
        )),
        b.ann("ys", opt_list(), b.list(vec![])),
        b.assign("ys", mixed()),
    ]);
}

#[test]
fn an_existing_list_is_not_coerced_into_optional() {
    let b = B::default();
    let opt_list = || b.ty("list", vec![b.opt("int", vec![])]);
    let msg = err(vec![
        b.def_full(
            "f",
            vec![b.param("xs", Some(opt_list()), None)],
            None,
            vec![b.ret(b.i(0))],
        ),
        b.assign("ys", b.list(vec![b.i(1)])),
        b.ex(b.call("f", vec![b.n("ys")])),
    ]);
    assert_eq!(
        msg,
        "argument `xs` of `f`: expected list[int | None], found list[int]"
    );
    let msg = err(vec![
        b.ann("xs", opt_list(), b.list(vec![])),
        b.assign("ys", b.list(vec![b.i(1)])),
        b.assign("xs", b.n("ys")),
    ]);
    assert_eq!(
        msg,
        "cannot change the type of `xs`: expected list[int | None], found list[int]"
    );
}

// Annotated variables are typed before inference reaches them.

#[test]
fn annotated_global_narrows_in_a_function_wherever_it_is_defined() {
    let b = B::default();
    let def = || {
        let body = vec![
            b.if_(
                is_none(&b, "x", CmpOp::IsNot),
                vec![b.ret(b.bin(BinOp::Add, b.n("x"), b.i(1)))],
                vec![],
            ),
            b.ret(b.i(0)),
        ];
        b.def("f", &[], body)
    };
    let global = || b.ann("x", b.opt("int", vec![]), b.i(1));
    for body in [vec![global(), def()], vec![def(), global()]] {
        let types = ok(body);
        assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
    }
}

#[test]
fn conflicting_annotations_are_an_error() {
    let b = B::default();
    let msg = err(vec![
        b.ann("x", b.ty("int", vec![]), b.i(1)),
        b.ann("x", b.ty("str", vec![]), b.s("a")),
    ]);
    assert_eq!(
        msg,
        "conflicting annotations for `x`: expected int, found str"
    );
}

#[test]
fn literals_under_optional_take_the_inner_type() {
    let b = B::default();
    let opt_list = || b.ty("list", vec![b.opt("int", vec![])]);
    let mixed = || b.list(vec![b.e(ExprKind::None), b.i(1)]);
    let literal = mixed();
    let span = literal.span;
    let outer = TypeExpr {
        optional: true,
        ..opt_list()
    };
    let nested = b.ty(
        "list",
        vec![TypeExpr {
            optional: true,
            ..opt_list()
        }],
    );
    let pair = b.ty("tuple", vec![b.opt("int", vec![]), b.ty("str", vec![])]);
    let opt_pair = TypeExpr {
        optional: true,
        ..pair
    };
    let types = ok(vec![
        b.ann("xs", outer, literal),
        b.ann("xss", nested, b.list(vec![mixed(), b.e(ExprKind::None)])),
        b.ann("p", opt_pair, b.tuple(vec![b.e(ExprKind::None), b.s("a")])),
    ]);
    assert_eq!(types.at(span), Some(&list_of(int_or_none())));
}

#[test]
fn literals_take_the_element_type_at_stores_and_appends() {
    let b = B::default();
    let nested = || b.ty("list", vec![b.ty("list", vec![b.opt("int", vec![])])]);
    let mixed = || b.list(vec![b.e(ExprKind::None), b.i(1)]);
    ok(vec![
        b.ann("xss", nested(), b.list(vec![b.list(vec![])])),
        b.assign_to(b.tidx(b.n("xss"), b.i(0)), mixed()),
        b.ex(b.method(b.n("xss"), "append", vec![mixed()])),
        b.assign_to(
            b.tidx(b.idx(b.n("xss"), b.i(0)), b.i(0)),
            b.e(ExprKind::None),
        ),
    ]);
    let msg = err(vec![
        b.ann("xss", nested(), b.list(vec![])),
        b.assign("ys", b.list(vec![b.i(1)])),
        b.ex(b.method(b.n("xss"), "append", vec![b.n("ys")])),
    ]);
    assert_eq!(
        msg,
        "appended value does not match the list: expected list[int | None], found list[int]"
    );
    let msg = err(vec![
        b.ann("xss", nested(), b.list(vec![])),
        b.assign("ys", b.list(vec![b.i(1)])),
        b.assign_to(b.tidx(b.n("xss"), b.i(0)), b.n("ys")),
    ]);
    assert_eq!(msg, "cannot assign list[int] into list[list[int | None]]");
}

#[test]
fn numeric_slots_never_widen() {
    let b = B::default();
    let floats = || b.ty("list", vec![b.ty("float", vec![])]);
    let msg = err(vec![
        b.ann("xs", floats(), b.list(vec![b.fl(1.0)])),
        b.assign_to(b.tidx(b.n("xs"), b.i(0)), b.i(1)),
    ]);
    assert_eq!(msg, "cannot assign int into list[float]");
    let msg = err(vec![b.ann("x", b.opt("float", vec![]), b.i(1))]);
    assert_eq!(
        msg,
        "value does not match its annotation: expected float | None, found int"
    );
    let msg = err(vec![b.ann("xs", floats(), b.list(vec![b.i(1)]))]);
    assert_eq!(
        msg,
        "element does not match the annotation: expected float, found int"
    );
}

// Guards: an `if` that leaves when `x` is None narrows what follows.

#[test]
fn guard_narrows_the_rest_of_the_function() {
    let b = B::default();
    let p = || b.param("x", Some(b.opt("int", vec![])), Some(b.e(ExprKind::None)));
    let plus_one = || b.ret(b.bin(BinOp::Add, b.n("x"), b.i(1)));
    let returns = vec![
        b.if_(is_none(&b, "x", CmpOp::Is), vec![b.ret(b.i(0))], vec![]),
        plus_one(),
    ];
    let types = ok(vec![b.def_full("f", vec![p()], None, returns)]);
    assert_eq!(types.func("f").map(|f| f.ret.clone()), Some(Type::Int));
    let otherwise = vec![
        b.if_(
            is_none(&b, "x", CmpOp::IsNot),
            vec![b.st(StmtKind::Pass)],
            vec![b.ret(b.i(0))],
        ),
        plus_one(),
    ];
    let types = ok(vec![b.def_full("g", vec![p()], None, otherwise)]);
    assert_eq!(types.func("g").map(|f| f.ret.clone()), Some(Type::Int));
}

#[test]
fn guard_with_continue_narrows_the_loop_body() {
    let b = B::default();
    let body = vec![
        b.if_(
            is_none(&b, "y", CmpOp::Is),
            vec![b.st(StmtKind::Continue)],
            vec![],
        ),
        b.aug("total", BinOp::Add, b.n("y")),
    ];
    ok(vec![
        b.ann(
            "ys",
            b.ty("list", vec![b.opt("int", vec![])]),
            b.list(vec![b.e(ExprKind::None), b.i(1)]),
        ),
        b.assign("total", b.i(0)),
        b.for_(b.t("y"), b.n("ys"), body),
    ]);
}

#[test]
fn guard_narrows_only_when_it_proves_not_none() {
    let b = B::default();
    let opt = || b.ann("x", b.opt("int", vec![]), b.e(ExprKind::None));
    let use_x = || b.ex(b.bin(BinOp::Add, b.n("x"), b.i(1)));
    let unsupported = "unsupported operand types for +: int | None and int";
    let stays = |c: &str| b.if_(is_none(&b, "x", CmpOp::Is), vec![b.ex(b.s(c))], vec![]);
    assert_eq!(
        err(vec![opt(), stays("body falls through"), use_x()]),
        unsupported
    );
    let reassigned = vec![
        b.if_(
            is_none(&b, "x", CmpOp::IsNot),
            vec![b.assign("x", b.e(ExprKind::None))],
            vec![b.st(StmtKind::Break)],
        ),
        use_x(),
    ];
    let body = vec![opt(), b.while_(b.bl(true), reassigned)];
    assert_eq!(err(body), unsupported);
    let after = vec![
        b.if_(
            is_none(&b, "x", CmpOp::Is),
            vec![b.st(StmtKind::Break)],
            vec![],
        ),
        b.assign("x", b.e(ExprKind::None)),
        use_x(),
    ];
    let body = vec![opt(), b.while_(b.bl(true), after)];
    assert_eq!(err(body), unsupported);
}

// ---- exceptions (D6) -------------------------------------------------------

/// Infers parsed `src`: the types, or the first error's message.
fn infer_src(src: &str) -> Result<Types, String> {
    let parsed = crate::lexer::lex(src).and_then(|t| crate::parser::parse(&t));
    let module = parsed.map_err(|e| format!("does not parse: {}", e.message))?;
    infer(&module).map_err(|e| e.message)
}

fn src_err(src: &str) -> String {
    match infer_src(src) {
        Ok(_) => "no error".to_string(),
        Err(e) => e,
    }
}

#[test]
fn exceptions_are_values_of_one_type() {
    let src = "def parse(s: str) -> int:\n    try:\n        return int(s)\n    except ValueError as e:\n        print(e)\n    except IndexError as e:\n        print(str(e))\n    return 0\n\
               err = ZeroDivisionError(\"x\")\nraise err\n";
    let types = infer_src(src).unwrap_or_else(|e| panic!("{e}"));
    let at = src.find("ZeroDivisionError").unwrap_or(0);
    let span = Span {
        start: at,
        end: at + "ZeroDivisionError(\"x\")".len(),
    };
    assert_eq!(types.at(span), Some(&Type::Exception));
    let ann = "def show(e: Exception) -> str:\n    return str(e)\nprint(show(ValueError(1)))\n";
    assert!(infer_src(ann).is_ok());
}

#[test]
fn raise_needs_an_exception() {
    assert!(src_err("raise 5\n").starts_with("`raise` needs an exception"));
    assert!(infer_src("raise ValueError\n").is_ok());
    assert!(src_err("ValueError = 1\nraise ValueError\n").starts_with("`raise` needs an exception"));
    let ctor = src_err("raise ValueError(1, 2)\n");
    assert_eq!(ctor, "`ValueError` takes 0 to 1 arguments but 2 were given");
}

#[test]
fn except_names_a_builtin_exception() {
    let handler = |name: &str| format!("try:\n    pass\nexcept {name}:\n    pass\n");
    for name in [
        "ValueError",
        "IndexError",
        "ZeroDivisionError",
        "OverflowError",
        "RecursionError",
        "ArithmeticError",
        "LookupError",
        "RuntimeError",
        "Exception",
    ] {
        assert!(infer_src(&handler(name)).is_ok(), "{name}");
    }
    assert!(src_err(&handler("KeyError")).starts_with("`KeyError` is not an exception"));
    assert!(src_err(&handler("BaseException")).starts_with("`BaseException` is not an"));
    let shadowed = format!("Exception = 1\n{}", handler("Exception"));
    assert!(src_err(&shadowed).contains("no longer names the builtin exception"));
    let shadowed = format!("def ValueError():\n    pass\n{}", handler("ValueError"));
    assert!(src_err(&shadowed).contains("no longer names the builtin exception"));
}

#[test]
fn every_class_of_a_tuple_handler_is_a_builtin_exception() {
    let handler =
        |classes: &str| format!("try:\n    pass\nexcept ({classes}) as e:\n    print(e)\n");
    assert!(infer_src(&handler("ValueError, IndexError")).is_ok());
    assert!(infer_src(&handler("ValueError, ValueError")).is_ok());
    assert!(src_err(&handler("ValueError, KeyError")).starts_with("`KeyError` is not an exception"));
    let shadowed = format!("IndexError = 1\n{}", handler("ValueError, IndexError"));
    assert!(src_err(&shadowed).contains("`IndexError` is redefined here"));
}

#[test]
fn exception_classes_are_not_held_in_variables() {
    let held = "errs = (ValueError, IndexError)\n";
    assert!(src_err(held).starts_with("exception classes can't be held in variables"));
    assert!(src_err("e = ValueError\n").starts_with("exception classes can't be held"));
    let caught = "errs = 1\ntry:\n    pass\nexcept errs:\n    pass\n";
    assert!(src_err(caught).starts_with("`errs` is a variable: exception classes can't be held"));
    let in_tuple = "errs = 1\ntry:\n    pass\nexcept (ValueError, errs):\n    pass\n";
    assert!(src_err(in_tuple).starts_with("`errs` is a variable"));
    let unknown = "try:\n    pass\nexcept errs:\n    pass\n";
    assert!(src_err(unknown).starts_with("`errs` is not an exception"));
}

#[test]
fn handler_name_has_one_fixed_type() {
    let src = "e = 1\ntry:\n    pass\nexcept ValueError as e:\n    pass\n";
    assert!(src_err(src).starts_with("cannot change the type of `e`"));
    let reuse = "try:\n    pass\nexcept ValueError as e:\n    pass\nexcept IndexError as e:\n    print(e + 1)\n";
    assert!(src_err(reuse).contains("Exception"));
}

#[test]
fn raise_ends_a_path() {
    let f = "def f(n: int) -> int:\n    if n > 0:\n        return n\n    raise ValueError(\"n\")\n";
    assert!(infer_src(f).is_ok());
    let both = "def f(s: str) -> int:\n    try:\n        return int(s)\n    except ValueError:\n        raise IndexError()\n";
    assert!(infer_src(both).is_ok());
    let open = "def f(s: str) -> int:\n    try:\n        return int(s)\n    except ValueError:\n        pass\n";
    assert!(src_err(open).contains("can reach its end"));
}

#[test]
fn raise_guard_narrows_the_rest_of_the_block() {
    let src = "def f(x: int | None) -> int:\n    if x is None:\n        raise ValueError(\"x\")\n    return x + 1\n";
    assert!(infer_src(src).is_ok());
    let in_try = "def f(x: int | None) -> int:\n    try:\n        if x is None:\n            raise ValueError(\"x\")\n        return x + 1\n    except ValueError:\n        return x + 1\n";
    assert!(src_err(in_try).contains("int | None"));
}

#[test]
fn break_inside_try_belongs_to_the_loop() {
    let src = "def f() -> int:\n    while True:\n        try:\n            break\n        except ValueError:\n            pass\n";
    assert!(src_err(src).contains("can reach its end"));
    let forever = "def f() -> int:\n    while True:\n        try:\n            pass\n        except ValueError:\n            pass\n";
    assert!(infer_src(forever).is_ok());
}

#[test]
fn a_shadowing_def_is_seen_whichever_comes_first() {
    let handler =
        "def f():\n    try:\n        raise IndexError()\n    except ValueError:\n        pass\n";
    let shadow = "def ValueError():\n    pass\n";
    for src in [format!("{handler}{shadow}"), format!("{shadow}{handler}")] {
        assert!(
            src_err(&src).contains("no longer names the builtin exception"),
            "{src}"
        );
    }
}

#[test]
fn heap_and_mutable_types_are_language_level() {
    let list = Type::List(Box::new(Type::Int));
    let tuple = |fields: Vec<Type>| Type::Tuple(fields);
    let opt = |t: Type| Type::Optional(Box::new(t));
    let cases = [
        (Type::Int, Some(false), Some(false)),
        (Type::Range, Some(false), Some(false)),
        (
            tuple(vec![Type::Int, Type::Float]),
            Some(false),
            Some(false),
        ),
        (opt(Type::Int), Some(false), Some(false)),
        (Type::Str, Some(true), Some(false)),
        (Type::Exception, Some(true), Some(false)),
        (tuple(vec![Type::Int, Type::Str]), Some(true), Some(false)),
        (list.clone(), Some(true), Some(true)),
        (tuple(vec![list.clone(), Type::Int]), Some(true), Some(true)),
        (opt(list.clone()), Some(true), Some(true)),
        (Type::Var(TypeVar(0)), None, None),
        (
            tuple(vec![Type::Var(TypeVar(0)), list]),
            Some(true),
            Some(true),
        ),
        (
            tuple(vec![Type::Var(TypeVar(0)), Type::Str]),
            Some(true),
            None,
        ),
    ];
    for (ty, heap, mutable) in cases {
        assert_eq!(ty.holds_heap(), heap, "{ty}");
        assert_eq!(ty.reaches_mutable(), mutable, "{ty}");
    }
}
