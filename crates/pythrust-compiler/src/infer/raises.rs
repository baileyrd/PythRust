//! Which exceptions escape each top-level function (grammar.md D8).
//! Checked after inference, so `int(s)` knows whether `s` is a `str`.
//!
//! A loose function without a `raises` clause gets the set its body
//! infers, through its callees, to a fixpoint for mutually recursive
//! ones. A tempered function, or any function with a clause, has the set
//! it declares, and its body must not let anything else escape. Callers
//! see that declared set, so a clause is a contract. A clause listing
//! `Exception` admits any exception, so it promises callers nothing; when
//! the body does not need it, a warning names the narrower clause.
//!
//! Only `raise` and `int(s)`/`float(s)` of a `str` count. The faults a
//! `try` can catch (division by zero, an index out of range, unpacking
//! arity) are implicit in the operators and never tracked here; lowering
//! has its own fact for them (`lower/faults.rs`). A set holds exception classes (B4): each escape is an
//! instance of its class or of one derived from it. It over-approximates:
//! `int(s)` counts whether or not `s` parses, and a raised value whose
//! class is not known before it runs (a parameter, a call's result) is
//! an `Exception`, which only `raises Exception` covers and only
//! `except Exception` catches.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ast::{
    Arg, Expr, ExprKind, FuncDef, Ident, Mode, Module, Stmt, StmtKind, Target, TargetKind,
};
use crate::diag::{Result, Warning};
use crate::exc::ExcKind;
use crate::span::Span;

use super::builtins;
use super::{error, scope, Type, Types};

/// The exceptions that may escape a function, by class (each escape is of
/// a class listed or one derived from it), each with the first place it
/// escapes from (a `raise`, a call, or the `raises` clause).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raises {
    sites: BTreeMap<ExcKind, Span>,
}

impl Raises {
    /// The classes of the escaping exceptions, sorted by name.
    pub fn escapes(&self) -> impl Iterator<Item = ExcKind> + '_ {
        self.sites.keys().copied()
    }

    /// Whether nothing escapes.
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    fn add(&mut self, e: ExcKind, at: Span) {
        self.sites.entry(e).or_insert(at);
    }

    fn extend(&mut self, other: Raises) {
        other.sites.into_iter().for_each(|(e, at)| self.add(e, at));
    }

    /// What escapes past handlers for `classes`: an escape of a class
    /// derived from none of them. One of a base class (an `Exception` of
    /// unknown class, an `ArithmeticError`) may or may not be of a handler's
    /// subclass, so it still escapes.
    fn caught(mut self, classes: &[ExcKind]) -> Raises {
        self.sites
            .retain(|e, _| !classes.iter().any(|c| e.is_a(*c)));
        self
    }

    /// As a declared set: whether it admits `e`, as one of its classes.
    fn covers(&self, e: ExcKind) -> bool {
        self.sites.keys().any(|d| e.is_a(*d))
    }

    fn same_escapes(&self, other: &Raises) -> bool {
        self.escapes().eq(other.escapes())
    }
}

/// Checks every declared set and returns each function's set (declared
/// where there is one, else inferred) and the warnings on clauses.
pub(super) fn check(
    module: &Module,
    defs: &[&FuncDef],
    types: &Types,
) -> Result<(HashMap<String, Raises>, Vec<Warning>)> {
    let mut globals = Bindings::new();
    bindings(&module.body, &mut globals);
    let mut declared = HashMap::new();
    for f in defs {
        if let Some(set) = declaration(f, defs, &globals)? {
            declared.insert(f.name.name.as_str(), set);
        }
    }
    let early = early(module);
    let late_shadow = defs
        .iter()
        .map(|f| f.name.name.as_str())
        .any(|n| builtins::is_builtin(n) && !early.contains(n));
    let mut a = Analysis {
        types,
        defs: defs.iter().map(|f| (f.name.name.as_str(), *f)).collect(),
        early,
        typed: !late_shadow,
        globals,
        sets: declared.clone(),
    };
    for group in scope::groups(defs) {
        a.infer_group(&group, &declared);
    }
    let mut warnings = Vec::new();
    for f in defs {
        if let Some(set) = declared.get(f.name.name.as_str()) {
            warnings.extend(a.check_body(f, set)?);
        }
    }
    warnings.sort_by_key(|w| (w.span.start, w.span.end));
    let sets = a.sets.into_iter().map(|(n, s)| (n.to_string(), s));
    Ok((sets.collect(), warnings))
}

/// The functions whose `def` runs before any other code: before every
/// other kind of module statement and every parameter default, its own
/// included. Any other `def` binds its name only when it runs, and a call
/// before that reaches the builtin of that name (`run` does this too).
fn early(module: &Module) -> HashSet<&str> {
    let mut out = HashSet::new();
    for s in &module.body {
        let StmtKind::FuncDef(f) = &s.kind else {
            break;
        };
        if f.params.iter().any(|p| p.default.is_some()) {
            break;
        }
        out.insert(f.name.name.as_str());
    }
    out
}

/// `f`'s declared set: its `raises` clause, or nothing for a tempered
/// function without one. `None` when the set is inferred.
fn declaration(f: &FuncDef, defs: &[&FuncDef], globals: &Bindings) -> Result<Option<Raises>> {
    if f.raises.is_empty() {
        return Ok((f.mode == Mode::Tempered).then(Raises::default));
    }
    let mut set = Raises::default();
    for name in &f.raises {
        let e = declared_escape(name, defs, globals)?;
        if set.sites.contains_key(&e) {
            let msg = format!("`{}` is listed twice in `raises`", name.name);
            return Err(error(name.span, msg));
        }
        set.add(e, name.span);
    }
    Ok(Some(set))
}

