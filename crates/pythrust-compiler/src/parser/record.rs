//! Frozen records (step 4, D129-D136): `@dataclass(frozen=True)` then
//! `class NAME:` with annotated fields and methods.
//!
//! `@dataclass` is a recognized form, not a general decorator (D132).
//! A method `m` of `P` becomes the top-level `def` named `P.m` (see
//! [`method_name`]) right after the record, its `self` annotated `P`, so
//! every later stage sees an ordinary function.

use super::{error, Parser};
use crate::ast::{
    method_name, Expr, ExprKind, Field, FuncDef, Ident, LocalModule, Module, ParamMode, RecordDef,
    Stmt, StmtKind, Target, TargetKind, TypeExpr,
};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::infer::scope;
use crate::span::Span;
use crate::token::TokenKind;

/// D146's message for a spelling other than `@dataclass` and
/// `@dataclass(frozen=True)`.
const SPELLING: &str =
    "write `@dataclass` or `@dataclass(frozen=True)`; other spellings are not supported yet";

/// What a record's body holds, gathered member by member.
#[derive(Default)]
struct Body {
    fields: Vec<Field>,
    methods: Vec<FuncDef>,
}

impl Body {
    /// Whether `name` is a field or method already.
    fn has(&self, name: &str) -> bool {
        let field = self.fields.iter().any(|f| f.name.name == name);
        let method = self.methods.iter().any(|m| m.name.name == name);
        field || method
    }
}

