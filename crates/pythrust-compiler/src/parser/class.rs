//! Plain classes (step 5-2, D144-D149): `class NAME ':'` and a body of
//! methods. A class's fields are the names its `__init__` stores through
//! `self` (D144); [`super::init`] checks that each is assigned on every
//! path before `self` is used (D145).

use super::record::{check_method, method, Body};
use super::{error, init, Parser};
use crate::ast::{
    Block, Expr, ExprKind, Field, FuncDef, Ident, Mode, RecordDef, RecordKind, Stmt, StmtKind,
    TypeExpr,
};
use crate::diag::Result;
use crate::span::Span;
use crate::token::TokenKind;

/// The dunders a plain class may define (D147, D153).
const DUNDERS: [&str; 5] = ["__init__", "__repr__", "__str__", "__enter__", "__exit__"];

impl Parser<'_> {
    /// `'class' NAME ['(' ')'] ':' body`: the class, then its methods as
    /// top-level `def`s `P.m`, as a record's (D149).
    pub(super) fn plain_class(&mut self) -> Result<Vec<Stmt>> {
        let start = self.bump().span;
        let name = self.binding_name("a class name")?;
        self.no_bases(&name, "classes")?;
        self.expect(&TokenKind::Colon)?;
        let (body, annotations) = self.class_body(&name)?;
        let init = body.methods.iter().find(|m| m.name.name == "__init__");
        let fields = match init {
            Some(def) => init::fields(&name, def, &annotations)?,
            None => Vec::new(),
        };
        let span = self.span_from(start);
        let class = RecordDef {
            name: name.clone(),
            fields,
            methods: body.methods.iter().map(|m| m.name.clone()).collect(),
            kind: RecordKind::Plain,
            span,
        };
        let first = Stmt {
            kind: StmtKind::Record(class),
            span,
        };
        let methods = body.methods.into_iter().map(|m| method(&name, m));
        Ok(std::iter::once(first).chain(methods).collect())
    }

    /// `NEWLINE INDENT [docstring] member+ DEDENT`, and the annotations
    /// `__init__` gave its fields (`self.x: T = v`).
    fn class_body(&mut self, class: &Ident) -> Result<(Body, Vec<(Ident, TypeExpr)>)> {
        self.ellipsis_stmt(" on its own indented line")?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        if matches!(self.peek(), TokenKind::Str(_)) && self.tok(1).kind == TokenKind::Newline {
            self.bump();
            self.bump();
        }
        let mut body = Body::default();
        let mut annotations = Vec::new();
        while !self.eat(&TokenKind::Dedent) {
            if let Some(def) = self.class_member(class, &mut annotations)? {
                check_method(class, RecordKind::Plain, &def, &body)?;
                body.methods.push(def);
            }
        }
        Ok((body, annotations))
    }

    /// One member: `pass`, or a method; anything else is a named error
    /// (D144, D147).
    fn class_member(
        &mut self,
        class: &Ident,
        annotations: &mut Vec<(Ident, TypeExpr)>,
    ) -> Result<Option<FuncDef>> {
        let tok = self.tok(0);
        match &tok.kind {
            TokenKind::Pass => {
                self.bump();
                self.expect(&TokenKind::Newline)?;
                Ok(None)
            }
            TokenKind::Def | TokenKind::Temper => {
                let at = usize::from(tok.kind == TokenKind::Temper) + 1;
                let init = matches!(&self.tok(at).kind, TokenKind::Name(n) if n == "__init__");
                let saved = std::mem::replace(&mut self.init, init.then(Vec::new));
                let def = self.nested(|p| p.func_head(|p| p.expect_name("a method name")));
                let stored = std::mem::replace(&mut self.init, saved);
                let def = def?;
                annotations.extend(stored.unwrap_or_default());
                let mut def = def;
                check_dunder(class, &mut def)?;
                Ok(Some(def))
            }
            TokenKind::At => Err(error(
                tok.span,
                "decorators on methods are not supported yet",
            )),
            TokenKind::Name(_) if self.tok(1).kind == TokenKind::Colon => {
                let msg = "class-body field declarations are not supported yet; \
                           assign `self.x` in `__init__`";
                Err(error(tok.span, msg))
            }
            _ => {
                let msg = "class attributes are not supported yet; a class's body holds \
                           methods, and `__init__` assigns its fields";
                Err(error(tok.span, msg))
            }
        }
    }
}