/// A name in a `raises` clause: a builtin exception class nothing
/// redefines; `Exception` admits any.
fn declared_escape(name: &Ident, defs: &[&FuncDef], globals: &Bindings) -> Result<ExcKind> {
    let n = name.name.as_str();
    let Some(builtin) = ExcKind::from_name(n) else {
        let names = builtins::exception_names();
        let msg = format!("`{n}` is not an exception; `raises` lists one of {names}");
        return Err(error(name.span, msg));
    };
    if shadowed(n, globals, defs.iter().map(|f| f.name.name.as_str())) {
        let msg = format!("`{n}` is redefined, so it no longer names the builtin exception");
        return Err(error(name.span, msg));
    }
    Ok(builtin)
}

/// Whether a module variable or a function named `n` hides the builtin
/// exception of that name, so a `raises` clause cannot name it.
fn shadowed<'a>(n: &str, globals: &Bindings, mut defs: impl Iterator<Item = &'a str>) -> bool {
    globals.contains_key(n) || defs.any(|d| d == n)
}

/// Where a variable's values come from.
#[derive(Debug)]
enum Source<'a> {
    Value(&'a Expr),
    /// `except Name as e`.
    Caught(ExcKind),
    /// A parameter, loop target or unpacked element.
    Opaque,
}

/// Every binding of each variable in a scope, flow-insensitively.
type Bindings<'a> = HashMap<&'a str, Vec<Source<'a>>>;

/// Adds the bindings of `block` and the blocks nested in it.
fn bindings<'a>(block: &'a [Stmt], out: &mut Bindings<'a>) {
    for s in block {
        match &s.kind {
            StmtKind::Assign { target, value } => bind_target(target, value, out),
            StmtKind::AnnAssign { target, value, .. } => {
                out.entry(&target.name)
                    .or_default()
                    .push(Source::Value(value));
            }
            StmtKind::For {
                target,
                body,
                orelse,
                ..
            } => {
                opaque(target, out);
                bindings(body, out);
                bindings(orelse, out);
            }
            StmtKind::While { branch, orelse } => {
                bindings(&branch.body, out);
                bindings(orelse, out);
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                std::iter::once(branch)
                    .chain(elifs)
                    .for_each(|b| bindings(&b.body, out));
                bindings(orelse, out);
            }
            StmtKind::Try { body, handlers } => {
                bindings(body, out);
                for h in handlers {
                    caught(h.binding.as_ref(), &h.classes, out);
                    bindings(&h.body, out);
                }
            }
            _ => {}
        }
    }
}

fn bind_target<'a>(target: &'a Target, value: &'a Expr, out: &mut Bindings<'a>) {
    match &target.kind {
        TargetKind::Name(name) => out.entry(name).or_default().push(Source::Value(value)),
        _ => opaque(target, out),
    }
}

fn opaque<'a>(target: &'a Target, out: &mut Bindings<'a>) {
    match &target.kind {
        TargetKind::Name(name) => out.entry(name).or_default().push(Source::Opaque),
        TargetKind::Tuple(items) => items.iter().for_each(|t| opaque(t, out)),
        TargetKind::Subscript { .. } => {}
    }
}

fn caught<'a>(binding: Option<&'a Ident>, classes: &[Ident], out: &mut Bindings<'a>) {
    let Some(b) = binding else {
        return;
    };
    let sources = classes
        .iter()
        .map(|c| ExcKind::from_name(&c.name).map_or(Source::Opaque, Source::Caught));
    out.entry(&b.name).or_default().extend(sources);
}

/// The module-wide facts, and each function's set so far.
struct Analysis<'a> {
    types: &'a Types,
    defs: HashMap<&'a str, &'a FuncDef>,
    /// Functions bound before any code runs, so a call by their name
    /// never reaches the builtin they may shadow.
    early: HashSet<&'a str>,
    /// Whether every value has its inferred type when it runs. Not when a
    /// `def` shadowing a builtin may run late: inference types its calls
    /// by the `def`, but a call before it runs gets the builtin's result
    /// (`int(str(x))` with a later `def str(...) -> int`).
    typed: bool,
    globals: Bindings<'a>,
    sets: HashMap<&'a str, Raises>,
}

impl<'a> Analysis<'a> {
    /// Infers the undeclared sets of one strongly connected `group`,
    /// from nothing (a call to a function with no set yet adds nothing)
    /// up to a fixpoint. Sets only grow, and there are
    /// finitely many escapes, so this stops.
    fn infer_group(&mut self, group: &[&'a FuncDef], declared: &HashMap<&str, Raises>) {
        let inferred: Vec<&'a FuncDef> = group
            .iter()
            .copied()
            .filter(|f| !declared.contains_key(f.name.name.as_str()))
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for f in &inferred {
                let set = self.body(f);
                let old = self.sets.insert(&f.name.name, set.clone());
                changed |= old.is_none_or(|o| !o.same_escapes(&set));
            }
        }
    }

    /// What `f`'s body lets escape, given the current sets.
    fn body(&self, f: &'a FuncDef) -> Raises {
        let mut locals = Bindings::new();
        for p in &f.params {
            locals.entry(&p.name.name).or_default().push(Source::Opaque);
        }
        bindings(&f.body, &mut locals);
        let cx = Cx {
            a: self,
            locals: &locals,
            module: false,
        };
        cx.block(&f.body)
    }