impl Parser<'_> {
    /// `'@' 'dataclass' ['(' 'frozen' '=' 'True' ')'] NEWLINE 'class'
    /// NAME ':' body`: the record, then its methods.
    pub(super) fn record(&mut self) -> Result<Vec<Stmt>> {
        let start = self.tok(0).span;
        let frozen = self.decorator()?;
        if !self.at(&TokenKind::Class) {
            let msg = "decorators are not supported yet, except `@dataclass(frozen=True)` \
                       on a class";
            return Err(error(start, msg));
        }
        self.bump();
        let name = self.binding_name("a class name")?;
        self.no_bases(&name)?;
        self.expect(&TokenKind::Colon)?;
        let body = self.record_body(&name, frozen)?;
        let span = self.span_from(start);
        let record = RecordDef {
            name: name.clone(),
            fields: body.fields,
            methods: body.methods.iter().map(|m| m.name.clone()).collect(),
            frozen,
            span,
        };
        let methods = body.methods.into_iter().map(|m| method(&name, m));
        let first = Stmt {
            kind: StmtKind::Record(record),
            span,
        };
        Ok(std::iter::once(first).chain(methods).collect())
    }

    /// The decorator line before `class`: `@dataclass(frozen=True)`, or
    /// bare `@dataclass` (D146), which makes a mutable record; whether it
    /// is frozen. A `class` with none is refused.
    fn decorator(&mut self) -> Result<bool> {
        let at = self.tok(0).span;
        if !self.eat(&TokenKind::At) {
            let msg = "classes without `@dataclass(frozen=True)` are not supported yet; \
                       use `@dataclass(frozen=True)`";
            return Err(error(at, msg));
        }
        let name = self.expect_name("a decorator name")?;
        if name.name != "dataclass" {
            let msg = "decorators are not supported yet, except `@dataclass(frozen=True)` \
                       on a class";
            return Err(error(at.merge(name.span), msg));
        }
        if !self.dataclass {
            let msg = "name `dataclass` is not defined; add `from dataclasses import dataclass`";
            return Err(error(name.span, msg));
        }
        if !self.at(&TokenKind::LParen) {
            self.expect(&TokenKind::Newline)?;
            return Ok(false);
        }
        let call = self.call(Expr {
            kind: ExprKind::Name(name.name.clone()),
            span: name.span,
        })?;
        frozen_only(&call)?;
        self.expect(&TokenKind::Newline)?;
        Ok(true)
    }

    /// No base classes and no type parameters (D132, D135); `()` is
    /// none.
    fn no_bases(&mut self, name: &Ident) -> Result<()> {
        if self.at(&TokenKind::LBracket) {
            return Err(error(
                self.tok(0).span,
                "generic records are not supported yet",
            ));
        }
        if !self.eat(&TokenKind::LParen) {
            return Ok(());
        }
        if self.eat(&TokenKind::RParen) {
            return Ok(());
        }
        let generic = matches!(self.peek(), TokenKind::Name(n) if n == "Generic");
        let exception =
            matches!(self.peek(), TokenKind::Name(n) if ExcKind::from_name(n).is_some());
        if exception {
            let msg = "decorators on an exception class are not supported yet";
            return Err(error(self.tok(0).span, msg));
        }
        let msg = match generic {
            true => "generic records are not supported yet".to_string(),
            false => format!(
                "base classes are not supported yet; `{}` stands alone",
                name.name
            ),
        };
        Err(error(self.tok(0).span, msg))
    }

    /// `NEWLINE INDENT member+ DEDENT`: a leading docstring, `pass`,
    /// fields and methods.
    fn record_body(&mut self, record: &Ident, frozen: bool) -> Result<Body> {
        self.ellipsis_stmt(" on its own indented line")?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        let mut body = Body::default();
        if matches!(self.peek(), TokenKind::Str(_)) && self.tok(1).kind == TokenKind::Newline {
            self.bump();
            self.bump();
        }
        while !self.eat(&TokenKind::Dedent) {
            self.member(record, frozen, &mut body)?;
        }
        Ok(body)
    }

    /// One member of a record's body.
    fn member(&mut self, record: &Ident, frozen: bool, body: &mut Body) -> Result<()> {
        let tok = self.tok(0);
        match &tok.kind {
            TokenKind::Pass => {
                self.bump();
                self.expect(&TokenKind::Newline)?;
                Ok(())
            }
            TokenKind::Def | TokenKind::Temper => {
                let def = self.nested(|p| p.func_head(|p| p.expect_name("a method name")))?;
                check_method(record, frozen, &def, body)?;
                body.methods.push(def);
                Ok(())
            }
            TokenKind::At => {
                let msg = "decorators on methods are not supported yet";
                Err(error(tok.span, msg))
            }
            TokenKind::Name(_) if self.tok(1).kind == TokenKind::Colon => {
                let field = self.field()?;
                check_member(&field.name, body)?;
                body.fields.push(field);
                Ok(())
            }
            _ => {
                let msg = "class attributes are not supported yet; a record's body holds \
                           annotated fields (`x: int`) and methods";
                Err(error(tok.span, msg))
            }
        }
    }

    /// `NAME ':' type NEWLINE`; a default is not supported yet (D132).
    fn field(&mut self) -> Result<Field> {
        let name = self.expect_name("a field name")?;
        self.expect(&TokenKind::Colon)?;
        let annotation = self.type_expr()?;
        if annotation.name.name == "ClassVar" {
            let msg = "`ClassVar` is not supported yet";
            return Err(error(annotation.span, msg));
        }
        if self.at(&TokenKind::Assign) {
            let msg = "field defaults are not supported yet; pass every field when \
                       constructing the record";
            return Err(error(self.tok(0).span, msg));
        }
        self.expect(&TokenKind::Newline)?;
        Ok(Field { name, annotation })
    }
}

/// The decorator's arguments: `frozen=True` alone (D132, D146).
fn frozen_only(call: &Expr) -> Result<()> {
    let ExprKind::Call { args, .. } = &call.kind else {
        return Err(error(call.span, SPELLING));
    };
    let mut frozen = false;
    for arg in args {
        let Some(keyword) = &arg.name else {
            let msg = "`@dataclass` takes only `frozen=True`";
            return Err(error(arg.span, msg));
        };
        if keyword.name != "frozen" {
            let msg = format!(
                "`@dataclass({}=...)` is not supported yet; only `frozen=True` is",
                keyword.name
            );
            return Err(error(arg.span, msg));
        }
        frozen = matches!(arg.value.kind, ExprKind::Bool(true));
    }
    match frozen {
        true => Ok(()),
        false => Err(error(call.span, SPELLING)),
    }
}

