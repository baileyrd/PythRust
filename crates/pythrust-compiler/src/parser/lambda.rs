//! `lambda` (step 6-2b, D194, D195): each lambda is lifted to a hidden
//! top-level def, so inference, ownership, `raises`, `run` and lowering
//! see an ordinary def. A lambda is its own name where it was written.
//!
//! A lambda reads its parameters, names bound in its body and
//! module-level names, and names of its enclosing function, which
//! `closure` turns into leading parameters (step 6-3a); a name of an
//! enclosing comprehension or lambda is the named error until cells exist.

use std::collections::BTreeSet;

use super::{error, Parser};
use crate::ast::{
    comp_name, Expr, ExprKind, FuncDef, Ident, Mode, Param, ParamMode, Stmt, StmtKind,
};
use crate::diag::Result;
use crate::infer::scope;
use crate::token::TokenKind;

const PLAIN: &str = "lambda parameters are plain names";

impl Parser<'_> {
    /// `'lambda' [NAME (',' NAME)*] ':' expr`
    pub(super) fn lambda(&mut self) -> Result<Expr> {
        let keyword = self.bump().span;
        let mut params = Vec::new();
        while !self.at(&TokenKind::Colon) {
            let name = self.binding_name("a lambda parameter")?;
            if self.at(&TokenKind::Assign) {
                return Err(error(self.tok(0).span, PLAIN));
            }
            params.push(Param {
                mode: ParamMode::Read,
                span: name.span,
                name,
                annotation: None,
                default: None,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        if self.at(&TokenKind::Star) || self.at(&TokenKind::DoubleStar) {
            return Err(error(self.tok(0).span, PLAIN));
        }
        let colon = self.expect(&TokenKind::Colon)?.span;
        let inner = self.lifted.len();
        let value = self.expr()?;
        let own: BTreeSet<String> = params.iter().map(|p| p.name.name.clone()).collect();
        no_capture(&self.lifted[inner..], &own, "the enclosing lambda")?;
        let span = self.span_from(keyword);
        let name = Ident {
            name: comp_name("lambda", keyword.start),
            span: keyword,
        };
        self.lifted.push(Stmt {
            kind: StmtKind::FuncDef(FuncDef {
                mode: Mode::Loose,
                name: name.clone(),
                params,
                returns: None,
                raises: Vec::new(),
                body: vec![Stmt {
                    kind: StmtKind::Return(Some(value)),
                    span: colon,
                }],
                span: keyword.merge(colon),
            }),
            span: keyword.merge(colon),
        });
        Ok(Expr {
            kind: ExprKind::Name(name.name),
            span,
        })
    }
}

/// No lambda of `lifted` reads a name of `bound`, which `owner` binds
/// (D195): a function, a comprehension or an enclosing lambda.
pub(super) fn no_capture<'a>(
    lifted: impl IntoIterator<Item = &'a Stmt>,
    bound: &BTreeSet<String>,
    owner: &str,
) -> Result<()> {
    for stmt in lifted {
        let StmtKind::FuncDef(def) = &stmt.kind else {
            continue;
        };
        let captured = scope::name_uses(def)
            .into_iter()
            .find(|(n, _)| bound.contains(*n));
        if let Some((name, span)) = captured {
            let msg = format!(
                "lambda uses `{name}`, which belongs to {owner}; \
                 captures need closures (not supported yet)"
            );
            return Err(error(span, msg));
        }
    }
    Ok(())
}