    /// The first escape from `f`'s body that `declared` does not admit,
    /// in source order, is an error; else a declared `Exception` the body
    /// does not need is a warning.
    fn check_body(&self, f: &'a FuncDef, declared: &Raises) -> Result<Option<Warning>> {
        let got = self.body(f);
        let missing = got
            .sites
            .iter()
            .filter(|(e, _)| !declared.covers(**e))
            .min_by_key(|(_, at)| at.start);
        let Some((e, at)) = missing else {
            let nameable =
                |e: ExcKind| !shadowed(e.name(), &self.globals, self.defs.keys().copied());
            return Ok(vacuous(f, declared, &got, nameable));
        };
        let mut all = declared.clone();
        all.extend(got.clone());
        Err(error(*at, undeclared(f, *e, &all, self.types)))
    }
}

/// The message for `f` letting `e` escape undeclared, with the clause
/// that would admit everything in `all`.
fn undeclared(f: &FuncDef, e: ExcKind, all: &Raises, types: &Types) -> String {
    let name = &f.name.name;
    let what = match e {
        ExcKind::Exception => {
            "any exception here (an `Exception`, or a value whose class is known only when \
             it runs)"
                .to_string()
        }
        class => format!("{} here", class.name()),
    };
    let fix = clause(all);
    if !f.raises.is_empty() {
        return format!(
            "`{name}` can raise {what}, which its `raises` clause does not list; \
             write `raises {fix}`"
        );
    }
    // Tempered, so `temper::check` has seen a complete return annotation.
    let ret = types.func(name).map(|sig| sig.ret.to_string());
    let ret = ret.unwrap_or_default();
    format!(
        "tempered `{name}` can raise {what}, so it must declare it; write `-> {ret} raises {fix}`"
    )
}

/// The warning for `f`'s clause listing `Exception` when no escape from
/// its body (`got`) is of unknown class, so the clause can say more: the
/// other declared classes plus what escapes, or no clause when that is
/// nothing. An `Exception` escaping from a callee whose own clause lists
/// it vacuously keeps this one quiet; the callee gets the warning. So
/// does a narrower clause naming a class the module redefines, which no
/// clause can name.
fn vacuous(
    f: &FuncDef,
    declared: &Raises,
    got: &Raises,
    nameable: impl Fn(ExcKind) -> bool,
) -> Option<Warning> {
    let at = *declared.sites.get(&ExcKind::Exception)?;
    if got.sites.contains_key(&ExcKind::Exception) {
        return None;
    }
    let mut narrow = got.clone();
    let listed = declared
        .sites
        .iter()
        .filter(|(e, _)| **e != ExcKind::Exception);
    listed.for_each(|(e, s)| narrow.add(*e, *s));
    if !clause_classes(&narrow).into_iter().all(nameable) {
        return None;
    }
    let escapes = match clause(got) {
        none if none.is_empty() => "nothing".to_string(),
        some => format!("only {some}"),
    };
    let fix = match clause(&narrow) {
        none if none.is_empty() => "remove the `raises` clause".to_string(),
        some => format!("write `raises {some}`"),
    };
    let name = &f.name.name;
    let message = format!(
        "`raises Exception` admits any exception, so it tells callers of `{name}` nothing, \
         and {escapes} can escape it; {fix}"
    );
    Some(Warning { span: at, message })
}

/// The clause admitting every escape in `set`: the classes no other one
/// there derives from (`Exception` admits all).
fn clause(set: &Raises) -> String {
    let names: Vec<&str> = clause_classes(set).into_iter().map(ExcKind::name).collect();
    names.join(", ")
}

/// The classes [`clause`] names.
fn clause_classes(set: &Raises) -> Vec<ExcKind> {
    let derived = |e: ExcKind| set.escapes().any(|d| d != e && e.is_a(d));
    set.escapes().filter(|e| !derived(*e)).collect()
}

/// One scope being walked: a function body, or the module for globals.
struct Cx<'s, 'a> {
    a: &'s Analysis<'a>,
    locals: &'s Bindings<'a>,
    /// Whether `locals` are the module's: names there fall back to the
    /// builtins until bound, where a function's locals do not.
    module: bool,
}