/// A member's name is new to the record, and not a dunder (D132).
fn check_member(name: &Ident, body: &Body) -> Result<()> {
    let n = &name.name;
    if n.starts_with("__") && n.ends_with("__") {
        let msg = format!("`{n}` in a record is not supported yet");
        return Err(error(name.span, msg));
    }
    if body.has(n) {
        let msg = format!("`{n}` is declared twice in this record; give each member its own name");
        return Err(error(name.span, msg));
    }
    Ok(())
}

/// A method's name is new, and it takes an unannotated `self` first,
/// which a frozen record's method only reads (D131); a mutable one's may
/// change or keep it (D149).
fn check_method(record: &Ident, frozen: bool, def: &FuncDef, body: &Body) -> Result<()> {
    check_member(&def.name, body)?;
    let Some(first) = def.params.first() else {
        let msg = format!(
            "method `{}` needs `self` as its first parameter",
            def.name.name
        );
        return Err(error(def.name.span, msg));
    };
    if first.name.name != "self" {
        let msg = format!(
            "method `{}` needs `self` as its first parameter, not `{}`",
            def.name.name, first.name.name
        );
        return Err(error(first.name.span, msg));
    }
    if frozen && first.mode != ParamMode::Read {
        let msg = "a method that changes or keeps `self` is not supported yet";
        return Err(error(first.span, msg));
    }
    if first.annotation.is_some() || first.default.is_some() {
        let msg = format!(
            "`self` is the record `{}`; drop its annotation",
            record.name
        );
        return Err(error(first.span, msg));
    }
    match changes_self(&def.body).filter(|_| frozen) {
        Some(at) => {
            let msg = "a method that changes or keeps `self` is not supported yet";
            Err(error(at, msg))
        }
        None => Ok(()),
    }
}

/// The builtin methods that change their receiver in place.
const MUTATORS: [&str; 9] = [
    "append",
    "pop",
    "extend",
    "insert",
    "remove",
    "sort",
    "add",
    "setdefault",
    "update",
];

/// Where `body` visibly changes a value reached through `self` (D131):
/// a changing method call, an item store or a `del` on it. A change
/// through a call that takes it is found by `own` once records lower
/// (step 4-2).
fn changes_self(body: &[Stmt]) -> Option<Span> {
    let call = scope::exprs(body).into_iter().find_map(|e| {
        let ExprKind::Call { func, .. } = &e.kind else {
            return None;
        };
        let ExprKind::Attr { value, attr } = &func.kind else {
            return None;
        };
        let reached = !matches!(&value.kind, ExprKind::Name(n) if n == "self");
        let changes = MUTATORS.contains(&attr.name.as_str());
        (reached && changes && from_self(value)).then_some(e.span)
    });
    call.or_else(|| stores(body))
}

/// An item store, augmented or not, or a `del` on a value reached
/// through `self`, in `body` or a block inside it.
fn stores(body: &[Stmt]) -> Option<Span> {
    let mut found = None;
    scope::walk(body, &mut |s| {
        let hit = match &s.kind {
            StmtKind::Assign { target, .. } | StmtKind::AugAssign { target, .. } => {
                target_from_self(target)
            }
            StmtKind::Del { value, .. } => from_self(value),
            _ => false,
        };
        if hit && found.is_none() {
            found = Some(s.span);
        }
    });
    found
}

