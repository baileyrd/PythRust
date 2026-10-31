//! Type inference: `ast::Module` to a typed view, or the first type error.
//! Thread C.
//!
//! Hindley-Milner over the phase-2 subset. Annotations are hints that
//! must agree with what is inferred. Top-level functions are generalized
//! in call-graph order (mutually recursive ones together), so a loose
//! `def` used at several types gets one generic signature. Operations
//! whose meaning depends on an operand type (`+`, `len`, `for`, indexing)
//! are deferred constraints, see [`solve`]; a generic function carries
//! its unsolved ones and re-checks them at each call.
//!
//! Types are keyed by the `Span` of the node they describe: expressions,
//! name targets and parameter names. The AST has no node ids, and no two
//! of those nodes share a span.

mod builtins;
mod expr;
mod raises;
pub(crate) mod scope;
mod solve;
mod stmt;
mod subst;
mod temper;
mod ty;

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};
use std::mem;

use crate::ast::{Block, FuncDef, Module, StmtKind, StubDef, TypeExpr};
use crate::diag::{DiagKind, Diagnostic, Result, Warning};
use crate::span::Span;

pub use builtins::is_builtin;
pub use raises::Raises;
use solve::{Constraint, Need};
use subst::Subst;
pub use ty::{FuncType, ParamType, Type, TypeVar, View};

/// The typed view of a module: what the backend consumes. The default
/// knows no types, for running a module built by hand.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Types {
    nodes: HashMap<Span, Type>,
    funcs: HashMap<String, FuncType>,
    raises: HashMap<String, Raises>,
    warnings: Vec<Warning>,
}

impl Types {
    /// The type of the expression, name target or parameter name at `span`.
    pub fn at(&self, span: Span) -> Option<&Type> {
        self.nodes.get(&span)
    }

    /// The signature of the top-level function `name`.
    pub fn func(&self, name: &str) -> Option<&FuncType> {
        self.funcs.get(name)
    }

    /// The exceptions that may escape the top-level function `name`:
    /// declared by its `raises` clause (or tempering), else inferred.
    pub fn raises(&self, name: &str) -> Option<&Raises> {
        self.raises.get(name)
    }

    /// Lints on the module, in source order: a `raises` clause listing
    /// `Exception` that its body does not need.
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }
}

/// Infers types for `module`, or returns the first type error.
pub fn infer(module: &Module) -> Result<Types> {
    let defs = top_level_defs(module)?;
    let mut c = Checker::default();
    c.declare_globals(module, &defs)?;
    c.declare_stubs(&module.stubs, &defs)?;
    for group in scope::groups(&defs) {
        c.infer_group(&group)?;
    }
    for s in &module.body {
        if !matches!(s.kind, StmtKind::FuncDef(_)) {
            c.stmt(s)?;
            c.settle()?;
        }
    }
    // Whatever still waits involves only variables nothing ever bound: no
    // value of those types is ever built (`bubble_sort([])` compares no
    // elements), so the operation never runs. It is accepted, and the
    // variables stay unconstrained in the typed view.
    solve::default_assigns(&mut c.subst, mem::take(&mut c.pending))?;
    c.resolve_schemes()?;
    let mut types = c.finish();
    temper::check(&defs, &types)?;
    (types.raises, types.warnings) = raises::check(module, &defs, &types)?;
    Ok(types)
}

fn top_level_defs(module: &Module) -> Result<Vec<&FuncDef>> {
    let mut seen = BTreeSet::new();
    let mut defs = Vec::new();
    for s in &module.body {
        let StmtKind::FuncDef(f) = &s.kind else {
            continue;
        };
        if !seen.insert(f.name.name.as_str()) {
            return Err(error(
                f.name.span,
                format!("`{}` is defined twice", f.name.name),
            ));
        }
        defs.push(f);
    }
    Ok(defs)
}

/// A top-level function as callers see it.
#[derive(Debug, Clone)]
enum Func {
    /// Being inferred with its group: calls use the signature directly.
    Mono(FuncType),
    /// Generalized: each call instantiates fresh variables.
    Poly(Scheme),
}

/// `vars` are generic; `needs` are re-checked at every call.
#[derive(Debug, Clone)]
struct Scheme {
    vars: Vec<TypeVar>,
    needs: Vec<Constraint>,
    func: FuncType,
}

