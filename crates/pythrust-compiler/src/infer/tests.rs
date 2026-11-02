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
        stubs: Vec::new(),
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
    let msg = err(vec![b.ann("x", b.ty("frozenset", vec![]), b.i(1))]);
    assert_eq!(msg, "unknown type `frozenset`");
    let bad = b.ty("dict", vec![b.ty("int", vec![])]);
    let msg = err(vec![b.ann("x", bad, b.i(1))]);
    assert!(msg.starts_with("`dict` takes two type arguments"), "{msg}");
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
            "`list` copies a list, dict, set, dict view or `map`'s results, not str",
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
        "KeyError",
    ] {
        assert!(infer_src(&handler(name)).is_ok(), "{name}");
    }
    assert!(src_err(&handler("TypeError")).starts_with("`TypeError` is not an exception"));
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
    assert!(
        src_err(&handler("ValueError, TypeError")).starts_with("`TypeError` is not an exception")
    );
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
        (
            Type::Dict(Box::new(Type::Str), Box::new(Type::Int)),
            Some(true),
            Some(true),
        ),
        (Type::Set(Box::new(Type::Int)), Some(true), Some(true)),
        (opt(Type::Set(Box::new(Type::Str))), Some(true), Some(true)),
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

// ---- dicts and sets (M6) ---------------------------------------------------

/// The return type `src` infers for its function `f`.
fn ret_of(src: &str) -> String {
    let types = infer_src(src).unwrap_or_else(|e| panic!("{e}\n{src}"));
    let sig = types.func("f").unwrap_or_else(|| panic!("no `f`"));
    sig.ret.to_string()
}

#[test]
fn dict_and_set_displays_infer_their_types() {
    let ret = |body: &str| ret_of(&format!("def f():\n    return {body}\n"));
    assert_eq!(ret("{\"a\": 1}"), "dict[str, int]");
    assert_eq!(
        ret("{(1, \"a\"): [1.5]}"),
        "dict[tuple[int, str], list[float]]"
    );
    assert_eq!(ret("{1, 2}"), "set[int]");
    assert_eq!(ret("set(\"ab\")"), "set[str]");
    assert_eq!(ret("set(range(3))"), "set[int]");
    assert_eq!(ret("{1: 2}.keys()"), "dict_keys[int]");
    assert_eq!(ret("{1: \"a\"}.values()"), "dict_values[str]");
    assert_eq!(ret("{1: \"a\"}.items()"), "dict_items[int, str]");
    assert_eq!(ret("list({1: \"a\"}.items())"), "list[tuple[int, str]]");
    assert_eq!(ret("list({\"k\": 1})"), "list[str]");
    assert_eq!(ret("sorted({2, 1})"), "list[int]");
    assert_eq!(ret("sorted({1: 2}.values())"), "list[int]");
    assert_eq!(ret("{1: \"a\"}.get(1)"), "str | None");
    assert_eq!(ret("{1: \"a\"}.get(1, \"z\")"), "str");
    assert_eq!(ret("len({1: 2}) + len({3})"), "int");
    assert_eq!(ret("2 in {1: 2}"), "bool");
}

#[test]
fn an_empty_display_takes_its_types_from_use() {
    let src = "def f():\n    d = {}\n    d[\"k\"] = [1]\n    return d\n";
    assert_eq!(ret_of(src), "dict[str, list[int]]");
    let src = "def f():\n    s = set()\n    s.add((1, None))\n    return s\n";
    assert_eq!(ret_of(src), "set[tuple[int, None]]");
    let src = "def f():\n    d: dict[str, int | None] = {\"a\": None}\n    return d\n";
    assert_eq!(ret_of(src), "dict[str, int | None]");
}

#[test]
fn get_of_an_optional_value_stays_optional() {
    let src = "def f():\n    d: dict[int, str | None] = {}\n    return d.get(1)\n";
    assert_eq!(ret_of(src), "str | None");
    let src = "def f():\n    d: dict[int, None] = {}\n    return d.get(1)\n";
    assert_eq!(ret_of(src), "None");
}

#[test]
fn generic_functions_take_dicts() {
    let src = "def first(d, k):\n    return d[k]\n\
               def f():\n    return (first({1: \"a\"}, 1), first({\"x\": 2.5}, \"x\"))\n";
    assert_eq!(ret_of(src), "tuple[str, float]");
}