/// A dunder method is one D147 admits; `__init__` returns nothing and
/// the printable ones return `str`, as CPython requires.
fn check_dunder(class: &Ident, def: &mut FuncDef) -> Result<()> {
    let n = def.name.name.as_str();
    if !(n.starts_with("__") && n.ends_with("__")) {
        return Ok(());
    }
    if !DUNDERS.contains(&n) {
        let msg = format!(
            "`{n}` is not supported yet; a class defines `__init__`, `__repr__`, `__str__`, \
             `__enter__` and `__exit__` only"
        );
        return Err(error(def.name.span, msg));
    }
    match n {
        "__enter__" => return Ok(()),
        "__exit__" => return check_exit(class, def),
        _ => {}
    }
    let returns = def.returns.as_ref().map(|t| t.name.name.as_str());
    let want = if n == "__init__" { "None" } else { "str" };
    match returns {
        Some(r) if r != want => {
            let msg = format!("`{}.{n}` returns `{want}`", class.name);
            Err(error(def.name.span, msg))
        }
        _ => Ok(()),
    }
}

/// `__exit__(self, exc_type, exc, tb)` is CPython's four-parameter form,
/// loose (a tempered one waits for spellable exception types, D153), and
/// returns `bool` or nothing. Every path that returns nothing returns
/// `False` (CPython: `None`, falsy), so `with` reads one type.
fn check_exit(class: &Ident, def: &mut FuncDef) -> Result<()> {
    let shown = &class.name;
    if def.mode == Mode::Tempered {
        let msg = format!("a tempered `{shown}.__exit__` is not supported yet");
        return Err(error(def.name.span, msg));
    }
    if def.params.len() != 4 {
        let msg = format!("`{shown}.__exit__` takes `self, exc_type, exc, tb`");
        return Err(error(def.name.span, msg));
    }
    match def.returns.as_ref().map(|t| t.name.name.as_str()) {
        Some("bool") => return Ok(()),
        Some("None") | None => {}
        Some(_) => {
            let msg = format!("`{shown}.__exit__` returns `bool` or nothing");
            return Err(error(def.name.span, msg));
        }
    }
    def.returns = None;
    false_returns(&mut def.body, def.span);
    if !matches!(def.body.last().map(|s| &s.kind), Some(StmtKind::Return(_))) {
        def.body.push(return_false(def.span));
    }
    Ok(())
}

/// Every bare `return` in the body as `return False`.
fn false_returns(block: &mut [Stmt], at: Span) {
    for s in block {
        match &mut s.kind {
            StmtKind::Return(v) if v.is_none() => *v = Some(false_expr(at)),
            kind => children_mut(kind)
                .into_iter()
                .for_each(|b| false_returns(b, at)),
        }
    }
}

fn return_false(span: Span) -> Stmt {
    let kind = StmtKind::Return(Some(false_expr(span)));
    Stmt { kind, span }
}

fn false_expr(span: Span) -> Expr {
    let kind = ExprKind::Bool(false);
    Expr { kind, span }
}

fn children_mut(kind: &mut StmtKind) -> Vec<&mut Block> {
    match kind {
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => std::iter::once(&mut branch.body)
            .chain(elifs.iter_mut().map(|b| &mut b.body))
            .chain([orelse])
            .collect(),
        StmtKind::While { branch, orelse } => vec![&mut branch.body, orelse],
        StmtKind::For { body, orelse, .. } => vec![body, orelse],
        StmtKind::With { body, .. } => vec![body],
        StmtKind::Try {
            body,
            handlers,
            finally,
        } => std::iter::once(body)
            .chain(handlers.iter_mut().map(|h| &mut h.body))
            .chain(finally)
            .collect(),
        _ => Vec::new(),
    }
}

