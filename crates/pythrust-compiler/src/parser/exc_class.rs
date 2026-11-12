//! User exception classes (step 4-3, D138-D143): `class NAME '(' base
//! ')' ':'` and a body of only `pass` or a docstring. Whether the base
//! is a builtin exception class is checked by inference, which sees
//! every module's classes (D138).

use super::{error, Parser};
use crate::ast::{ExcClassDef, Ident, Stmt, StmtKind};
use crate::diag::Result;
use crate::token::TokenKind;

/// D139's message for any other member of the body.
const ONLY_PASS: &str = "class attributes are not supported yet; an exception class's body \
                         holds only `pass` or a docstring";

impl Parser<'_> {
    /// Whether the tokens ahead are `class NAME (` with a base: an
    /// exception class rather than a record.
    pub(super) fn at_exc_class(&self) -> bool {
        let named = matches!(self.tok(1).kind, TokenKind::Name(_));
        let based = self.tok(2).kind == TokenKind::LParen && self.tok(3).kind != TokenKind::RParen;
        self.at(&TokenKind::Class) && named && based
    }

    /// `'class' NAME '(' base [','] ')' ':' body`.
    pub(super) fn exc_class(&mut self) -> Result<Stmt> {
        let start = self.bump().span;
        let name = self.binding_name("a class name")?;
        self.expect(&TokenKind::LParen)?;
        let base = self.base()?;
        self.expect(&TokenKind::Colon)?;
        self.exc_body()?;
        let span = self.span_from(start);
        let def = ExcClassDef { name, base, span };
        Ok(Stmt {
            kind: StmtKind::ExcClass(def),
            span,
        })
    }

    /// The one base and the `)`: a name, a module's `a.E`, or one a
    /// `from` import bound, by its qualified name.
    fn base(&mut self) -> Result<Ident> {
        let base = self.exc_name("a base class name")?;
        if base.name == "Generic" {
            return Err(error(base.span, "generic records are not supported yet"));
        }
        if self.at(&TokenKind::Assign) {
            let msg = "keyword arguments in a class's bases are not supported yet";
            return Err(error(base.span, msg));
        }
        if base.name == "BaseException" {
            let msg = "`BaseException` is not supported yet; derive from `Exception`";
            return Err(error(base.span, msg));
        }
        let comma = self.eat(&TokenKind::Comma);
        if comma && self.tok(1).kind == TokenKind::Assign {
            let msg = "keyword arguments in a class's bases are not supported yet";
            return Err(error(self.tok(0).span, msg));
        }
        if comma && !self.at(&TokenKind::RParen) {
            let msg = "multiple base classes are not supported yet";
            return Err(error(self.tok(0).span, msg));
        }
        self.expect(&TokenKind::RParen)?;
        Ok(base)
    }

    /// An exception class's name in a base, `raise`'s clause or a
    /// handler: a name, a module's `a.E`, or one a `from` import bound,
    /// by its qualified name (D140).
    pub(super) fn exc_name(&mut self, what: &str) -> Result<Ident> {
        let first = self.expect_name(what)?;
        let name = self.imported_type(first.name, first.span)?;
        Ok(Ident {
            name,
            span: self.span_from(first.span),
        })
    }

    /// `NEWLINE INDENT [docstring] ('pass' NEWLINE)* DEDENT`.
    fn exc_body(&mut self) -> Result<()> {
        self.ellipsis_stmt(" on its own indented line")?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        if matches!(self.peek(), TokenKind::Str(_)) && self.tok(1).kind == TokenKind::Newline {
            self.bump();
            self.bump();
        }
        while !self.eat(&TokenKind::Dedent) {
            self.exc_member()?;
        }
        Ok(())
    }

    /// `pass`; anything else is D139's named error.
    fn exc_member(&mut self) -> Result<()> {
        let tok = self.tok(0);
        let msg = match &tok.kind {
            TokenKind::Pass => {
                self.bump();
                return self.expect(&TokenKind::Newline).map(|_| ());
            }
            TokenKind::Def | TokenKind::Temper | TokenKind::At => {
                "methods on an exception class are not supported yet"
            }
            TokenKind::Name(_) if self.tok(1).kind == TokenKind::Colon => {
                "fields on an exception class are not supported yet"
            }
            _ => ONLY_PASS,
        };
        Err(error(tok.span, msg))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Module, StmtKind};
    use crate::{lexer, parser};

    fn parse(src: &str) -> Result<Module, String> {
        let toks = lexer::lex(src).map_err(|e| e.message)?;
        parser::parse(&toks).map_err(|e| e.message)
    }

    fn err(src: &str) -> String {
        parse(src).err().unwrap_or_default()
    }

    #[test]
    fn a_class_with_a_base_is_an_exception_class() {
        for src in [
            "class E(ValueError):\n    pass\n",
            "class E(ValueError,):\n    \"doc · é\"\n",
            "class E(Exception):\n    \"doc\"\n    pass\n",
        ] {
            let m = parse(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            let StmtKind::ExcClass(c) = &m.body[0].kind else {
                panic!("not an exception class: {src}");
            };
            assert_eq!(c.name.name, "E");
        }
    }

    #[test]
    fn an_exception_class_body_holds_only_pass_or_a_docstring() {
        let cases = [
            (
                "    code: int\n",
                "fields on an exception class are not supported yet",
            ),
            (
                "    def f(self):\n        pass\n",
                "methods on an exception class",
            ),
            (
                "    temper def f(self):\n        pass\n",
                "methods on an exception class",
            ),
            (
                "    @staticmethod\n    def f():\n        pass\n",
                "methods on an",
            ),
            (
                "    code = 3\n",
                "an exception class's body holds only `pass`",
            ),
        ];
        for (body, msg) in cases {
            let got = err(&format!("class E(Exception):\n{body}"));
            assert!(got.contains(msg), "{body}: {got}");
        }
    }

    #[test]
    fn an_exception_class_has_one_plain_base() {
        let cases = [
            (
                "ValueError, KeyError",
                "multiple base classes are not supported yet",
            ),
            (
                "Exception, metaclass=M",
                "keyword arguments in a class's bases are not",
            ),
            (
                "metaclass=M",
                "keyword arguments in a class's bases are not supported yet",
            ),
            (
                "BaseException",
                "`BaseException` is not supported yet; derive from `Exception`",
            ),
            ("Generic[T]", "generic records are not supported yet"),
        ];
        for (bases, msg) in cases {
            let got = err(&format!("class E({bases}):\n    pass\n"));
            assert!(got.contains(msg), "{bases}: {got}");
        }
    }

    #[test]
    fn a_class_without_a_base_is_still_a_record_error() {
        let msg = "classes without `@dataclass(frozen=True)` are not supported yet";
        assert!(err("class E:\n    pass\n").contains(msg));
        assert!(err("class E():\n    pass\n").contains(msg));
        let nested = err("def f():\n    class E(Exception):\n        pass\n");
        assert!(nested.contains("`def` is only allowed") || nested.contains("top level"));
        let in_if = err("if True:\n    class E(Exception):\n        pass\n");
        assert!(in_if.contains("a class must be defined at the top level of a module"));
    }
}