fn target_from_self(t: &Target) -> bool {
    match &t.kind {
        TargetKind::Name(_) => false,
        TargetKind::Subscript { value, .. } => from_self(value),
        // `self.x = v` is a field store, which inference refuses on a
        // frozen record (D158); `self.a.x = v` changes what `self` reaches.
        TargetKind::Attr { value, .. } => {
            !matches!(&value.kind, ExprKind::Name(n) if n == "self") && from_self(value)
        }
        TargetKind::Tuple(ts) => ts.iter().any(target_from_self),
    }
}

/// Whether `e` is `self`, or reached from it by fields and items.
fn from_self(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name(n) => n == "self",
        ExprKind::Attr { value, .. } | ExprKind::Subscript { value, .. } => from_self(value),
        _ => false,
    }
}

/// Method `def` of `record` as the top-level `def` named `P.m`, `self`
/// annotated with the record.
fn method(record: &Ident, mut def: FuncDef) -> Stmt {
    def.name.name = method_name(&record.name, &def.name.name);
    if let Some(slf) = def.params.first_mut() {
        slf.annotation = Some(record_type(record, slf.name.span));
    }
    let span = def.span;
    Stmt {
        kind: StmtKind::FuncDef(def),
        span,
    }
}

/// The annotation naming `record`, at `span`.
fn record_type(record: &Ident, span: Span) -> TypeExpr {
    TypeExpr {
        name: Ident {
            name: record.name.clone(),
            span,
        },
        args: Vec::new(),
        optional: false,
        span,
    }
}

/// The module-wide rules of a file's records (D129, D136): a record's
/// name is no other binding's, and an annotation evaluated when its
/// statement runs (a field's, a `def`'s parameters and result) names
/// only records defined before it.
pub(super) fn check_module(body: &[Stmt]) -> Result<()> {
    check_exc_classes(body)?;
    let records: Vec<(usize, &RecordDef)> = body
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match &s.kind {
            StmtKind::Record(r) => Some((i, r)),
            _ => None,
        })
        .collect();
    if records.is_empty() {
        return Ok(());
    }
    for &(_, r) in &records {
        check_unbound(&r.name, r.span, "a record", body)?;
    }
    for (at, s) in body.iter().enumerate() {
        let at = runs_at(s, at, &records);
        for t in evaluated(s) {
            check_defined(t, at, &records)?;
        }
    }
    Ok(())
}

/// Where statement `s`, at `at` in the body, evaluates its annotations:
/// a method's run in its record's body, before the record exists.
fn runs_at(s: &Stmt, at: usize, records: &[(usize, &RecordDef)]) -> usize {
    let StmtKind::FuncDef(f) = &s.kind else {
        return at;
    };
    let Some((record, _)) = f.name.name.split_once('.') else {
        return at;
    };
    let own = records.iter().find(|(_, r)| r.name.name == record);
    own.map_or(at, |(i, _)| *i)
}

/// No record of the linked `module` has a local module's name, which
/// its methods' names `P.m` would share with that module's globals.
pub(super) fn check_modules(module: &Module, locals: &[LocalModule]) -> Result<()> {
    for s in &module.body {
        let StmtKind::Record(r) = &s.kind else {
            continue;
        };
        if locals.iter().any(|m| m.name == r.name.name) {
            let msg = format!(
                "`{}` names both a record and a module; rename the record",
                r.name.name
            );
            return Err(error(r.name.span, msg));
        }
    }
    Ok(())
}

/// No exception class is named like a builtin one, or rebound (D142).
fn check_exc_classes(body: &[Stmt]) -> Result<()> {
    for s in body {
        let StmtKind::ExcClass(c) = &s.kind else {
            continue;
        };
        if ExcKind::from_name(&c.name.name).is_some() {
            let msg = format!(
                "`{}` names a builtin exception class; choose another name",
                c.name.name
            );
            return Err(error(c.name.span, msg));
        }
        check_unbound(&c.name, c.span, "an exception class", body)?;
    }
    Ok(())
}