/// A field `__init__` stores, annotated if a `self.x: T = v` store gave
/// it a type.
pub(super) fn field(name: &Ident, annotations: &[(Ident, TypeExpr)]) -> Field {
    let annotation = annotations
        .iter()
        .find(|(n, _)| n.name == name.name)
        .map(|(_, t)| t.clone());
    Field {
        name: name.clone(),
        annotation,
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Module, RecordDef, RecordKind, StmtKind};
    use crate::{lexer, parser};

    fn parse(src: &str) -> Result<Module, String> {
        let tokens = lexer::lex(src).map_err(|d| d.message)?;
        parser::parse(&tokens).map_err(|d| d.message)
    }

    fn err(src: &str) -> String {
        match parse(src) {
            Ok(_) => panic!("expected an error for:\n{src}"),
            Err(msg) => msg,
        }
    }

    /// `class P:` with `__init__`'s body `init`, as parsed.
    fn class(init: &str) -> Result<RecordDef, String> {
        let src = format!("class P:\n    def __init__(self, k: int):\n{init}");
        let m = parse(&src)?;
        match &m.body[0].kind {
            StmtKind::Record(r) => Ok(r.clone()),
            other => panic!("not a class: {other:?}"),
        }
    }

    fn fields(init: &str) -> Vec<String> {
        let r = class(init).unwrap_or_else(|e| panic!("{init}: {e}"));
        assert_eq!(r.kind, RecordKind::Plain);
        r.fields.iter().map(|f| f.name.name.clone()).collect()
    }

    #[test]
    fn fields_are_init_stores_in_first_store_order() {
        let init = "        self.b = k\n        self.a: str = 'x'\n        self.b += 1\n";
        assert_eq!(fields(init), ["b", "a"]);
        let r = class(init).unwrap_or_else(|e| panic!("{e}"));
        assert!(r.fields[0].annotation.is_none());
        assert!(r.fields[1].annotation.is_some());
        assert_eq!(fields("        self.x, self.y = k, k\n"), ["x", "y"]);
        assert!(fields("        pass\n").is_empty());
        let none = parse("class P:\n    def m(self):\n        return 1\n");
        let none = none.unwrap_or_else(|e| panic!("{e}"));
        let StmtKind::Record(r) = &none.body[0].kind else {
            panic!("not a class");
        };
        assert!(r.fields.is_empty());
        assert_eq!(r.methods.len(), 1);
    }

    #[test]
    fn every_path_assigns_every_field() {
        let ok = [
            "        if k:\n            self.x = 1\n        else:\n            self.x = 2\n",
            "        if k:\n            raise ValueError('ü')\n        self.x = 1\n",
            "        try:\n            self.x = 1 // k\n        except ZeroDivisionError:\n            self.x = 0\n",
            "        self.x = 1\n        if k:\n            return\n        self.x = 2\n",
            "        with ThreadPoolExecutor() as e:\n            self.x = 1\n",
        ];
        for init in ok {
            assert_eq!(fields(init), ["x"], "{init}");
        }
        let missing = "field `x` of `P` is not assigned on every path through `__init__`";
        let bad = [
            "        if k:\n            self.x = 1\n",
            // An `__exit__` may suppress the body's exception (D153).
            "        with k as e:\n            self.x = 1\n",
            "        if k:\n            return\n        self.x = 1\n",
            "        while k:\n            self.x = 1\n",
            "        for i in range(k):\n            self.x = i\n",
            "        try:\n            self.x = 1\n        except ValueError:\n            pass\n",
        ];
        for init in bad {
            assert_eq!(class(init).err().as_deref(), Some(missing), "{init}");
        }
    }

    #[test]
    fn self_waits_for_every_field() {
        let early = "`self` is used before every field of `P` is assigned";
        let read = "field `y` of `P` is read before it is assigned";
        let cases = [
            ("        self.m()\n        self.x = 1\n", early),
            ("        print(self)\n        self.x = 1\n", early),
            (
                "        self.x = 1\n        self.x = self.y\n        self.y = 2\n",
                read,
            ),
            ("        self.y += 1\n        self.y = 0\n", read),
        ];
        for (init, msg) in cases {
            assert_eq!(class(init).err().as_deref(), Some(msg), "{init}");
        }
        let after = "        self.x = 1\n        self.y = self.x\n        print(self)\n";
        assert_eq!(fields(after), ["x", "y"]);
        let twice = class("        self.x: int = 1\n        self.x: int = 2\n");
        assert!(twice.err().is_some_and(|e| e.contains("annotated twice")));
    }

    #[test]
    fn a_class_body_holds_methods() {
        let cases = [
            ("class P:\n    x: int\n", "class-body field declarations"),
            (
                "class P:\n    k = 3\n",
                "class attributes are not supported yet",
            ),
            (
                "class P:\n    @staticmethod\n    def f():\n        pass\n",
                "decorators on methods",
            ),
            (
                "class P:\n    def __eq__(self, o) -> bool:\n        return True\n",
                "`__eq__` is not supported yet",
            ),
            (
                "class P:\n    def __repr__(self) -> int:\n        return 1\n",
                "`P.__repr__` returns `str`",
            ),
            (
                "class P:\n    def __init__(self) -> int:\n        return 1\n",
                "`P.__init__` returns `None`",
            ),
            (
                "class P[T]:\n    pass\n",
                "generic classes are not supported yet",
            ),
        ];
        for (src, msg) in cases {
            let got = err(src);
            assert!(got.contains(msg), "{src}: {got}");
        }
        let doc = "class P:\n    \"Doc é.\"\n    def __str__(self) -> str:\n        return 'p'\n";
        assert!(parse(doc).is_ok());
    }
}