/// The scope being inferred. `ret` is `None` at module level.
#[derive(Debug, Default)]
struct Frame {
    locals: HashMap<String, Type>,
    ret: Option<Type>,
    /// Names known not to be `None` here, with their narrowed type
    /// (`if x is not None:` on an `int | None` makes `x` an `int`).
    narrowed: HashMap<String, Type>,
    /// Names an enclosing `if x:` or `while x:` tests only for truth,
    /// which does not narrow them.
    truthy: BTreeSet<String>,
    /// Operations on `truthy` names that were still `T | None`, so an
    /// error on one can suggest `x is not None`.
    truth_uses: Vec<TruthUse>,
}

/// A constraint at `span` with a `truthy` name as a direct operand, and
/// the same constraint with that operand narrowed to its present type.
#[derive(Debug)]
struct TruthUse {
    span: Span,
    name: String,
    narrowed: Need,
}

#[derive(Debug, Default)]
struct Checker {
    subst: Subst,
    nodes: HashMap<Span, Type>,
    globals: HashMap<String, Type>,
    funcs: HashMap<String, Func>,
    pending: Vec<Constraint>,
    frame: Frame,
    /// Types of parameters with defaults, across all groups. Never
    /// generic, and resolved afresh at each generalization.
    defaults: Vec<Type>,
    /// Rust binding functions that raise a class the program redefines,
    /// with that class's name.
    stub_redefined: HashMap<String, String>,
}

impl Checker {
    fn declare_globals(&mut self, module: &Module, defs: &[&FuncDef]) -> Result<()> {
        for name in scope::assigned(&module.body) {
            let ty = self.subst.fresh();
            self.globals.insert(name, ty);
        }
        let globals = self.globals.clone();
        self.declare_annotated(&module.body, &globals)?;
        match defs
            .iter()
            .find(|f| self.globals.contains_key(&f.name.name))
        {
            Some(f) => Err(error(
                f.name.span,
                format!("`{}` is both a function and a variable", f.name.name),
            )),
            None => Ok(()),
        }
    }

    /// Gives each Rust binding function its stub's signature (M6b). A
    /// stub has no body to check, and its dotted name no program can
    /// bind, so it is never generic or shadowed. A class it raises that
    /// the program redefines is recorded for its calls to refuse.
    fn declare_stubs(&mut self, stubs: &[StubDef], defs: &[&FuncDef]) -> Result<()> {
        for f in stubs {
            let redefined = f.raises.iter().find(|r| {
                self.globals.contains_key(&r.name) || defs.iter().any(|d| d.name.name == r.name)
            });
            if let Some(r) = redefined {
                self.stub_redefined
                    .insert(f.name.name.clone(), r.name.clone());
            }
            let mut params = Vec::new();
            for p in &f.params {
                let ty = self.annotation(&p.annotation)?;
                params.push(ParamType {
                    name: p.name.name.clone(),
                    ty,
                    has_default: false,
                });
            }
            let ret = self.annotation(&f.returns)?;
            let sig = FuncType { params, ret };
            self.funcs.insert(f.name.name.clone(), Func::Mono(sig));
        }
        Ok(())
    }

    /// Gives each annotated variable in `block` its annotated type up
    /// front, so code that runs earlier in inference order (a function
    /// body reading an annotated global) sees it.
    fn declare_annotated(&mut self, block: &Block, vars: &HashMap<String, Type>) -> Result<()> {
        for (name, annotation) in scope::annotated(block) {
            let declared = self.annotation(annotation)?;
            let Some(var) = vars.get(name) else {
                continue;
            };
            self.subst.unify(var, &declared).map_err(|clash| {
                let detail = solve::mismatch(&self.subst, clash, var, &declared);
                error(
                    annotation.span,
                    format!("conflicting annotations for `{name}`: {detail}"),
                )
            })?;
        }
        Ok(())
    }