/// No variable, parameter, function or other class takes the name of
/// `what` (a record or an exception class) `class`, defined at `at`.
fn check_unbound(class: &Ident, at: Span, what: &str, body: &[Stmt]) -> Result<()> {
    let name = class.name.as_str();
    let mut bound = scope::assigned(body).into_iter().map(|n| source(&n));
    let mut clash = bound.any(|n| n == name);
    for s in body {
        match &s.kind {
            StmtKind::FuncDef(f) => {
                clash |= f.name.name == name;
                clash |= f.params.iter().any(|p| p.name.name == name);
                clash |= scope::assigned(&f.body).iter().any(|n| source(n) == name);
            }
            StmtKind::Record(other) => clash |= other.span != at && other.name.name == name,
            StmtKind::ExcClass(other) => clash |= other.span != at && other.name.name == name,
            _ => {}
        }
    }
    match clash {
        false => Ok(()),
        true => {
            let msg = format!("`{name}` names {what}; choose another name");
            Err(error(class.span, msg))
        }
    }
}

fn source(n: &str) -> String {
    crate::ast::source_name(n).to_string()
}

/// The annotations statement `s` evaluates when it runs.
fn evaluated(s: &Stmt) -> Vec<&TypeExpr> {
    match &s.kind {
        StmtKind::Record(r) => r.fields.iter().map(|f| &f.annotation).collect(),
        StmtKind::FuncDef(f) => {
            // A method's `self` is annotated by the parser, not the source.
            let skip = usize::from(f.name.name.contains('.'));
            let params = f.params.iter().skip(skip);
            let params = params.filter_map(|p| p.annotation.as_ref());
            params.chain(f.returns.as_ref()).collect()
        }
        _ => Vec::new(),
    }
}

/// `t` names no record that the statement at `at` runs before.
fn check_defined(t: &TypeExpr, at: usize, records: &[(usize, &RecordDef)]) -> Result<()> {
    let later = records
        .iter()
        .any(|(i, r)| *i >= at && r.name.name == t.name.name);
    if later {
        let msg = format!(
            "`{}` is not defined yet: a record is defined when its `class` runs",
            t.name.name
        );
        return Err(error(t.name.span, msg));
    }
    t.args
        .iter()
        .try_for_each(|a| check_defined(a, at, records))
}

#[cfg(test)]
mod tests {
    use crate::ast::{Module, StmtKind};
    use crate::{lexer, parser};

    const IMPORT: &str = "from dataclasses import dataclass\n";

    fn parse(src: &str) -> Result<Module, String> {
        let tokens = lexer::lex(src).map_err(|d| d.message)?;
        parser::parse(&tokens).map_err(|d| d.message)
    }

    /// The parser's message for `src` after the import.
    fn err(src: &str) -> String {
        match parse(&format!("{IMPORT}{src}")) {
            Ok(_) => panic!("expected an error for:\n{src}"),
            Err(msg) => msg,
        }
    }

    fn record(body: &str) -> String {
        format!("@dataclass(frozen=True)\nclass P:\n{body}")
    }

    #[test]
    fn a_record_is_followed_by_its_methods_as_defs() {
        let src = format!(
            "{IMPORT}{}",
            record("    \"Doc.\"\n    x: int\n    def m(self, k: int) -> int:\n        return k\n")
        );
        let m = parse(&src).unwrap();
        let StmtKind::Record(r) = &m.body[0].kind else {
            panic!("a record first: {:?}", m.body)
        };
        assert_eq!(r.name.name, "P");
        assert_eq!(r.fields.len(), 1);
        assert_eq!(r.methods[0].name, "m");
        let StmtKind::FuncDef(f) = &m.body[1].kind else {
            panic!("then its method")
        };
        assert_eq!(f.name.name, "P.m");
        let slf = f.params[0]
            .annotation
            .as_ref()
            .map(|t| t.name.name.as_str());
        assert_eq!(slf, Some("P"));
        assert!(parse(&format!("{IMPORT}{}", record("    pass\n"))).is_ok());
        assert!(parse(&format!(
            "{IMPORT}@dataclass(frozen=True)\nclass P():\n    pass\n"
        ))
        .is_ok());
    }