#[test]
fn keys_and_elements_must_be_hashable() {
    let msg = |src: &str| src_err(src);
    let kinds = "a key or element is int, str, bool, None or a tuple of them";
    let unhashable = |t: &str| format!("unhashable type: {t}; {kinds}");
    let later = |t: &str| format!("{t} dict keys and set elements are not supported yet; {kinds}");
    assert_eq!(msg("d = {[1]: 2}\n"), unhashable("list[int]"));
    assert_eq!(msg("s = {(1, [2])}\n"), unhashable("list[int]"));
    assert_eq!(msg("s = {{1: 2}}\n"), unhashable("dict[int, int]"));
    assert_eq!(msg("s = set([[1]])\n"), unhashable("list[int]"));
    assert_eq!(msg("x: set[set[int]] = set()\n"), unhashable("set[int]"));
    assert_eq!(msg("s = {{1: 2}.keys()}\n"), unhashable("dict_keys[int]"));
    assert_eq!(msg("d = {1.5: 2}\n"), later("float"));
    assert_eq!(msg("d = {(1, 2.5): 2}\n"), later("float"));
    assert_eq!(msg("s = set()\ns.add(1.0)\n"), later("float"));
    assert_eq!(msg("def f(d):\n    d[0.5] = 1\nf({})\n"), later("float"));
    assert_eq!(msg("s = {ValueError()}\n"), later("Exception"));
    assert_eq!(
        msg("x: dict[int | None, tuple[str, bool]] = {}\n"),
        "no error"
    );
}

#[test]
fn dict_and_set_misuse_is_an_error() {
    let cases = [
        ("d = {1: 2, \"a\": 3}\n", "dict keys must share one type"),
        ("d = {1: 2, 3: \"a\"}\n", "dict values must share one type"),
        ("s = {1, \"a\"}\n", "set elements must share one type"),
        (
            "d = {1: 2}\nd[\"a\"] = 3\n",
            "dict key or set element: expected int, found str",
        ),
        (
            "d = {1: 2}\nprint(d[\"a\"])\n",
            "dict key or set element: expected int, found str",
        ),
        (
            "d = {1: 2}\nd[1] = \"x\"\n",
            "cannot assign str into dict[int, int]",
        ),
        ("s = {1}\nprint(s[0])\n", "set[int] is not subscriptable"),
        ("s = {1}\ns[0] = 1\n", "set[int] is not subscriptable"),
        ("d = {1: 2}\nd.add(1)\n", "`add` needs a set"),
        ("s = {1}\ns.get(1)\n", "`get` needs a dict"),
        ("s = [1]\ns.keys()\n", "`keys` needs a dict"),
        (
            "s = {1}\ns.add(\"a\")\n",
            "dict key or set element: expected int, found str",
        ),
        (
            "d = {1: 2}\nd.get()\n",
            "`get` takes 1 or 2 arguments but 0 were given",
        ),
        (
            "d = {1: 2}\nd.keys(1)\n",
            "`keys` takes no arguments but 1 were given",
        ),
        ("s = {1}\ns.add(x=1)\n", "`add` takes no keyword arguments"),
        (
            "d = {1: 2}\nd.get(1, \"a\")\n",
            "the default of `get` must match",
        ),
        ("d = {1: 2}\nd.pop(1)\n", "unknown method `.pop`"),
        ("print({1: 2} < {1: 2})\n", "cannot order dict[int, int]"),
        (
            "print({1} == [1])\n",
            "cannot compare set[int] with list[int]",
        ),
        (
            "print({1: 2} + {3: 4})\n",
            "unsupported operand types for +",
        ),
        ("x: dict[int] = {}\n", "`dict` takes two type arguments"),
        (
            "x: set[int, int] = set()\n",
            "`set` takes one type argument",
        ),
        ("x = set(1)\n", "int is not iterable"),
        (
            "x = set(1, 2)\n",
            "`set` takes 0 to 1 arguments but 2 were given",
        ),
        (
            "x = {1: 2}.keys()\nprint(x == x)\n",
            "dict_keys[int] cannot be compared",
        ),
        (
            "print({1: 2}.items()[0])\n",
            "dict_items[int, int] is not subscriptable",
        ),
        (
            "a, b = {1: 2, 3: 4}.keys()\n",
            "cannot unpack dict_keys[int]",
        ),
        (
            "print({1: 2}.values() + [1])\n",
            "unsupported operand types for +",
        ),
        (
            "print(list(\"ab\"))\n",
            "`list` copies a list, dict, set, dict view or `map`'s results, not str",
        ),
        (
            "print(sorted([{1}]))\n",
            "cannot order set[int] against set[int]",
        ),
        (
            "print(sorted([1], reverse=True))\n",
            "`sorted` takes no keyword arguments",
        ),
        (
            "d = {1: \"a\"}\nprint(d.get(\"x\"))\n",
            "dict key or set element: expected int, found str",
        ),
    ];
    for (src, want) in cases {
        let got = src_err(src);
        assert!(got.contains(want), "{src:?}: {got}");
    }
}