impl<'a> Cx<'_, 'a> {
    fn block(&self, block: &'a [Stmt]) -> Raises {
        let mut out = Raises::default();
        block.iter().for_each(|s| out.extend(self.stmt(s)));
        out
    }

    fn stmt(&self, s: &'a Stmt) -> Raises {
        let mut out = Raises::default();
        match &s.kind {
            StmtKind::Assign { target, value } | StmtKind::AugAssign { target, value, .. } => {
                self.expr(value, &mut out);
                self.target(target, &mut out);
            }
            StmtKind::AnnAssign { value, .. } | StmtKind::Expr(value) => {
                self.expr(value, &mut out);
            }
            StmtKind::Return(value) => value.iter().for_each(|v| self.expr(v, &mut out)),
            StmtKind::Raise(value) => {
                self.expr(value, &mut out);
                for k in self.kinds(value, &mut BTreeSet::new()) {
                    out.add(k, value.span);
                }
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                for b in std::iter::once(branch).chain(elifs) {
                    self.expr(&b.test, &mut out);
                    out.extend(self.block(&b.body));
                }
                out.extend(self.block(orelse));
            }
            StmtKind::While { branch, orelse } => {
                self.expr(&branch.test, &mut out);
                out.extend(self.block(&branch.body));
                out.extend(self.block(orelse));
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                self.expr(iter, &mut out);
                self.target(target, &mut out);
                out.extend(self.block(body));
                out.extend(self.block(orelse));
            }
            StmtKind::Try { body, handlers } => {
                let classes: Vec<ExcKind> = handlers
                    .iter()
                    .flat_map(|h| &h.classes)
                    .filter_map(|c| ExcKind::from_name(&c.name))
                    .collect();
                out.extend(self.block(body).caught(&classes));
                handlers
                    .iter()
                    .for_each(|h| out.extend(self.block(&h.body)));
            }
            StmtKind::FuncDef(_) | StmtKind::Break | StmtKind::Continue | StmtKind::Pass => {}
        }
        out
    }

    /// The expressions inside subscript targets.
    fn target(&self, target: &'a Target, out: &mut Raises) {
        match &target.kind {
            TargetKind::Name(_) => {}
            TargetKind::Subscript { value, index } => {
                self.expr(value, out);
                self.expr(index, out);
            }
            TargetKind::Tuple(items) => items.iter().for_each(|t| self.target(t, out)),
        }
    }

    /// Adds what evaluating `e` may raise: calls to functions that raise,
    /// and `int(s)`/`float(s)` of a string.
    fn expr(&self, e: &Expr, out: &mut Raises) {
        match &e.kind {
            ExprKind::Call { func, args } => {
                args.iter().for_each(|a| self.expr(&a.value, out));
                match &func.kind {
                    ExprKind::Name(name) => self.call(name, args.first(), e.span, out),
                    _ => self.expr(func, out),
                }
            }
            ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Set(items) => {
                items.iter().for_each(|x| self.expr(x, out));
            }
            ExprKind::Dict(items) => items
                .iter()
                .flat_map(|(k, v)| [k, v])
                .for_each(|x| self.expr(x, out)),
            ExprKind::Unary { operand, .. } => self.expr(operand, out),
            ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
                self.expr(left, out);
                self.expr(right, out);
            }
            ExprKind::Compare { left, rest } => {
                self.expr(left, out);
                rest.iter().for_each(|(_, x)| self.expr(x, out));
            }
            ExprKind::IfExp { test, body, orelse } => {
                [test, body, orelse]
                    .into_iter()
                    .for_each(|x| self.expr(x, out));
            }
            ExprKind::Subscript { value, index } => {
                self.expr(value, out);
                self.expr(index, out);
            }
            ExprKind::Attr { value, .. } => self.expr(value, out),
            ExprKind::Name(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::None => {}
        }
    }

    /// A call by name: a top-level function lets its set escape here;
    /// `int(s)` and `float(s)` raise `ValueError` unless `s` is surely a
    /// number.
    /// A call may reach a builtin that a `def` shadows when the `def` may
    /// not have run yet, so both count.
    fn call(&self, name: &str, first: Option<&Arg>, at: Span, out: &mut Raises) {
        if self.a.defs.contains_key(name) {
            let set = self.a.sets.get(name);
            set.into_iter()
                .flat_map(Raises::escapes)
                .for_each(|e| out.add(e, at));
        }
        if self.a.early.contains(name) {
            return;
        }
        let parses = matches!(name, "int" | "float")
            && first
                .is_some_and(|arg| !self.a.typed || may_be_str(self.a.types.at(arg.value.span)));
        if parses {
            out.add(ExcKind::Value, at);
        }
    }

    /// Which exceptions `raise value` may raise. `seen` cuts cycles
    /// between variables (`a = b`, `b = a`), which add nothing new.
    fn kinds(&self, value: &'a Expr, seen: &mut BTreeSet<&'a str>) -> BTreeSet<ExcKind> {
        let unknown = BTreeSet::from([ExcKind::Exception]);
        match &value.kind {
            ExprKind::Name(name) => self.name_kinds(name, seen),
            ExprKind::Call { func, .. } => match &func.kind {
                ExprKind::Name(name) if !self.a.defs.contains_key(name.as_str()) => {
                    ExcKind::from_name(name).map_or(unknown, |k| BTreeSet::from([k]))
                }
                _ => unknown,
            },
            ExprKind::IfExp { body, orelse, .. } => {
                let mut out = self.kinds(body, seen);
                out.extend(self.kinds(orelse, seen));
                out
            }
            ExprKind::Logical { left, right, .. } => {
                let mut out = self.kinds(left, seen);
                out.extend(self.kinds(right, seen));
                out
            }
            _ => unknown,
        }
    }

    /// A variable's kinds, from all its bindings. A module-level name
    /// may also be read before it is bound, which finds the builtin
    /// exception of that name (`raise ValueError`); a local cannot.
    fn name_kinds(&self, name: &'a str, seen: &mut BTreeSet<&'a str>) -> BTreeSet<ExcKind> {
        if !self.module {
            if let Some(sources) = self.locals.get(name) {
                return self.sources(name, sources, seen);
            }
            let module = Cx {
                a: self.a,
                locals: &self.a.globals,
                module: true,
            };
            return module.name_kinds(name, &mut BTreeSet::new());
        }
        let mut out: BTreeSet<ExcKind> = ExcKind::from_name(name).into_iter().collect();
        match self.locals.get(name) {
            Some(sources) => out.extend(self.sources(name, sources, seen)),
            None if out.is_empty() => {
                out.insert(ExcKind::Exception);
            }
            None => {}
        }
        out
    }

    /// The kinds `name`'s bindings give it; nothing on a cycle back to it.
    fn sources(
        &self,
        name: &'a str,
        sources: &[Source<'a>],
        seen: &mut BTreeSet<&'a str>,
    ) -> BTreeSet<ExcKind> {
        if !seen.insert(name) {
            return BTreeSet::new();
        }
        sources.iter().flat_map(|s| self.source(s, seen)).collect()
    }

    fn source(&self, s: &Source<'a>, seen: &mut BTreeSet<&'a str>) -> BTreeSet<ExcKind> {
        match s {
            Source::Value(v) => self.kinds(v, seen),
            Source::Caught(k) => BTreeSet::from([*k]),
            Source::Opaque => BTreeSet::from([ExcKind::Exception]),
        }
    }
}