    #[test]
    fn a_record_is_frozen_or_a_bare_dataclass() {
        let spelling = "write `@dataclass` or `@dataclass(frozen=True)`";
        for (src, frozen) in [("@dataclass", false), ("@dataclass(frozen=True)", true)] {
            let m = parse(&format!("{IMPORT}{src}\nclass P:\n    x: int\n"));
            let m = m.unwrap_or_else(|e| panic!("{src}: {e}"));
            let StmtKind::Record(r) = &m.body[0].kind else {
                panic!("{src}: not a record");
            };
            assert_eq!(r.frozen, frozen, "{src}");
        }
        assert!(err("@dataclass(frozen=False)\nclass P:\n    x: int\n").contains(spelling));
        assert!(err("@dataclass()\nclass P:\n    x: int\n").contains(spelling));
        let eq = err("@dataclass(frozen=True, eq=True)\nclass P:\n    x: int\n");
        assert!(
            eq.contains("`@dataclass(eq=...)` is not supported yet"),
            "{eq}"
        );
        assert!(err("@dataclass(True)\nclass P:\n    x: int\n").contains("only `frozen=True`"));
        assert!(err("class P:\n    x: int\n").contains("classes without"));
        assert!(err("@other\ndef f():\n    pass\n").contains("decorators are not supported yet"));
        let missing = parse("@dataclass(frozen=True)\nclass P:\n    x: int\n").unwrap_err();
        assert!(
            missing.contains("add `from dataclasses import dataclass`"),
            "{missing}"
        );
    }

    #[test]
    fn the_dataclasses_import_has_one_form() {
        let msg = parse("import dataclasses\n").unwrap_err();
        assert!(
            msg.contains("write `from dataclasses import dataclass`"),
            "{msg}"
        );
        let msg = parse("from dataclasses import dataclass as dc\n").unwrap_err();
        assert!(
            msg.contains("only `from dataclasses import dataclass`"),
            "{msg}"
        );
        let msg = parse("from dataclasses import replace\n").unwrap_err();
        assert!(
            msg.contains("`dataclasses.replace` is not supported yet"),
            "{msg}"
        );
    }

    #[test]
    fn a_records_body_holds_fields_and_methods_only() {
        let cases = [
            ("    x: int = 0\n", "field defaults are not supported yet"),
            ("    k = 3\n", "class attributes are not supported yet"),
            ("    x: ClassVar[int]\n", "`ClassVar` is not supported yet"),
            ("    x: int\n    x: str\n", "`x` is declared twice"),
            (
                "    def x(self):\n        pass\n    x: int\n",
                "`x` is declared twice",
            ),
            (
                "    __slots__: int\n",
                "`__slots__` in a record is not supported yet",
            ),
            (
                "    def __eq__(self, o):\n        return True\n",
                "`__eq__` in a record",
            ),
            (
                "    @property\n    def m(self):\n        pass\n",
                "decorators on methods",
            ),
            (
                "    def m():\n        pass\n",
                "needs `self` as its first parameter",
            ),
            ("    def m(me):\n        pass\n", "not `me`"),
            (
                "    def m(self: int):\n        pass\n",
                "drop its annotation",
            ),
        ];
        for (body, want) in cases {
            let msg = err(&record(body));
            assert!(msg.contains(want), "{body}: {msg}");
        }
        assert!(err("@dataclass(frozen=True)\nclass P[T]:\n    x: T\n").contains("generic"));
        assert!(
            err("@dataclass(frozen=True)\nclass P(Generic[T]):\n    x: T\n").contains("generic")
        );
        assert!(err("@dataclass(frozen=True)\nclass P(Q):\n    x: int\n").contains("base classes"));
    }