#[test]
fn optional_dict_and_set_literals_check_against_the_present_type() {
    let src = "x: dict[str, int] | None = {\"a\": 1}\ny: set[int | None] | None = {None}\n";
    assert!(infer_src(src).is_ok(), "{}", src_err(src));
}

#[test]
fn a_dict_or_set_probe_must_be_hashable() {
    let why = "float dict keys and set elements are not supported yet";
    assert!(src_err("print(1.0 in {1})\n").contains(why));
    assert!(src_err("print(1.0 not in {1: 2})\n").contains(why));
    assert!(src_err("print(1.0 in {1: 2}.keys())\n").contains(why));
    assert_eq!(src_err("print(1.0 in {1: 2}.values())\n"), "no error");
    assert_eq!(
        src_err("print(True in {1}, 1.0 in [1], 1 in {1: 2})\n"),
        "no error"
    );
}

#[test]
fn get_without_a_default_never_drops_its_none() {
    let msg = "`get` without a default may return None, but int is expected";
    assert!(src_err("d = {}\nx: int = d.get(1)\n").starts_with(msg));
    assert!(src_err("d = {}\nx = d.get(1)\nx = 5\n").starts_with(msg));
    assert_eq!(
        src_err("d = {}\nx: int | None = d.get(1)\nprint(x)\n"),
        "no error"
    );
    assert_eq!(src_err("d = {}\nprint(d.get(1))\n"), "no error");
}

#[test]
fn get_keeps_its_none_through_generic_and_typed_returns() {
    let msg = "`get` without a default may return None, but int is expected";
    let typed = "def f(d) -> int:\n    return d.get(1)\nprint(f({}))\n";
    assert!(src_err(typed).contains(msg), "{}", src_err(typed));
    let generic = "def f(d):\n    return d.get(1)\nx: int = f({})\n";
    assert!(src_err(generic).contains(msg), "{}", src_err(generic));
    let optional = "def f(d) -> int | None:\n    return d.get(1)\nprint(f({}))\n";
    assert_eq!(src_err(optional), "no error");
}

#[test]
fn a_bool_key_fits_an_int_key_in_displays_and_generic_helpers() {
    let display = "d: dict[int, int] = {True: 7}\nprint(d[1])\n";
    assert_eq!(src_err(display), "no error");
    let get = "def f(d):\n    return d.get(True)\nprint(f({1: 7}))\n";
    assert_eq!(src_err(get), "no error");
    let index = "def f(d):\n    return d[True]\nprint(f({1: 7}))\n";
    assert_eq!(src_err(index), "no error");
    let unused = "d = {}\nprint(d.get(True))\n";
    assert_eq!(src_err(unused), "no error");
    let probe = "def f(d, k):\n    return d.get(k)\nprint(f({1: 2}, True), f({\"a\": 1}, \"a\"))\n";
    assert_eq!(src_err(probe), "no error");
    let generic = "def f(d, k):\n    return d[k]\nprint(f({1: 2}, True), f({\"a\": 1}, \"a\"))\n";
    assert_eq!(src_err(generic), "no error");
    let bare = "def f(d: dict, k):\n    return d[k]\nprint(f({1: 2}, True))\n";
    assert_eq!(src_err(bare), "no error");
    let typed = "def f(d: dict[int, int], k):\n    return d[k]\nprint(f({1: 2}, \"a\"))\n";
    assert!(src_err(typed).contains("dict key"), "{}", src_err(typed));
    let wrong = "d: dict[str, int] = {True: 7}\n";
    assert!(src_err(wrong).contains("dict key"), "{}", src_err(wrong));
    // A set element is a key too (Nano, 2026-10-31); an int never fits a
    // bool key.
    let set = "s: set[int] = {True, 2}\ns.add(False)\nt = {1}\nt.add(True)\nprint(s, t)\n";
    assert_eq!(src_err(set), "no error");
    for narrow in [
        "d = {True: 1}\nd[1] = 2\n",
        "s = {True}\ns.add(1)\n",
        "s: set[bool] = {1}\n",
    ] {
        assert!(
            src_err(narrow).contains("expected bool, found int"),
            "{narrow}: {}",
            src_err(narrow)
        );
    }
}