/// Whether an `int()`/`float()` argument of type `t` may be a string,
/// which may not parse. A generic argument may be.
fn may_be_str(t: Option<&Type>) -> bool {
    !matches!(t, Some(Type::Int | Type::Float | Type::Bool))
}

#[cfg(test)]
mod tests {
    use crate::{infer, lexer, parser};

    /// `src`'s types, or the first diagnostic's message.
    fn types(src: &str) -> Result<infer::Types, String> {
        let tokens = lexer::lex(src).map_err(|d| d.message)?;
        let module = parser::parse(&tokens).map_err(|d| d.message)?;
        infer::infer(&module).map_err(|d| d.message)
    }

    /// What may escape `f` in `src`, as a `raises` clause would list it.
    fn raises(src: &str, f: &str) -> String {
        let types = types(src).unwrap_or_else(|e| panic!("{e}\n{src}"));
        let set = types.raises(f).unwrap_or_else(|| panic!("no `{f}`"));
        let names: Vec<String> = set.escapes().map(|e| e.name().to_string()).collect();
        names.join(", ")
    }

    fn err(src: &str) -> String {
        match types(src) {
            Ok(_) => panic!("expected an error for:\n{src}"),
            Err(msg) => msg,
        }
    }

    /// `src`'s warnings, as (the text the span covers, message).
    fn warnings(src: &str) -> Vec<(String, String)> {
        let types = types(src).unwrap_or_else(|e| panic!("{e}\n{src}"));
        let at = |w: &crate::Warning| src[w.span.start..w.span.end].to_string();
        types
            .warnings()
            .iter()
            .map(|w| (at(w), w.message.clone()))
            .collect()
    }

    fn vacuous(name: &str, escapes: &str, fix: &str) -> (String, String) {
        let message = format!(
            "`raises Exception` admits any exception, so it tells callers of `{name}` \
             nothing, and {escapes} can escape it; {fix}"
        );
        ("Exception".to_string(), message)
    }