    /// Infers a group of mutually recursive functions, then generalizes.
    fn infer_group(&mut self, group: &[&FuncDef]) -> Result<()> {
        let outer = mem::take(&mut self.pending);
        for f in group {
            let sig = self.signature(f)?;
            self.funcs.insert(f.name.name.clone(), Func::Mono(sig));
        }
        for f in group {
            self.body(f)?;
        }
        let waiting = solve::solve(&mut self.subst, mem::take(&mut self.pending))?;
        self.pending = outer;
        self.generalize(group, waiting)
    }

    fn signature(&mut self, f: &FuncDef) -> Result<FuncType> {
        let mut params = Vec::new();
        for p in &f.params {
            let ty = self.hint(p.annotation.as_ref())?;
            self.nodes.insert(p.name.span, ty.clone());
            params.push(ParamType {
                name: p.name.name.clone(),
                ty,
                has_default: p.default.is_some(),
            });
        }
        let ret = self.hint(f.returns.as_ref())?;
        Ok(FuncType { params, ret })
    }

    /// An annotation's type, or a fresh variable when there is none.
    fn hint(&mut self, annotation: Option<&TypeExpr>) -> Result<Type> {
        match annotation {
            Some(t) => self.annotation(t),
            None => Ok(self.subst.fresh()),
        }
    }

    /// Infers defaults (in module scope) and the body of `f`. A default's
    /// type joins `defaults`: one default object is shared by every call,
    /// so its type must not be generic.
    fn body(&mut self, f: &FuncDef) -> Result<()> {
        let sig = self.mono(f)?;
        for (p, pt) in f.params.iter().zip(&sig.params) {
            let Some(default) = &p.default else {
                continue;
            };
            let t = self.checked(default, &pt.ty)?;
            let what = format!("default for `{}`", pt.name);
            self.expect(default.span, &pt.ty, &t, &what)?;
            self.defaults.push(pt.ty.clone());
        }
        let mut locals: HashMap<String, Type> = sig
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        for name in scope::assigned(&f.body) {
            let fresh = self.subst.fresh();
            locals.entry(name).or_insert(fresh);
        }
        self.declare_annotated(&f.body, &locals)?;
        self.frame = Frame {
            locals,
            ret: Some(sig.ret.clone()),
            narrowed: HashMap::new(),
            truthy: BTreeSet::new(),
            truth_uses: Vec::new(),
        };
        self.block(&f.body)?;
        self.frame = Frame::default();
        if !scope::falls_through(&f.body) {
            return Ok(());
        }
        let name = &f.name.name;
        let none = self.subst.assign(&sig.ret, &Type::None);
        none.map(drop).map_err(|_| {
            let ret = self.subst.zonk(&sig.ret);
            let msg = format!(
                "`{name}` returns {ret}, but can reach its end, which returns None; \
                 annotate `-> {ret} | None` if that is intended"
            );
            error(f.name.span, msg)
        })
    }

    fn mono(&self, f: &FuncDef) -> Result<FuncType> {
        let name = &f.name.name;
        match self.funcs.get(name) {
            Some(Func::Mono(sig)) => Ok(sig.clone()),
            _ => Err(error(
                f.name.span,
                format!("internal: `{name}` has no signature in its group"),
            )),
        }
    }

    /// Turns each function's monomorphic signature into a scheme over the
    /// variables no global or default can reach. Waiting constraints on
    /// those variables join the scheme; the rest stay pending outside.
    ///
    /// The environment is resolved now, not when it was recorded, so a
    /// variable bound after the fact (`xs.append([])` on a default) stays
    /// fixed. Earlier schemes need no entry: their non-generic variables
    /// come from globals or defaults, which are both included.
    fn generalize(&mut self, group: &[&FuncDef], waiting: Vec<Constraint>) -> Result<()> {
        let mut env = BTreeSet::new();
        for t in self.globals.values().chain(&self.defaults) {
            self.subst.free_vars(t, &mut env);
        }
        let vars: Vec<BTreeSet<TypeVar>> = waiting
            .iter()
            .map(|c| solve::constraint_vars(&self.subst, c))
            .collect();
        let mut claimed = vec![false; waiting.len()];
        for f in group {
            let sig = self.mono(f)?;
            let generic = self.generic_vars(&sig, &vars, &env);
            let mut needs = Vec::new();
            for (i, c) in waiting.iter().enumerate() {
                if !vars[i].is_disjoint(&generic) {
                    claimed[i] = true;
                    needs.push(c.clone());
                }
            }
            let scheme = Scheme {
                vars: generic.into_iter().collect(),
                needs,
                func: sig,
            };
            self.funcs.insert(f.name.name.clone(), Func::Poly(scheme));
        }
        let unclaimed = waiting.into_iter().zip(claimed).filter(|(_, used)| !used);
        self.pending.extend(unclaimed.map(|(c, _)| c));
        Ok(())
    }

