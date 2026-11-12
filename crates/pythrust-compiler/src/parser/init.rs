//! A plain class's fields and the D145 check of its `__init__`: the
//! fields are the names `__init__` stores through `self`, in first-store
//! order (D144), and every path assigns each before `self` is used or
//! `__init__` ends (D145).

use std::collections::BTreeSet;

use super::class::field;
use super::error;
use crate::ast::{
    Expr, ExprKind, Field, FuncDef, Ident, Stmt, StmtKind, Target, TargetKind, TypeExpr,
};
use crate::diag::Result;
use crate::infer::scope;
use crate::span::Span;

/// The fields assigned so far on a path; `None` once the path has ended.
type Flow = Option<BTreeSet<String>>;

/// Class `class`'s fields, from its `__init__` `def`, checked by D145.
pub(super) fn fields(
    class: &Ident,
    def: &FuncDef,
    annotations: &[(Ident, TypeExpr)],
) -> Result<Vec<Field>> {
    check_annotations(annotations)?;
    let names = stored(&def.body);
    let init = Init {
        class: &class.name,
        fields: names.iter().map(|n| n.name.clone()).collect(),
    };
    if let Some(done) = init.block(&def.body, Some(BTreeSet::new()))? {
        init.complete(&done, def.span)?;
    }
    Ok(names.iter().map(|n| field(n, annotations)).collect())
}

/// A field is annotated at most once (`self.x: T = v`).
fn check_annotations(annotations: &[(Ident, TypeExpr)]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for (name, _) in annotations {
        if !seen.insert(name.name.as_str()) {
            let msg = format!(
                "field `{}` is annotated twice; annotate its first store only",
                name.name
            );
            return Err(error(name.span, msg));
        }
    }
    Ok(())
}

/// The fields `body` stores through `self`, each at its first store.
fn stored(body: &[Stmt]) -> Vec<Ident> {
    let mut out: Vec<Ident> = Vec::new();
    scope::walk(body, &mut |s| {
        let (StmtKind::Assign { target, .. } | StmtKind::AugAssign { target, .. }) = &s.kind else {
            return;
        };
        for f in target_fields(target) {
            if !out.iter().any(|o| o.name == f.name) {
                out.push(f.clone());
            }
        }
    });
    out
}

/// The `self.x` stores in `target`, through any unpacking.
fn target_fields(target: &Target) -> Vec<&Ident> {
    match &target.kind {
        TargetKind::Attr { value, attr } if is_self(value) => vec![attr],
        TargetKind::Tuple(items) => items.iter().flat_map(target_fields).collect(),
        _ => Vec::new(),
    }
}

fn is_self(e: &Expr) -> bool {
    matches!(&e.kind, ExprKind::Name(n) if n == "self")
}

/// One class's `__init__` under check.
struct Init<'a> {
    class: &'a str,
    fields: BTreeSet<String>,
}