    #[test]
    fn raise_and_parse_builtins_escape() {
        let src = "\
def f(s: str, n: int, x: float):
    if n < 0:
        raise IndexError(\"negative\")
    if n == 0:
        raise ZeroDivisionError
    return int(s) + int(x) + float(n)
";
        assert_eq!(
            raises(src, "f"),
            "IndexError, ValueError, ZeroDivisionError"
        );
    }

    #[test]
    fn faults_and_numeric_conversions_do_not_escape() {
        let src = "\
def f(xs: list[int], n: int):
    return xs[10] + n // 0 + int(2.5) + int(True) + int() + float(n)
";
        assert_eq!(raises(src, "f"), "");
    }

    #[test]
    fn catchable_faults_need_no_clause_and_leave_the_lint_alone() {
        let src = "\
temper def at(xs: list[int], i: int) -> int:
    a, b = xs
    return xs[i] // a
def g(xs: list[int]) -> int raises Exception:
    return xs[0] % 2
";
        assert_eq!(raises(src, "at"), "");
        let fix = "remove the `raises` clause";
        assert_eq!(warnings(src), vec![vacuous("g", "nothing", fix)]);
    }

    #[test]
    fn a_generic_argument_may_be_a_string() {
        assert_eq!(raises("def f(x):\n    return int(x)\n", "f"), "ValueError");
    }

    #[test]
    fn handlers_catch_by_name_and_their_bodies_escape() {
        let src = "\
def f(s: str) -> int:
    try:
        if s == \"\":
            raise IndexError
        return int(s)
    except ValueError:
        raise OverflowError(\"bad\")
";
        assert_eq!(raises(src, "f"), "IndexError, OverflowError");
    }

    #[test]
    fn a_tuple_handler_catches_each_of_its_classes() {
        let src = "\
def f(n: int) -> None:
    try:
        if n == 0:
            raise IndexError
        if n == 1:
            raise ZeroDivisionError
        raise RecursionError
    except (LookupError, ZeroDivisionError):
        pass
";
        assert_eq!(raises(src, "f"), "RecursionError");
    }

    #[test]
    fn a_tuple_binding_re_raised_may_be_any_of_its_classes() {
        let src = "\
def f(n: int) -> None:
    try:
        if n == 0:
            raise IndexError
        raise ValueError
    except (IndexError, ValueError) as e:
        raise e
";
        assert_eq!(raises(src, "f"), "IndexError, ValueError");
    }

    #[test]
    fn a_handler_catches_the_classes_derived_from_its_own() {
        let src = "\
def f(n: int) -> None:
    try:
        if n == 0:
            raise IndexError
        if n == 1:
            raise ZeroDivisionError
        raise ArithmeticError
    except LookupError:
        pass
    except ZeroDivisionError:
        pass
";
        // An `ArithmeticError` may be of any of its subclasses, or none.
        assert_eq!(raises(src, "f"), "ArithmeticError");
        let any = "def f(e: Exception) -> None:\n    try:\n        raise e\n    except Exception:\n        pass\n";
        assert_eq!(raises(any, "f"), "");
        let plain = "def f() -> None:\n    raise Exception(\"x\")\n";
        assert_eq!(raises(plain, "f"), "Exception");
    }

    #[test]
    fn a_base_class_handler_removes_every_subclass() {
        let src = "\
def f(n: int) -> None:
    try:
        if n == 0:
            raise ZeroDivisionError
        if n == 1:
            raise OverflowError
        if n == 2:
            raise ArithmeticError
        raise IndexError
    except ArithmeticError:
        pass
";
        assert_eq!(raises(src, "f"), "IndexError");
        let any = src.replace("except ArithmeticError", "except Exception");
        assert_eq!(raises(&any, "f"), "");
    }

    #[test]
    fn a_clause_admits_the_classes_derived_from_its_own() {
        let src = "\
temper def f(n: int) -> int raises ArithmeticError, LookupError:
    if n == 0:
        raise ZeroDivisionError
    if n == 1:
        raise IndexError
    return int(\"1\") // n
";
        let ok = src.replace("return int(\"1\") // n", "return n");
        assert_eq!(raises(&ok, "f"), "ArithmeticError, LookupError");
        assert_eq!(
            err(src),
            "`f` can raise ValueError here, which its `raises` clause does not list; \
             write `raises ArithmeticError, LookupError, ValueError`"
        );
        let base = "temper def f() -> None raises ZeroDivisionError:\n    raise ArithmeticError\n";
        assert!(err(base).starts_with("`f` can raise ArithmeticError here"));
    }

    #[test]
    fn nested_try_catches_outside_in() {
        let src = "\
def f(s: str) -> int:
    try:
        try:
            return int(s)
        except IndexError:
            return 0
    except ValueError:
        return -1
";
        assert_eq!(raises(src, "f"), "");
    }

    #[test]
    fn calls_carry_the_callee_set_and_recursion_reaches_a_fixpoint() {
        let src = "\
def even(n, s):
    if n == 0:
        return int(s)
    return odd(n - 1, s)

def odd(n, s):
    if n == 0:
        raise IndexError
    return even(n - 1, s)

def top():
    try:
        return even(4, \"1\")
    except IndexError:
        return 0
";
        assert_eq!(raises(src, "even"), "IndexError, ValueError");
        assert_eq!(raises(src, "odd"), "IndexError, ValueError");
        assert_eq!(raises(src, "top"), "ValueError");
    }

    #[test]
    fn a_reraised_variable_has_the_kinds_bound_to_it() {
        let src = "\
err = OverflowError(\"module\")

def f(s: str, flag: bool):
    try:
        return int(s)
    except ValueError as e:
        other = e if flag else IndexError()
        raise other

def g():
    raise err

def h(a: bool):
    x = ValueError()
    y = x
    x = y
    raise x
";
        assert_eq!(raises(src, "f"), "IndexError, ValueError");
        assert_eq!(raises(src, "g"), "OverflowError");
        assert_eq!(raises(src, "h"), "ValueError");
    }

    #[test]
    fn a_value_of_unknown_kind_escapes_every_handler() {
        let src = "\
def make(msg):
    return ValueError(msg)

def f(e: Exception):
    raise e

def g():
    try:
        raise make(\"x\")
    except ValueError:
        pass

def h(es: list[Exception]):
    for e in es:
        raise e
";
        assert_eq!(raises(src, "f"), "Exception");
        assert_eq!(raises(src, "g"), "Exception");
        assert_eq!(raises(src, "h"), "Exception");
    }

    #[test]
    fn tempered_must_declare_what_escapes() {
        let src = "temper def parse(s: str) -> int:\n    return int(s)\n";
        assert_eq!(
            err(src),
            "tempered `parse` can raise ValueError here, so it must declare it; \
             write `-> int raises ValueError`"
        );
        let ok = "temper def parse(s: str) -> int raises ValueError:\n    return int(s)\n";
        assert_eq!(raises(ok, "parse"), "ValueError");
    }

    #[test]
    fn a_clause_must_list_every_escape() {
        let src = "\
temper def f(s: str) -> list[int] | None raises ValueError:
    if s == \"\":
        raise IndexError
    return [int(s)]
";
        assert_eq!(
            err(src),
            "`f` can raise IndexError here, which its `raises` clause does not list; \
             write `raises IndexError, ValueError`"
        );
    }

    #[test]
    fn the_first_undeclared_escape_in_source_order_is_reported() {
        let src = "\
temper def f(s: str) -> float:
    if s == \"\":
        raise ZeroDivisionError
    return float(s)
";
        assert!(err(src).starts_with("tempered `f` can raise ZeroDivisionError here"));
    }

    #[test]
    fn exception_admits_values_of_unknown_kind() {
        let src = "temper def f(e: Exception) -> None:\n    raise e\n";
        assert_eq!(
            err(src),
            "tempered `f` can raise any exception here (an `Exception`, or a value whose \
             class is known only when it runs), so it must declare it; write \
             `-> None raises Exception`"
        );
        let ok = "temper def f(e: Exception) -> None raises Exception:\n    raise e\n";
        assert_eq!(raises(ok, "f"), "Exception");
    }

    #[test]
    fn callers_see_the_declared_set() {
        let src = "\
temper def parse(s: str) -> int raises ValueError, IndexError:
    return int(s)

temper def twice(s: str) -> int raises ValueError:
    return parse(s) * 2
";
        assert_eq!(
            err(src),
            "`twice` can raise IndexError here, which its `raises` clause does not list; \
             write `raises IndexError, ValueError`"
        );
        let loose = src.replace(
            "temper def twice(s: str) -> int raises ValueError",
            "def twice(s)",
        );
        assert_eq!(raises(&loose, "twice"), "IndexError, ValueError");
    }

    #[test]
    fn a_tempered_call_that_cannot_raise_needs_no_clause() {
        let src = "\
temper def safe(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return 0

temper def f(s: str) -> int:
    return safe(s) + 1
";
        assert_eq!(raises(src, "f"), "");
    }

    #[test]
    fn a_loose_clause_is_checked_too() {
        let src = "def f(s) raises IndexError:\n    return int(s)\n";
        assert_eq!(
            err(src),
            "`f` can raise ValueError here, which its `raises` clause does not list; \
             write `raises IndexError, ValueError`"
        );
    }

    #[test]
    fn a_clause_may_list_more_than_escapes() {
        let src = "temper def f(n: int) -> int raises ValueError:\n    return n\n";
        assert_eq!(raises(src, "f"), "ValueError");
    }

    #[test]
    fn clause_names_are_builtin_exceptions_listed_once() {
        let unknown = "temper def f() -> None raises TypeError:\n    pass\n";
        assert_eq!(
            err(unknown),
            "`TypeError` is not an exception; `raises` lists one of ArithmeticError, \
             Exception, IndexError, KeyError, LookupError, OverflowError, RecursionError, \
             RuntimeError, ValueError, ZeroDivisionError"
        );
        let twice = "temper def f() -> None raises ValueError, ValueError:\n    pass\n";
        assert_eq!(err(twice), "`ValueError` is listed twice in `raises`");
        let redefined =
            "def ValueError():\n    pass\ntemper def f() -> None raises ValueError:\n    pass\n";
        assert_eq!(
            err(redefined),
            "`ValueError` is redefined, so it no longer names the builtin exception"
        );
        let global = "IndexError = 1\ndef f() raises IndexError:\n    pass\n";
        assert_eq!(
            err(global),
            "`IndexError` is redefined, so it no longer names the builtin exception"
        );
    }

    #[test]
    fn a_call_before_a_shadowing_def_runs_reaches_the_builtin() {
        let late = "\
temper def f() -> int:
    return int(\"bad\")

print(f())

temper def int(s: str) -> int:
    return 0
";
        assert_eq!(
            err(late),
            "tempered `f` can raise ValueError here, so it must declare it; \
             write `-> int raises ValueError`"
        );
        let early = "\
temper def int(s: str) -> int:
    return 0

temper def f() -> int:
    return int(\"bad\")

print(f())
";
        assert_eq!(raises(early, "f"), "");
        // A default runs before the `def` after it binds its name.
        let default = "\
def g(n: int = 0):
    return n

def f():
    return int(\"bad\")

def int(s: str) -> int:
    return 0
";
        assert_eq!(raises(default, "f"), "ValueError");
    }

    #[test]
    fn a_late_shadow_makes_no_argument_surely_a_number() {
        let nested = "\
temper def f() -> int:
    return int(str(\"bad\"))

print(f())

temper def str(s: str) -> int:
    return 0
";
        assert_eq!(
            err(nested),
            "tempered `f` can raise ValueError here, so it must declare it; \
             write `-> int raises ValueError`"
        );
        let assigned = nested.replace(
            "    return int(str(\"bad\"))",
            "    x = str(\"bad\")\n    return int(x)",
        );
        assert!(err(&assigned).starts_with("tempered `f` can raise ValueError here"));
        let early = "\
temper def str(s: str) -> int:
    return 0

temper def f() -> int:
    return int(str(\"bad\"))
";
        assert_eq!(raises(early, "f"), ""); // Through a parameter, and an indexed composite result.
        let param = "\
temper def parse(n: int) -> int:
    return int(n)

print(parse(str(\"bad\")))

temper def str(s: str) -> int:
    return 0
";
        assert!(err(param).starts_with("tempered `parse` can raise ValueError here"));
        let indexed = "\
temper def f() -> int:
    return int(list([\"bad\"])[0])

print(f())

temper def list(xs: list[str]) -> list[int]:
    return [0]
";
        assert!(err(indexed).starts_with("tempered `f` can raise ValueError here"));
    }

    #[test]
    fn a_global_read_before_it_is_bound_is_the_builtin() {
        let src = "\
temper def f() -> None raises IndexError:
    raise ValueError

f()
ValueError = IndexError()
";
        assert_eq!(
            err(src),
            "`f` can raise ValueError here, which its `raises` clause does not list; \
             write `raises IndexError, ValueError`"
        );
        let module = "x = ValueError\nValueError = IndexError()\ndef f():\n    raise x\n";
        assert_eq!(raises(module, "f"), "IndexError, ValueError");
    }

    #[test]
    fn signature_errors_come_first() {
        let src = "temper def f(s) -> int:\n    return int(s)\n";
        assert_eq!(err(src), "tempered `f` must annotate parameter `s`");
    }

    #[test]
    fn an_unneeded_raises_exception_warns_with_the_narrower_clause() {
        let src = "temper def f(s: str) -> int raises Exception:\n    return int(s)\n";
        let want = vacuous("f", "only ValueError", "write `raises ValueError`");
        assert_eq!(warnings(src), vec![want]);
        let loose = "def f(s) raises Exception:\n    return int(s)\n";
        let want = vacuous("f", "only ValueError", "write `raises ValueError`");
        assert_eq!(warnings(loose), vec![want]);
    }

    #[test]
    fn a_raises_exception_nothing_needs_says_to_remove_the_clause() {
        let src = "temper def f(n: int) -> int raises Exception:\n    return n\n";
        let want = vacuous("f", "nothing", "remove the `raises` clause");
        assert_eq!(warnings(src), vec![want]);
    }

    #[test]
    fn the_narrower_clause_keeps_the_other_listed_classes() {
        let src = "\
def f(n: int) -> None raises IndexError, Exception:
    raise ValueError
def g(n: int) -> None raises ArithmeticError, Exception:
    raise ZeroDivisionError
def h(n: int) -> None raises IndexError, Exception:
    pass
";
        let f = vacuous(
            "f",
            "only ValueError",
            "write `raises IndexError, ValueError`",
        );
        let g = vacuous(
            "g",
            "only ZeroDivisionError",
            "write `raises ArithmeticError`",
        );
        let h = vacuous("h", "nothing", "write `raises IndexError`");
        assert_eq!(warnings(src), vec![f, g, h]);
    }

    #[test]
    fn a_needed_raises_exception_does_not_warn() {
        let src = "\
temper def relay(owned e: Exception) -> None raises Exception:
    raise e
def caught(n: int) -> None raises Exception:
    try:
        raise ValueError
    except Exception as e:
        raise e
";
        assert_eq!(warnings(src), vec![]);
    }

    #[test]
    fn only_the_callee_whose_clause_is_vacuous_warns() {
        let src = "\
def inner(s: str) -> int raises Exception:
    return int(s)
def outer(s: str) -> int raises Exception:
    return inner(s)
";
        let want = vacuous("inner", "only ValueError", "write `raises ValueError`");
        assert_eq!(warnings(src), vec![want]);
    }

    #[test]
    fn no_clause_or_an_undeclared_escape_does_not_warn() {
        assert_eq!(warnings("def f(e):\n    raise e\n"), vec![]);
        assert_eq!(
            warnings("temper def f(n: int) -> int:\n    return n\n"),
            vec![]
        );
        let src = "temper def f(n: int) -> None raises ValueError:\n    raise ValueError\n";
        assert_eq!(warnings(src), vec![]);
    }

    #[test]
    fn the_narrower_clause_has_no_duplicate_or_covered_class() {
        let src = "\
def f(n: int) -> None raises ValueError, Exception:
    raise ValueError
def g(n: int) -> None raises LookupError, Exception:
    if n == 0:
        raise IndexError
    raise ValueError
";
        let f = vacuous("f", "only ValueError", "write `raises ValueError`");
        let g = vacuous(
            "g",
            "only IndexError, ValueError",
            "write `raises LookupError, ValueError`",
        );
        assert_eq!(warnings(src), vec![f, g]);
    }

    #[test]
    fn a_narrower_clause_naming_a_redefined_class_does_not_warn() {
        let function = "\
def ValueError():
    pass
def f(s: str) -> int raises Exception:
    return int(s)
";
        assert_eq!(warnings(function), vec![]);
        let variable = "\
ZeroDivisionError = 1
def f(n: int) -> None raises Exception:
    raise IndexError
def g(n: int) -> None raises ArithmeticError, Exception:
    raise OverflowError
";
        let f = vacuous("f", "only IndexError", "write `raises IndexError`");
        let g = vacuous("g", "only OverflowError", "write `raises ArithmeticError`");
        assert_eq!(warnings(variable), vec![f, g]);
    }

    #[test]
    fn warnings_come_in_source_order() {
        let src = "\
def b(n: int) -> int raises Exception:
    return n
def a(n: int) -> int raises Exception:
    return b(n)
";
        let starts: Vec<usize> = types(src)
            .unwrap()
            .warnings()
            .iter()
            .map(|w| w.span.start)
            .collect();
        assert_eq!(starts.len(), 1, "{starts:?}");
        let src = src.replace("return b(n)", "return n");
        let warned = types(&src).unwrap();
        let starts: Vec<usize> = warned.warnings().iter().map(|w| w.span.start).collect();
        assert_eq!(starts.len(), 2);
        assert!(starts.windows(2).all(|w| w[0] < w[1]), "{starts:?}");
    }

    /// `src` with the `raises` clause on the line at `at` replaced as
    /// `message` suggests.
    fn apply(src: &str, at: usize, message: &str) -> String {
        let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
        let line_end = src[at..].find('\n').map_or(src.len(), |i| at + i);
        let line = &src[line_start..line_end];
        let clause = line.find(" raises ").unwrap_or_else(|| panic!("{line}"));
        let colon = line.rfind(':').unwrap_or_else(|| panic!("{line}"));
        let fix = match message.split_once("write `raises ") {
            Some((_, rest)) => format!(" raises {}", rest.trim_end_matches('`')),
            None if message.ends_with("remove the `raises` clause") => String::new(),
            None => panic!("no fix in {message}"),
        };
        let fixed = format!("{}{fix}{}", &line[..clause], &line[colon..]);
        format!("{}{fixed}{}", &src[..line_start], &src[line_end..])
    }

    /// Applying every suggested clause, innermost first as the warnings
    /// lead, leaves a program that checks with no warning.
    #[test]
    fn each_suggested_clause_checks_clean() {
        let programs = [
            "temper def f(s: str) -> int raises Exception:\n    return int(s)\n",
            "def f(s) raises Exception:\n    return int(s)\n",
            "temper def f(n: int) -> int raises Exception:\n    return n\n",
            "def f(n: int) -> None raises IndexError, Exception:\n    raise ValueError\n",
            "def f(n: int) -> None raises ArithmeticError, Exception:\n    raise ZeroDivisionError\n",
            "def f(n: int) -> None raises ValueError, Exception:\n    raise ValueError\n",
            "def f(n: int) -> None raises LookupError, Exception:\n    if n == 0:\n        raise IndexError\n    raise ValueError\n",
            "def ValueError():\n    pass\ndef f(n: int) -> None raises Exception:\n    raise IndexError\n",
            "LookupError = 1\ndef f(n: int) -> None raises Exception:\n    raise IndexError\n",
            "temper def inner(s: str) -> int raises Exception:\n    return int(s)\ntemper def outer(s: str) -> int raises Exception:\n    return inner(s)\nprint(outer(\"1\"))\n",
        ];
        for program in programs {
            let mut src = program.to_string();
            for _ in 0..4 {
                let types = types(&src).unwrap_or_else(|e| panic!("{e}\n{src}"));
                let Some(w) = types.warnings().first() else {
                    break;
                };
                src = apply(&src, w.span.start, &w.message);
            }
            let types = types(&src).unwrap_or_else(|e| panic!("{e}\n{src}"));
            assert_eq!(types.warnings(), &[], "{src}");
            assert_ne!(src, program, "no warning for:\n{program}");
        }
    }
}