    /// Free variables of `sig` outside `env`, closed over the constraints
    /// that connect them to further variables.
    fn generic_vars(
        &self,
        sig: &FuncType,
        constraint_vars: &[BTreeSet<TypeVar>],
        env: &BTreeSet<TypeVar>,
    ) -> BTreeSet<TypeVar> {
        let mut generic = BTreeSet::new();
        for t in sig.params.iter().map(|p| &p.ty).chain([&sig.ret]) {
            self.subst.free_vars(t, &mut generic);
        }
        generic.retain(|v| !env.contains(v));
        loop {
            let linked: Vec<TypeVar> = constraint_vars
                .iter()
                .filter(|vs| !vs.is_disjoint(&generic))
                .flatten()
                .filter(|v| !env.contains(v) && !generic.contains(v))
                .copied()
                .collect();
            if linked.is_empty() {
                return generic;
            }
            generic.extend(linked);
        }
    }

    /// A fresh copy of `scheme`'s signature; its constraints are queued
    /// against the call at `span`.
    fn instantiate(&mut self, name: &str, scheme: &Scheme, span: Span) -> FuncType {
        let map: HashMap<TypeVar, Type> = scheme
            .vars
            .iter()
            .map(|v| (*v, self.subst.fresh()))
            .collect();
        for c in &scheme.needs {
            let need = solve::rename(&self.subst, &c.need, &map);
            self.pending.push(Constraint {
                need,
                span,
                via: Some(name.to_string()),
            });
        }
        let subst = &self.subst;
        subst::map_func(&scheme.func, |t| subst.rename(t, &map))
    }

    /// Re-solves every scheme's constraints once the module is known. A
    /// constraint can involve a global as well as generic variables
    /// (`def f(x): return x * N`); now that `N` is known it may resolve,
    /// which completes the signature. What still waits is truly generic.
    fn resolve_schemes(&mut self) -> Result<()> {
        let needs: Vec<Constraint> = self
            .funcs
            .values()
            .filter_map(|f| match f {
                Func::Poly(scheme) => Some(scheme.needs.clone()),
                Func::Mono(_) => None,
            })
            .flatten()
            .collect();
        solve::solve(&mut self.subst, needs).map(drop)
    }

    fn need(&mut self, need: Need, span: Span) {
        self.pending.push(Constraint {
            need,
            span,
            via: None,
        });
    }

    /// Solves what can be solved now, so errors surface where they arise.
    fn settle(&mut self) -> Result<()> {
        self.pending = solve::solve(&mut self.subst, mem::take(&mut self.pending))?;
        Ok(())
    }

    /// Assigns `found` into `expected` (see `Subst::assign`); on failure
    /// the message names `what`.
    fn expect(&mut self, span: Span, expected: &Type, found: &Type, what: &str) -> Result<()> {
        let deferred = self.subst.assign(expected, found).map_err(|clash| {
            let detail = solve::mismatch(&self.subst, clash, expected, found);
            error(span, format!("{what}: {detail}"))
        })?;
        for need in solve::assigns(deferred, what) {
            self.need(need, span);
        }
        Ok(())
    }

    fn finish(self) -> Types {
        let subst = &self.subst;
        let nodes = self
            .nodes
            .iter()
            .map(|(span, t)| (*span, subst.zonk(t)))
            .collect();
        let funcs = self
            .funcs
            .iter()
            .map(|(name, f)| {
                let sig = match f {
                    Func::Mono(sig) => sig,
                    Func::Poly(scheme) => &scheme.func,
                };
                (name.clone(), subst.zonk_func(sig))
            })
            .collect();
        Types {
            nodes,
            funcs,
            raises: HashMap::new(),
            warnings: Vec::new(),
        }
    }
}

fn error(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        span,
        message,
        kind: DiagKind::Type,
    }
}