    #[test]
    fn a_method_changing_self_is_refused() {
        let msg = "a method that changes or keeps `self` is not supported yet";
        for body in [
            "        self.xs.append(1)\n",
            "        self.xs[0] = 1\n",
            "        self.xs[0] += 1\n",
            "        del self.xs[0]\n",
            "        if True:\n            self.d.update({})\n",
        ] {
            let src = record(&format!("    xs: list[int]\n    def m(self):\n{body}"));
            assert_eq!(err(&src), msg, "{body}");
        }
        let src = record("    xs: list[int]\n    def m(mut self):\n        pass\n");
        assert_eq!(err(&src), msg);
        // Reading, or changing a copy, is fine.
        let ok = record(
            "    xs: list[int]\n    def m(self):\n        ys = list(self.xs)\n\
             \x20       ys.append(len(self.xs))\n        return ys\n",
        );
        assert!(parse(&format!("{IMPORT}{ok}")).is_ok());
    }

    #[test]
    fn a_class_is_defined_at_top_level() {
        let src = format!(
            "def f():\n    {}",
            record("    x: int\n").replace('\n', "\n    ")
        );
        let msg = err(&src);
        assert!(msg.contains("top level of a module"), "{msg}");
    }

    #[test]
    fn annotations_name_records_defined_before_them() {
        let later = "`Q` is not defined yet";
        let src = format!(
            "def f(q: Q) -> int:\n    return 1\n{}",
            "@dataclass(frozen=True)\nclass Q:\n    x: int\n"
        );
        assert!(err(&src).contains(later));
        assert!(err(&record("    nxt: P | None\n")).contains("`P` is not defined yet"));
        let own = record("    x: int\n    def m(self, o: P) -> int:\n        return 1\n");
        assert!(err(&own).contains("`P` is not defined yet"));
        let own = record("    x: int\n    def m(self) -> list[P]:\n        return []\n");
        assert!(err(&own).contains("`P` is not defined yet"));
        // A body may name a later record; an earlier one anywhere.
        let ok = format!(
            "def make():\n    return Q(1)\n{}{}",
            "@dataclass(frozen=True)\nclass Q:\n    x: int\n",
            "@dataclass(frozen=True)\nclass R:\n    q: Q\n    def m(self, o: Q) -> Q:\n        return o\n"
        );
        assert!(parse(&format!("{IMPORT}{ok}")).is_ok());
        assert!(err(&record("    x: \"P\"\n")).contains("string annotations are not supported yet"));
    }

    #[test]
    fn a_record_is_not_named_like_a_module() {
        let util = lexer::lex("def m():\n    return 1\n").unwrap();
        let other = lexer::lex("import util\nx = util.m()\n").unwrap();
        let main = format!(
            "{IMPORT}import other\n{}",
            record("    x: int\n").replace("class P", "class util")
        );
        let main = lexer::lex(&main).unwrap();
        let units = [
            parser::Unit {
                name: "util",
                tokens: &util,
            },
            parser::Unit {
                name: "other",
                tokens: &other,
            },
        ];
        let msg = parser::parse_program(&units, &[], &main, &[])
            .unwrap_err()
            .message;
        assert_eq!(
            msg,
            "`util` names both a record and a module; rename the record"
        );
    }

    #[test]
    fn a_records_name_is_no_other_binding() {
        let base = record("    x: int\n");
        for extra in [
            "P = 1\n",
            "def P():\n    pass\n",
            "def f(P):\n    pass\n",
            "def f():\n    P = 2\n",
            "for P in []:\n    pass\n",
            "print([P for P in []])\n",
            &record("    y: int\n"),
        ] {
            let msg = err(&format!("{base}{extra}"));
            assert_eq!(msg, "`P` names a record; choose another name", "{extra}");
        }
    }
}