#[test]
fn a_view_inside_a_compared_value_is_refused() {
    let why = "dict_keys[int] cannot be compared";
    for cmp in [
        "[v] == [v]",
        "(v, 1) != (v, 1)",
        "{1: v} == {1: v}",
        "[v] in [[v]]",
    ] {
        let src = format!("v = {{1: 2}}.keys()\nprint({cmp})\n");
        assert!(src_err(&src).contains(why), "{cmp}: {}", src_err(&src));
    }
    let later = "xs = []\nprint(xs == xs)\nxs.append({1: 2}.keys())\n";
    assert!(src_err(later).contains(why), "{}", src_err(later));
    let fine = "print([[1]] == [[1]], ({1: 2}, 1) == ({1: 2}, 1), [1] in [[1]])\n";
    assert_eq!(src_err(fine), "no error");
}

#[test]
fn listdir_takes_its_path_by_keyword_and_nothing_else_does() {
    let ok = "import os\nprint(os.listdir(path=\".\"), os.listdir(\".\"), os.listdir())\n";
    assert_eq!(src_err(ok), "no error");
    let cases = [
        (
            "import os\nos.listdir(dir=\".\")\n",
            "`os.listdir` got an unexpected keyword argument `dir`",
        ),
        (
            "import os\nos.listdir(\".\", path=\".\")\n",
            "`os.listdir` takes 0 to 1 arguments but 2 were given",
        ),
        (
            "import os\nos.path.isdir(s=\".\")\n",
            "`os.path.isdir` got an unexpected keyword argument `s`",
        ),
        (
            "import os\nos.listdir(path=1)\n",
            "argument of `os.listdir`",
        ),
    ];
    for (src, want) in cases {
        let got = src_err(src);
        assert!(got.contains(want), "{src:?}: {got}");
    }
}

#[test]
fn files_read_bytes_that_size_and_compare() {
    let ok = "f = open(\"p\", \"rb\")\ndata: bytes = f.read()\nf.close()\n\
              print(len(data), data == f.read(), data != data, str(data))\nif data:\n    print(f)\n";
    assert_eq!(src_err(ok), "no error");
    let cases = [
        ("f = open(\"p\")\n", "only binary reading is supported yet"),
        (
            "f = open(\"p\", \"r\")\n",
            "only binary reading is supported yet",
        ),
        (
            "m = \"rb\"\nf = open(\"p\", m)\n",
            "only binary reading is supported yet",
        ),
        ("f = open(1, \"rb\")\n", "argument of `open`"),
        (
            "f = open(\"p\", \"rb\", 1)\n",
            "`open` takes 1 to 2 arguments",
        ),
        ("x = [1]\nx.read()\n", "`read` needs a file from `open`"),
        (
            "f = open(\"p\", \"rb\")\nf.read(10)\n",
            "`read` takes no arguments",
        ),
        (
            "f = open(\"p\", \"rb\")\nprint(f == f)\n",
            "files compare by identity",
        ),
        (
            "f = open(\"p\", \"rb\")\nprint(f.read() < f.read())\n",
            "cannot order bytes",
        ),
        (
            "f = open(\"p\", \"rb\")\nd = {f.read(): 1}\n",
            "bytes dict keys and set elements",
        ),
        (
            "f = open(\"p\", \"rb\")\nprint(f.read() + \"a\")\n",
            "unsupported operand types for +",
        ),
        ("x: bytes = \"a\"\n", "expected bytes, found str"),
        (
            "import hashlib\nprint(hashlib.sha256(\"a\").hexdigest())\n",
            "argument of `hashlib.sha256`",
        ),
    ];
    for (src, want) in cases {
        let got = src_err(src);
        assert!(got.contains(want), "{src:?}: {got}");
    }
}

// ---- Rust binding stubs (M6b) ----------------------------------------------

const FASTFS: &str = "\
temper def file_xxh3(path: str) -> str raises OSError: ...
temper def add(a: int, b: int) -> int: ...
temper def any_error() -> None raises Exception: ...
";

/// Infers `src` with the binding `fastfs` of [`FASTFS`].
fn with_fastfs(src: &str) -> Result<Types, String> {
    let binding = crate::parser::test_binding("fastfs", FASTFS);
    let parsed = crate::lexer::lex(src).and_then(|t| crate::parser::parse_with(&t, &[binding]));
    let module = parsed.map_err(|e| format!("does not parse: {}", e.message))?;
    infer(&module).map_err(|e| e.message)
}

