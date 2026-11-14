//! `key=` of `sorted`, `min` and `max` (step 6-1, D181-D185): a call with
//! `key=NAME` is rewritten once, here, into comprehensions over index
//! pairs, so inference, `raises`, `run` and `build` see only what they
//! already do. The key runs once per element, in order, before any
//! comparison; the index breaks ties, which makes `sorted` stable (also
//! with `reverse=True`) and has `min`/`max` take the first of equal keys:
//!
//! ```text
//! sorted(XS, key=F, reverse=R)
//!   [a[s*p[1]] for a in [[v for v in XS]] for s in [-1 if R else 1]
//!               for p in sorted([(F(x), s*i) for i, x in enumerate(a)],
//!                               reverse=s < 0)]
//! min(XS, key=F)   [a[m[1]] for a in [..] for m in [min([(F(x), i) ..])]][0]
//! max(XS, key=F)   the same with (F(x), -i) and a[-m[1]]
//! ```
//!
//! Inference keys its types by span, so every synthesized node gets a
//! span of its own, inside the call and equal to no span the source gave.

use std::collections::HashSet;

use super::error;
use crate::ast::{
    comp_name, Arg, BinOp, CmpOp, CompFor, Expr, ExprKind, Gather, Ident, Stmt, StmtKind, Target,
    TargetKind, UnaryOp,
};
use crate::diag::Result;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Whether the program defines its own `sorted`, `min` or `max` at the
/// top level, which then is no builtin to rewrite, wherever its `def`
/// stands (D115). A method of that name, inside a block, is no such `def`.
pub(super) fn defines_ordering(tokens: &[Token]) -> bool {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match &t.kind {
            TokenKind::Indent => depth += 1,
            TokenKind::Dedent => depth = depth.saturating_sub(1),
            TokenKind::Def if depth == 0 => {
                let named = tokens.get(i + 1).map(|n| &n.kind);
                if matches!(named, Some(TokenKind::Name(n)) if matches!(n.as_str(), "sorted" | "min" | "max"))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// `call`, or what it means if it is `sorted`/`min`/`max` with `key=`.
/// A call with other keywords, or the wrong number of arguments, is left
/// to inference to refuse.
pub(super) fn rewrite(call: Expr) -> Result<Expr> {
    let ExprKind::Call { func, args } = &call.kind else {
        return Ok(call);
    };
    let ExprKind::Name(name) = &func.kind else {
        return Ok(call);
    };
    let Some(key) = args.iter().find(|a| is_kw(a, "key")) else {
        return Ok(call);
    };
    let extra = if name == "sorted" { "reverse" } else { "" };
    let others = args.iter().filter(|a| {
        a.name
            .as_ref()
            .is_some_and(|k| k.name != "key" && k.name != extra)
    });
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let one = positional.len() == 1 || (name != "sorted" && !positional.is_empty());
    if !matches!(name.as_str(), "sorted" | "min" | "max") || !one || others.count() > 0 {
        return Ok(call);
    }
    let Some(f) = key_function(name, key)? else {
        let kept = args.iter().filter(|a| !is_kw(a, "key")).cloned().collect();
        return Ok(Expr {
            kind: ExprKind::Call {
                func: func.clone(),
                args: kept,
            },
            span: call.span,
        });
    };
    let reverse = args
        .iter()
        .find(|a| is_kw(a, "reverse"))
        .map(|a| a.value.clone());
    let xs: Vec<Expr> = positional.iter().map(|a| a.value.clone()).collect();
    let mut b = Build::new(&call, call.span);
    Ok(b.keyed(name, f, xs, reverse))
}

/// `xs.sort(key=F, reverse=R)` as a statement: the sorted copy first, so a
/// raising key leaves `xs` as it was, then each element stored back:
/// `for i, v in enumerate(sorted(xs, key=F, reverse=R)): xs[i] = v`.
pub(super) fn sort_stmt(lhs: Expr) -> Result<StmtKind> {
    let ExprKind::Call { func, args } = &lhs.kind else {
        return Ok(StmtKind::Expr(lhs));
    };
    let ExprKind::Attr { value, attr } = &func.kind else {
        return Ok(StmtKind::Expr(lhs));
    };
    let ExprKind::Name(xs) = &value.kind else {
        return Ok(StmtKind::Expr(lhs));
    };
    let keyed = args.iter().any(|a| is_kw(a, "key"));
    if attr.name != "sort" || !keyed || !args.iter().all(|a| a.name.is_some()) {
        return Ok(StmtKind::Expr(lhs));
    }
    let mut b = Build::new(&lhs, lhs.span);
    let (name, receiver) = (
        b.e(ExprKind::Name("sorted".to_string())),
        value.as_ref().clone(),
    );
    let mut all = vec![Build::arg(receiver)];
    all.extend(args.iter().cloned());
    let call = Expr {
        kind: ExprKind::Call {
            func: Box::new(name),
            args: all,
        },
        span: lhs.span,
    };
    let sorted = rewrite(call)?;
    if matches!(&sorted.kind, ExprKind::Call { .. }) {
        return Ok(StmtKind::Expr(lhs));
    }
    let mut b = Build::new(&sorted, lhs.span);
    let (ti, tv) = (b.target("ki"), b.target("kv"));
    let target = Target {
        span: b.span(),
        kind: TargetKind::Tuple(vec![ti, tv]),
    };
    let iter = b.call("enumerate", vec![sorted]);
    let slot = Target {
        span: b.span(),
        kind: TargetKind::Subscript {
            value: Box::new(Expr {
                kind: ExprKind::Name(xs.clone()),
                span: b.span(),
            }),
            index: Box::new(b.name("ki")),
        },
    };
    let store = Stmt {
        kind: StmtKind::Assign {
            target: slot,
            value: b.name("kv"),
        },
        span: lhs.span,
    };
    Ok(StmtKind::For {
        target,
        iter,
        body: vec![store],
        orelse: Vec::new(),
    })
}

fn is_kw(arg: &Arg, name: &str) -> bool {
    arg.name.as_ref().is_some_and(|k| k.name == name)
}

/// The function `key=` names: `None` means no key, anything but a name
/// is an error.
fn key_function(call: &str, key: &Arg) -> Result<Option<Expr>> {
    match &key.value.kind {
        ExprKind::None => Ok(None),
        ExprKind::Name(_) => Ok(Some(key.value.clone())),
        _ => {
            let msg = format!("`key=` of `{call}` takes only the name of a function yet");
            Err(error(key.value.span, msg))
        }
    }
}

/// Builds nodes at spans no other node has.
struct Build {
    used: HashSet<Span>,
    range: Span,
    next: (usize, usize),
    offset: usize,
}

impl Build {
    /// Spans inside `range`, the source call, that `e` does not use.
    fn new(e: &Expr, range: Span) -> Build {
        let mut used = HashSet::new();
        spans(e, &mut used);
        Build {
            used,
            range,
            next: (range.start, range.start),
            offset: range.start,
        }
    }

    /// The next span inside the call that nothing uses.
    fn span(&mut self) -> Span {
        loop {
            let (a, b) = self.next;
            self.next = if b < self.range.end {
                (a, b + 1)
            } else {
                (a + 1, a + 1)
            };
            let span = Span { start: a, end: b };
            if a > self.range.end {
                // The call is too short for more; share the last span.
                return self.range;
            }
            if self.used.insert(span) {
                return span;
            }
        }
    }

    fn e(&mut self, kind: ExprKind) -> Expr {
        let span = self.span();
        Expr { kind, span }
    }

    fn name(&mut self, n: &str) -> Expr {
        let name = comp_name(n, self.offset);
        self.e(ExprKind::Name(name))
    }

    fn int(&mut self, n: i64) -> Expr {
        self.e(ExprKind::Int(n))
    }

    fn target(&mut self, n: &str) -> Target {
        let span = self.span();
        Target {
            kind: TargetKind::Name(comp_name(n, self.offset)),
            span,
        }
    }

    fn list(&mut self, items: Vec<Expr>) -> Expr {
        self.e(ExprKind::List(items))
    }

    fn binary(&mut self, op: BinOp, left: Expr, right: Expr) -> Expr {
        let (left, right) = (Box::new(left), Box::new(right));
        self.e(ExprKind::Binary { op, left, right })
    }

    fn neg(&mut self, operand: Expr) -> Expr {
        let operand = Box::new(operand);
        self.e(ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        })
    }

    fn call(&mut self, func: &str, args: Vec<Expr>) -> Expr {
        let func = Box::new(self.e(ExprKind::Name(func.to_string())));
        let args = args.into_iter().map(Self::arg).collect();
        self.e(ExprKind::Call { func, args })
    }

    fn arg(value: Expr) -> Arg {
        Arg {
            name: None,
            span: value.span,
            value,
        }
    }

    fn index(&mut self, value: Expr, index: Expr) -> Expr {
        let (value, index) = (Box::new(value), Box::new(index));
        self.e(ExprKind::Subscript { value, index })
    }

    /// `[gather for target in iter]` clauses, one `for` each.
    fn comp(&mut self, gather: Expr, clauses: Vec<(Target, Expr)>) -> Expr {
        let clauses = clauses
            .into_iter()
            .map(|(target, iter)| CompFor {
                span: iter.span,
                target,
                iter,
                ifs: Vec::new(),
            })
            .collect();
        let gather = Gather::List(Box::new(gather));
        self.e(ExprKind::Comp { gather, clauses })
    }

    /// `[v for v in xs]`: a list of what `xs` iterates over.
    fn copy(&mut self, xs: Vec<Expr>) -> Expr {
        let xs = match <[Expr; 1]>::try_from(xs) {
            Ok([one]) => one,
            Err(many) => self.list(many),
        };
        let (v, t) = (self.name("v"), self.target("v"));
        self.comp(v, vec![(t, xs)])
    }

    /// `[(F(x), j) for i, x in enumerate(a)]`, `j` being `sign`'s index.
    fn pairs(&mut self, f: Expr, sign: Sign) -> Expr {
        let x = self.name("x");
        let key = self.e(ExprKind::Call {
            func: Box::new(f),
            args: vec![Self::arg(x)],
        });
        let i = self.name("i");
        let j = match sign {
            Sign::Plus => i,
            Sign::Minus => self.neg(i),
            Sign::Var => {
                let s = self.name("s");
                self.binary(BinOp::Mul, s, i)
            }
        };
        let pair = self.e(ExprKind::Tuple(vec![key, j]));
        let a = self.name("a");
        let iter = self.call("enumerate", vec![a]);
        let (ti, tx) = (self.target("i"), self.target("x"));
        let tuple = Target {
            span: self.span(),
            kind: TargetKind::Tuple(vec![ti, tx]),
        };
        self.comp(pair, vec![(tuple, iter)])
    }

    fn keyed(&mut self, which: &str, f: Expr, xs: Vec<Expr>, reverse: Option<Expr>) -> Expr {
        let (ta, copy) = (self.target("a"), self.copy(xs));
        let own = self.list(vec![copy]);
        if which != "sorted" {
            return self.extreme(which, f, (ta, own));
        }
        let (ts, tp) = (self.target("s"), self.target("p"));
        let (sign, order) = match reverse {
            Some(r) => (self.sign(r), Some(self.descending())),
            None => (self.int(1), None),
        };
        let signs = self.list(vec![sign]);
        let pairs = self.pairs(f, Sign::Var);
        let mut args = vec![Self::arg(pairs)];
        if let Some(order) = order {
            args.push(Arg {
                name: Some(Ident {
                    name: "reverse".to_string(),
                    span: order.span,
                }),
                span: order.span,
                value: order,
            });
        }
        let func = Box::new(self.e(ExprKind::Name("sorted".to_string())));
        let sorted = self.e(ExprKind::Call { func, args });
        let (a, s, p) = (self.name("a"), self.name("s"), self.name("p"));
        let one = self.int(1);
        let second = self.index(p, one);
        let at = self.binary(BinOp::Mul, s, second);
        let pick = self.index(a, at);
        self.comp(pick, vec![(ta, own), (ts, signs), (tp, sorted)])
    }

    /// `-1 if R else 1`: the sign on the index of a reversed sort.
    fn sign(&mut self, reverse: Expr) -> Expr {
        let (body, orelse) = (self.int(-1), self.int(1));
        self.e(ExprKind::IfExp {
            test: Box::new(reverse),
            body: Box::new(body),
            orelse: Box::new(orelse),
        })
    }

    /// `s < 0`.
    fn descending(&mut self) -> Expr {
        let (s, zero) = (self.name("s"), self.int(0));
        self.e(ExprKind::Compare {
            left: Box::new(s),
            rest: vec![(CmpOp::Lt, zero)],
        })
    }

    /// `min`/`max`: the call over the pairs, then the element at the
    /// winning index.
    fn extreme(&mut self, which: &str, f: Expr, a: (Target, Expr)) -> Expr {
        let sign = if which == "min" {
            Sign::Plus
        } else {
            Sign::Minus
        };
        let pairs = self.pairs(f, sign);
        let best = self.call(which, vec![pairs]);
        let boxed = self.list(vec![best]);
        let tm = self.target("m");
        let (arr, m) = (self.name("a"), self.name("m"));
        let one = self.int(1);
        let second = self.index(m, one);
        let at = if which == "min" {
            second
        } else {
            self.neg(second)
        };
        let pick = self.index(arr, at);
        let all = self.comp(pick, vec![a, (tm, boxed)]);
        let zero = self.int(0);
        self.index(all, zero)
    }
}

/// Which index a key pair carries: `i`, `-i` or `s*i`.
#[derive(Clone, Copy)]
enum Sign {
    Plus,
    Minus,
    Var,
}

/// Every span of `e`'s expressions, targets and names.
fn spans(e: &Expr, out: &mut HashSet<Span>) {
    out.insert(e.span);
    let mut go = |x: &Expr| spans(x, out);
    match &e.kind {
        ExprKind::Name(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => {}
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Set(xs) => xs.iter().for_each(go),
        ExprKind::Dict(items) => items.iter().for_each(|(k, v)| {
            go(k);
            go(v);
        }),
        ExprKind::Unary { operand, .. } => go(operand),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            go(left);
            go(right);
        }
        ExprKind::Compare { left, rest } => {
            go(left);
            rest.iter().for_each(|(_, x)| go(x));
        }
        ExprKind::IfExp { test, body, orelse } => {
            [test, body, orelse].into_iter().for_each(|x| go(x))
        }
        ExprKind::Call { func, args } => {
            go(func);
            args.iter().for_each(|a| go(&a.value));
        }
        ExprKind::Subscript { value, index } => {
            go(value);
            go(index);
        }
        ExprKind::Slice {
            value,
            start,
            stop,
            step,
        } => {
            go(value);
            [start, stop, step]
                .into_iter()
                .flatten()
                .for_each(|x| go(x));
        }
        ExprKind::Attr { value, attr } => {
            go(value);
            out.insert(attr.span);
        }
        ExprKind::Comp { gather, clauses } => {
            gather.exprs().into_iter().for_each(go);
            for c in clauses {
                target_spans(&c.target, out);
                spans(&c.iter, out);
                c.ifs.iter().for_each(|x| spans(x, out));
            }
        }
    }
}

fn target_spans(t: &Target, out: &mut HashSet<Span>) {
    out.insert(t.span);
    if let TargetKind::Tuple(items) = &t.kind {
        items.iter().for_each(|x| target_spans(x, out));
    }
}