impl Init<'_> {
    fn block(&self, block: &[Stmt], mut flow: Flow) -> Result<Flow> {
        for s in block {
            let Some(done) = flow else { return Ok(None) };
            flow = self.stmt(s, done)?;
        }
        Ok(flow)
    }

    fn stmt(&self, s: &Stmt, mut done: BTreeSet<String>) -> Result<Flow> {
        match &s.kind {
            StmtKind::Assign { target, value } => {
                self.uses(value, &done)?;
                self.target(target, &done)?;
                done.extend(target_fields(target).into_iter().map(|f| f.name.clone()));
                Ok(Some(done))
            }
            StmtKind::AugAssign { target, value, .. } => {
                self.uses(value, &done)?;
                self.target(target, &done)?;
                if let Some(f) = target_fields(target).first() {
                    self.read(f, &done)?;
                }
                Ok(Some(done))
            }
            StmtKind::Return(value) => {
                value.iter().try_for_each(|v| self.uses(v, &done))?;
                self.complete(&done, s.span)?;
                Ok(None)
            }
            StmtKind::Raise(value) => {
                self.uses(value, &done)?;
                Ok(None)
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                let branches = std::iter::once(branch).chain(elifs);
                let mut outs = Vec::new();
                for b in branches {
                    self.uses(&b.test, &done)?;
                    outs.push(self.block(&b.body, Some(done.clone()))?);
                }
                outs.push(self.block(orelse, Some(done))?);
                Ok(meet(outs))
            }
            StmtKind::While { branch, orelse } => {
                self.uses(&branch.test, &done)?;
                self.block(&branch.body, Some(done.clone()))?;
                self.block(orelse, Some(done))
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                self.uses(iter, &done)?;
                self.target(target, &done)?;
                self.block(body, Some(done.clone()))?;
                self.block(orelse, Some(done))
            }
            StmtKind::Try {
                body,
                handlers,
                finally,
            } => {
                let mut outs = vec![self.block(body, Some(done.clone()))?];
                for h in handlers {
                    outs.push(self.block(&h.body, Some(done.clone()))?);
                }
                let joined = meet(outs);
                let Some(fin) = finally else {
                    return Ok(joined);
                };
                let ended = joined.is_none();
                let after = self.block(fin, Some(joined.unwrap_or(done)))?;
                Ok(after.filter(|_| !ended))
            }
            StmtKind::With { value, body, .. } => {
                self.uses(value, &done)?;
                self.block(body, Some(done))
            }
            _ => {
                for e in scope::own_exprs(s) {
                    self.uses(e, &done)?;
                }
                Ok(Some(done))
            }
        }
    }

    /// The expressions in a store's target other than its `self.x` parts.
    fn target(&self, target: &Target, done: &BTreeSet<String>) -> Result<()> {
        match &target.kind {
            TargetKind::Name(_) => Ok(()),
            TargetKind::Attr { value, .. } if is_self(value) => Ok(()),
            TargetKind::Attr { value, .. } => self.uses(value, done),
            TargetKind::Subscript { value, index } => {
                self.uses(value, done)?;
                self.uses(index, done)
            }
            TargetKind::Tuple(items) => items.iter().try_for_each(|t| self.target(t, done)),
        }
    }

    /// `e` reads no unassigned field and uses `self` only for an
    /// assigned field, until every field is assigned.
    fn uses(&self, e: &Expr, done: &BTreeSet<String>) -> Result<()> {
        if self.fields.is_subset(done) {
            return Ok(());
        }
        let mut reads: Vec<Span> = Vec::new();
        for x in scope::tree(e) {
            match &x.kind {
                ExprKind::Attr { value, attr } if is_self(value) => {
                    reads.push(value.span);
                    self.read(attr, done)?;
                }
                ExprKind::Name(n) if n == "self" && !reads.contains(&x.span) => {
                    return Err(self.early(x.span));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `self.<attr>` is a read of an assigned field.
    fn read(&self, attr: &Ident, done: &BTreeSet<String>) -> Result<()> {
        if done.contains(&attr.name) {
            return Ok(());
        }
        if !self.fields.contains(&attr.name) {
            return Err(self.early(attr.span));
        }
        let msg = format!(
            "field `{}` of `{}` is read before it is assigned",
            attr.name, self.class
        );
        Err(error(attr.span, msg))
    }

    fn early(&self, span: Span) -> crate::diag::Diagnostic {
        let msg = format!(
            "`self` is used before every field of `{}` is assigned",
            self.class
        );
        error(span, msg)
    }

    /// `__init__` ends with every field assigned.
    fn complete(&self, done: &BTreeSet<String>, span: Span) -> Result<()> {
        let Some(missing) = self.fields.difference(done).next() else {
            return Ok(());
        };
        let msg = format!(
            "field `{missing}` of `{}` is not assigned on every path through `__init__`",
            self.class
        );
        Err(error(span, msg))
    }
}

/// The fields assigned on every path that goes on; `None` when none does.
fn meet(outs: Vec<Flow>) -> Flow {
    outs.into_iter().flatten().reduce(|a, b| &a & &b)
}