#[test]
fn calls_check_against_the_stub() {
    let src = "import fastfs\nh = fastfs.file_xxh3(\"p\")\nn = fastfs.add(b=2, a=1)\n";
    let types = with_fastfs(src).unwrap_or_else(|e| panic!("{e}"));
    let sig = types
        .func("fastfs.add")
        .map(|f| (f.params.len(), f.ret.clone()));
    assert_eq!(sig, Some((2, Type::Int)));
    assert_eq!(
        types.func("fastfs.file_xxh3").map(|f| f.ret.clone()),
        Some(Type::Str)
    );
    let err = |src: &str| with_fastfs(src).err().unwrap_or_default();
    assert_eq!(
        err("import fastfs\nfastfs.add(1, \"a\")\n"),
        "argument `b` of `fastfs.add`: expected int, found str"
    );
    assert_eq!(
        err("import fastfs\nfastfs.add(1)\n"),
        "`fastfs.add` is missing argument `b`"
    );
    assert!(err("import fastfs\nx: str = fastfs.add(1, 2)\n").contains("expected str, found int"));
}

#[test]
fn a_binding_function_is_not_a_task_yet() {
    // Nano, 2026-11-02: not until an app needs it.
    let src = "import fastfs\nimport concurrent.futures\ndef main() -> None:\n    \
               with concurrent.futures.ThreadPoolExecutor() as ex:\n        \
               print(ex.submit(fastfs.add, 1, 2).result())\n        \
               print(list(ex.map(fastfs.file_xxh3, [\"p\"])))\nmain()\n";
    let got = with_fastfs(src).err().unwrap_or_default();
    assert!(
        got.contains("cannot run the Rust binding function `fastfs.add`"),
        "{got}"
    );
    let mapped = src.replace("print(ex.submit(fastfs.add, 1, 2).result())\n        ", "");
    let got = with_fastfs(&mapped).err().unwrap_or_default();
    assert!(
        got.contains("`map` cannot run the Rust binding function `fastfs.file_xxh3`"),
        "{got}"
    );
}

#[test]
fn a_stub_raises_what_it_declares() {
    let types = with_fastfs("import fastfs\nfastfs.add(1, 2)\n").unwrap_or_else(|e| panic!("{e}"));
    let escapes = |n: &str| {
        types
            .raises(n)
            .map(|r| r.escapes().map(|e| e.name()).collect::<Vec<_>>())
    };
    assert_eq!(escapes("fastfs.file_xxh3"), Some(vec!["OSError"]));
    assert_eq!(escapes("fastfs.add"), Some(vec![]));
    assert_eq!(escapes("fastfs.any_error"), Some(vec!["Exception"]));
    // A tempered caller must declare what a stub raises, and need not
    // declare what one does not.
    let quiet = "import fastfs\ntemper def f() -> int:\n    return fastfs.add(1, 2)\n";
    assert!(with_fastfs(quiet).is_ok());
    let loud = "import fastfs\ntemper def f() -> str:\n    return fastfs.file_xxh3(\"p\")\n";
    let err = with_fastfs(loud).err().unwrap_or_default();
    assert!(
        err.contains("must declare it; write `-> str raises OSError`"),
        "{err}"
    );
    let caught = "import fastfs\ntemper def f() -> str:\n    try:\n        return fastfs.file_xxh3(\"p\")\n    except OSError:\n        return \"\"\n";
    assert!(with_fastfs(caught).is_ok());
    // A loose caller's inferred set includes it.
    let loose = with_fastfs("import fastfs\ndef g():\n    fastfs.any_error()\n");
    let set = loose.map(|t| t.raises("g").map(|r| r.escapes().count()));
    assert_eq!(set, Ok(Some(1)));
}

/// A stub's `OSError` is the builtin whatever the program defines; a
/// call to it where the program redefines the class is refused at the
/// call, in the program, and a program that never calls it checks.
#[test]
fn a_redefined_class_refuses_calls_to_stubs_that_raise_it() {
    for redefine in ["OSError = 1\n", "def OSError() -> int:\n    return 1\n"] {
        assert!(with_fastfs(&format!("import fastfs\n{redefine}")).is_ok());
        assert!(with_fastfs(&format!("import fastfs\n{redefine}fastfs.add(1, 2)\n")).is_ok());
        let call = format!("import fastfs\n{redefine}print(fastfs.file_xxh3(\"x\"))\n");
        let binding = crate::parser::test_binding("fastfs", FASTFS);
        let tokens = crate::lexer::lex(&call).unwrap_or_else(|d| panic!("{d}"));
        let parsed = crate::parser::parse_with(&tokens, &[binding]);
        let module = parsed.unwrap_or_else(|d| panic!("{d}"));
        let Err(err) = infer(&module) else {
            panic!("{call:?} checked");
        };
        assert_eq!(&call[err.span.start..err.span.end], "fastfs.file_xxh3");
        assert!(
            err.message.contains("which this program redefines"),
            "{}",
            err.message
        );
    }
}
