//! Parser tests over hand-built token streams.
//!
//! `toks` turns space-separated words into tokens whose spans are the
//! words' byte offsets: `NL`, `INDENT`, `DEDENT` are layout, `'abc'` is a
//! string, words with a `.` (other than `.` itself) are floats. `sx`
//! prints an AST as an s-expression so expected trees stay readable.

use super::parse;
use crate::ast::*;
use crate::diag::{self, DiagKind, Diagnostic};
use crate::infer::{infer, Type, Types};
use crate::lexer::lex;
use crate::span::Span;
use crate::token::{Token, TokenKind};

fn kind_of(word: &str) -> TokenKind {
    use TokenKind as T;
    match word {
        "NL" => T::Newline,
        "INDENT" => T::Indent,
        "DEDENT" => T::Dedent,
        "EOF" => T::EndMarker,
        "def" => T::Def,
        "return" => T::Return,
        "if" => T::If,
        "elif" => T::Elif,
        "else" => T::Else,
        "while" => T::While,
        "for" => T::For,
        "in" => T::In,
        "is" => T::Is,
        "|" => T::Pipe,
        "break" => T::Break,
        "continue" => T::Continue,
        "pass" => T::Pass,
        "and" => T::And,
        "or" => T::Or,
        "not" => T::Not,
        "True" => T::True,
        "False" => T::False,
        "None" => T::None,
        "temper" => T::Temper,
        "try" => T::Try,
        "except" => T::Except,
        "finally" => T::Finally,
        "raise" => T::Raise,
        "del" => T::Del,
        "as" => T::As,
        "with" => T::With,
        "+" => T::Plus,
        "-" => T::Minus,
        "*" => T::Star,
        "/" => T::Slash,
        "//" => T::DoubleSlash,
        "%" => T::Percent,
        "**" => T::DoubleStar,
        "==" => T::EqEq,
        "!=" => T::NotEq,
        "<" => T::Lt,
        "<=" => T::LtEq,
        ">" => T::Gt,
        ">=" => T::GtEq,
        "=" => T::Assign,
        "+=" => T::PlusEq,
        "-=" => T::MinusEq,
        "*=" => T::StarEq,
        "/=" => T::SlashEq,
        "//=" => T::DoubleSlashEq,
        "%=" => T::PercentEq,
        "**=" => T::DoubleStarEq,
        "(" => T::LParen,
        ")" => T::RParen,
        "[" => T::LBracket,
        "]" => T::RBracket,
        "{" => T::LBrace,
        "}" => T::RBrace,
        "," => T::Comma,
        ":" => T::Colon,
        "." => T::Dot,
        "->" => T::Arrow,
        w if w.starts_with('\'') => T::Str(w.trim_matches('\'').to_string()),
        w if w.contains('.') => T::Float(w.parse().unwrap()),
        w if w.starts_with(|c: char| c.is_ascii_digit()) => T::Int(w.parse().unwrap()),
        w => T::Name(w.to_string()),
    }
}

/// Tokens for `src` plus a final `EndMarker`.
fn toks(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut offset = 0;
    for word in src.split(' ') {
        if !word.is_empty() {
            let span = Span {
                start: offset,
                end: offset + word.len(),
            };
            out.push(Token {
                kind: kind_of(word),
                span,
            });
        }
        offset += word.len() + 1;
    }
    let end = Span {
        start: src.len(),
        end: src.len(),
    };
    out.push(Token {
        kind: TokenKind::EndMarker,
        span: end,
    });
    out
}

fn parse_src(src: &str) -> Module {
    parse(&toks(src)).unwrap_or_else(|d| panic!("{src:?}: {d}"))
}

fn parse_err(src: &str) -> Diagnostic {
    match parse(&toks(src)) {
        Ok(m) => panic!("{src:?} parsed: {}", sx_module(&m)),
        Err(d) => d,
    }
}

/// S-expression of the module body.
fn ok(src: &str) -> String {
    sx_module(&parse_src(src))
}

/// The error message, after checking it is a parse error.
fn err(src: &str) -> String {
    let d = parse_err(src);
    assert_eq!(d.kind, DiagKind::Parse);
    d.message
}

/// Source text a span covers.
fn text(src: &str, span: Span) -> &str {
    &src[span.start..span.end]
}

// S-expression printer.

fn sx_module(m: &Module) -> String {
    join(m.body.iter().map(sx_stmt))
}

fn join(parts: impl Iterator<Item = String>) -> String {
    parts.collect::<Vec<_>>().join(" ")
}

fn sx_block(b: &Block) -> String {
    format!("({})", join(b.iter().map(sx_stmt)))
}

fn sx_stmt(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Assign { target, value } => format!("(= {} {})", sx_target(target), sx(value)),
        StmtKind::AnnAssign {
            target,
            annotation,
            value,
        } => format!("(: {} {} {})", target.name, sx_type(annotation), sx(value)),
        StmtKind::AugAssign { target, op, value } => {
            format!("({}= {} {})", bin(*op), sx_target(target), sx(value))
        }
        StmtKind::Return(None) => "(return)".to_string(),
        StmtKind::Return(Some(e)) => format!("(return {})", sx(e)),
        StmtKind::Break => "break".to_string(),
        StmtKind::Continue => "continue".to_string(),
        StmtKind::Pass => "pass".to_string(),
        StmtKind::Expr(e) => sx(e),
        StmtKind::FuncDef(f) => sx_def(f),
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let elifs = elifs.iter().map(|b| format!(" (elif {})", sx_branch(b)));
            format!(
                "(if {}{}{})",
                sx_branch(branch),
                elifs.collect::<String>(),
                sx_else(orelse)
            )
        }
        StmtKind::While { branch, orelse } => {
            format!("(while {}{})", sx_branch(branch), sx_else(orelse))
        }
        StmtKind::For {
            target,
            iter,
            body,
            orelse,
        } => format!(
            "(for {} {} {}{})",
            sx_target(target),
            sx(iter),
            sx_block(body),
            sx_else(orelse)
        ),
        StmtKind::Raise(e) => format!("(raise {})", sx(e)),
        StmtKind::Del { value, index } => format!("(del {} {})", sx(value), sx(index)),
        StmtKind::Try {
            body,
            handlers,
            finally,
        } => {
            let handlers = handlers.iter().map(sx_handler);
            let finally = finally
                .as_ref()
                .map(|f| format!(" (finally {})", sx_block(f)));
            format!(
                "(try {}{}{})",
                sx_block(body),
                handlers.collect::<String>(),
                finally.unwrap_or_default()
            )
        }
        StmtKind::With {
            value,
            binding,
            body,
        } => format!("(with {} {} {})", sx(value), binding.name, sx_block(body)),
        StmtKind::ExcClass(c) => format!("(class {} {})", c.name.name, c.base.name),
        StmtKind::Record(r) => {
            let fields = r.fields.iter().map(|f| match &f.annotation {
                Some(t) => format!(" (: {} {})", f.name.name, sx_type(t)),
                None => format!(" {}", f.name.name),
            });
            let methods = r.methods.iter().map(|m| format!(" {}", m.name));
            format!(
                "(record {}{}{})",
                r.name.name,
                fields.collect::<String>(),
                methods.collect::<String>()
            )
        }
    }
}

fn sx_handler(h: &Handler) -> String {
    let binding = h.binding.as_ref().map(|b| format!(" as {}", b.name));
    let binding = binding.unwrap_or_default();
    let classes: Vec<&str> = h.classes.iter().map(|c| c.name.as_str()).collect();
    format!(
        " (except {}{binding} {})",
        classes.join(" "),
        sx_block(&h.body)
    )
}

fn sx_branch(b: &Branch) -> String {
    format!("{} {}", sx(&b.test), sx_block(&b.body))
}

fn sx_else(b: &Block) -> String {
    if b.is_empty() {
        return String::new();
    }
    format!(" (else {})", sx_block(b))
}

fn sx_def(f: &FuncDef) -> String {
    let word = match f.mode {
        Mode::Loose => "def",
        Mode::Tempered => "temper-def",
    };
    let params = join(f.params.iter().map(sx_param));
    let returns = f
        .returns
        .as_ref()
        .map_or(String::new(), |t| format!(" -> {}", sx_type(t)));
    let raises = match f.raises.as_slice() {
        [] => String::new(),
        names => format!(" raises {}", join(names.iter().map(|n| n.name.clone()))),
    };
    format!(
        "({word} {} ({params}){returns}{raises} {})",
        f.name.name,
        sx_block(&f.body)
    )
}

fn sx_param(p: &Param) -> String {
    let mode = match p.mode {
        ParamMode::Read => "",
        ParamMode::Mut => "mut ",
        ParamMode::Owned => "owned ",
    };
    let ann = p.annotation.as_ref().map(|t| format!(": {}", sx_type(t)));
    let default = p.default.as_ref().map(|e| format!(" = {}", sx(e)));
    match (mode, ann, default) {
        ("", None, None) => p.name.name.clone(),
        (m, a, d) => format!(
            "[{m}{}{}{}]",
            p.name.name,
            a.unwrap_or_default(),
            d.unwrap_or_default()
        ),
    }
}

fn sx_type(t: &TypeExpr) -> String {
    let suffix = if t.optional { " | None" } else { "" };
    if t.args.is_empty() {
        return format!("{}{suffix}", t.name.name);
    }
    let args = t.args.iter().map(sx_type).collect::<Vec<_>>().join(", ");
    format!("{}[{args}]{suffix}", t.name.name)
}

fn sx_target(t: &Target) -> String {
    match &t.kind {
        TargetKind::Name(n) => n.clone(),
        TargetKind::Subscript { value, index } => format!("(sub {} {})", sx(value), sx(index)),
        TargetKind::Attr { value, attr } => format!("(. {} {})", sx(value), attr.name),
        TargetKind::Tuple(items) => format!("(tuple {})", join(items.iter().map(sx_target))),
    }
}

fn sx(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(n) => n.clone(),
        ExprKind::Int(v) => v.to_string(),
        ExprKind::Float(v) => format!("{v:?}"),
        ExprKind::Str(s) => format!("'{s}'"),
        ExprKind::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        ExprKind::None => "None".to_string(),
        ExprKind::Tuple(items) => format!("(tuple{})", spaced(items)),
        ExprKind::List(items) => format!("(list{})", spaced(items)),
        ExprKind::Dict(items) => {
            let pairs = items.iter().map(|(k, v)| format!(" ({} {})", sx(k), sx(v)));
            format!("(dict{})", pairs.collect::<String>())
        }
        ExprKind::Set(items) => format!("(set{})", spaced(items)),
        ExprKind::Unary { op, operand } => {
            let op = match op {
                UnaryOp::Neg => "neg",
                UnaryOp::Pos => "pos",
                UnaryOp::Not => "not",
            };
            format!("({op} {})", sx(operand))
        }
        ExprKind::Binary { op, left, right } => {
            format!("({} {} {})", bin(*op), sx(left), sx(right))
        }
        ExprKind::Logical { op, left, right } => {
            let op = match op {
                LogicalOp::And => "and",
                LogicalOp::Or => "or",
            };
            format!("({op} {} {})", sx(left), sx(right))
        }
        ExprKind::Compare { left, rest } => {
            let rest = rest
                .iter()
                .map(|(op, e)| format!(" {} {}", cmp(*op), sx(e)));
            format!("(cmp {}{})", sx(left), rest.collect::<String>())
        }
        ExprKind::IfExp { test, body, orelse } => {
            format!("(ifexp {} {} {})", sx(test), sx(body), sx(orelse))
        }
        ExprKind::Call { func, args } => {
            let args = args.iter().map(|a| match &a.name {
                Some(n) => format!(" (kw {} {})", n.name, sx(&a.value)),
                None => format!(" {}", sx(&a.value)),
            });
            format!("(call {}{})", sx(func), args.collect::<String>())
        }
        ExprKind::Subscript { value, index } => format!("(sub {} {})", sx(value), sx(index)),
        ExprKind::Slice {
            value,
            start,
            stop,
            step,
        } => format!(
            "(slice {} {} {} {})",
            sx(value),
            start.as_deref().map(sx).unwrap_or_default(),
            stop.as_deref().map(sx).unwrap_or_default(),
            step.as_deref().map(sx).unwrap_or_default()
        ),
        ExprKind::Attr { value, attr } => format!("(. {} {})", sx(value), attr.name),
        ExprKind::Comp { gather, clauses } => {
            let clauses = clauses.iter().map(|c| {
                let ifs = c.ifs.iter().map(|x| format!(" (if {})", sx(x)));
                let ifs: String = ifs.collect();
                format!(" (for {} {}{ifs})", sx_target(&c.target), sx(&c.iter))
            });
            let head = match gather {
                Gather::List(e) => format!("listcomp {}", sx(e)),
                Gather::Set(e) => format!("setcomp {}", sx(e)),
                Gather::Dict(k, v) => format!("dictcomp {} {}", sx(k), sx(v)),
                Gather::Gen(e) => format!("genexp {}", sx(e)),
            };
            format!("({head}{})", clauses.collect::<String>())
        }
    }
}

fn spaced(items: &[Expr]) -> String {
    items.iter().map(|e| format!(" {}", sx(e))).collect()
}

fn bin(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::FloorDiv => "//",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
    }
}

fn cmp(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::NotEq => "!=",
        CmpOp::Lt => "<",
        CmpOp::LtEq => "<=",
        CmpOp::Gt => ">",
        CmpOp::GtEq => ">=",
        CmpOp::In => "in",
        CmpOp::NotIn => "not-in",
        CmpOp::Is => "is",
        CmpOp::IsNot => "is-not",
    }
}

// Token stream boundary.

#[test]
fn empty_module() {
    let m = parse_src("");
    assert!(m.body.is_empty());
    assert_eq!(m.span, Span { start: 0, end: 0 });
}

#[test]
fn rejects_stream_without_single_final_end_marker() {
    assert!(parse(&[]).is_err());
    let mut no_end = toks("x NL");
    no_end.pop();
    assert!(parse(&no_end).is_err());
    let early_end = toks("EOF x NL");
    assert!(parse(&early_end).is_err());
}

#[test]
fn statement_needs_newline() {
    assert_eq!(err("x"), "expected newline, found end of input");
    assert_eq!(err("x = 1 2 NL"), "expected newline, found integer `2`");
}

// Simple statements.

#[test]
fn expression_statements() {
    assert_eq!(ok("f ( x ) NL"), "(call f x)");
    assert_eq!(ok("pass NL"), "pass");
    assert_eq!(ok("1 , 2 NL"), "(tuple 1 2)");
}

#[test]
fn assignment_and_unpacking() {
    assert_eq!(ok("x = 1 NL"), "(= x 1)");
    assert_eq!(ok("a , b = b , a NL"), "(= (tuple a b) (tuple b a))");
    assert_eq!(ok("a , = t NL"), "(= (tuple a) t)");
    assert_eq!(
        ok("( a , ( b , c ) ) = t NL"),
        "(= (tuple a (tuple b c)) t)"
    );
    assert_eq!(ok("( a ) = 1 NL"), "(= a 1)");
    assert_eq!(ok("x [ i ] = 2 NL"), "(= (sub x i) 2)");
    assert_eq!(ok("p . x = 2 NL"), "(= (. p x) 2)");
    assert_eq!(ok("p . q . x += 2 NL"), "(+= (. (. p q) x) 2)");
    assert_eq!(ok("f ( ) [ 0 ] = 2 NL"), "(= (sub (call f) 0) 2)");
}

#[test]
fn invalid_assignment_targets() {
    assert_eq!(err("f ( ) = 1 NL"), "cannot assign to a function call");
    assert_eq!(err("1 = x NL"), "cannot assign to a literal");
    assert_eq!(err("[ a ] = x NL"), "cannot assign to a list");
    assert_eq!(err("( ) = x NL"), "cannot assign to an empty tuple");
    assert_eq!(
        err("a + b = x NL"),
        "cannot assign to an operator expression"
    );
    assert_eq!(err("a , 1 = x NL"), "cannot assign to a literal");
    assert_eq!(err("a = b = 1 NL"), "chained assignment is not supported");
}

#[test]
fn list_comprehensions_own_the_names_they_bind() {
    // The `[` is at offset 0, so each name a clause binds becomes `name@0`.
    assert_eq!(ok("[ x for x in xs ] NL"), "(listcomp x@0 (for x@0 xs))");
    assert_eq!(
        ok("[ x + y for x in xs if x for y in x if y > 1 if z ] NL"),
        "(listcomp (+ x@0 y@0) (for x@0 xs (if x@0)) (for y@0 x@0 (if (cmp y@0 > 1)) (if z)))"
    );
    assert_eq!(
        ok("[ a for a , ( b , c ) in ps ] NL"),
        "(listcomp a@0 (for (tuple a@0 (tuple b@0 c@0)) ps))"
    );
    assert_eq!(
        ok("[ a if c else b for a in xs ] NL"),
        "(listcomp (ifexp c a@0 b) (for a@0 xs))"
    );
}

#[test]
fn a_comprehension_s_first_iterable_is_the_enclosing_scope_s() {
    assert_eq!(ok("[ x for x in x ] NL"), "(listcomp x@0 (for x@0 x))");
    assert_eq!(
        ok("y = [ x for x in x ] NL"),
        "(= y (listcomp x@4 (for x@4 x)))"
    );
    // The inner `[` is at offset 2: its names are its own, the outer's
    // stay the outer's, its first iterable included.
    assert_eq!(
        ok("[ [ x for y in x ] for x in xs ] NL"),
        "(listcomp (listcomp x@0 (for y@2 x@0)) (for x@0 xs))"
    );
    assert_eq!(
        ok("[ [ x for x in x ] for x in xs ] NL"),
        "(listcomp (listcomp x@2 (for x@2 x@0)) (for x@0 xs))"
    );
}

#[test]
fn comprehension_forms_not_supported_yet_are_named() {
    let target =
        "a comprehension target other than a name or a tuple of names is not supported yet";
    assert_eq!(err("[ 1 for x [ 0 ] in xs ] NL"), target);
    assert_eq!(err("[ 1 for a , x [ 0 ] in xs ] NL"), target);
    assert_eq!(
        err("[ 1 for f ( ) in xs ] NL"),
        "cannot assign to a function call"
    );
}

/// A generator expression (D119) is parsed in parentheses or as a call's
/// only argument, with a list comprehension's own scope; next to another
/// argument it must be parenthesized, as in CPython.
#[test]
fn generator_expressions() {
    assert_eq!(
        ok("s = sum ( x for x in xs if x ) NL"),
        "(= s (call sum (genexp x@10 (for x@10 xs (if x@10)))))"
    );
    assert_eq!(
        ok("g = ( x for x in xs ) NL"),
        "(= g (genexp x@4 (for x@4 xs)))"
    );
    assert_eq!(
        ok("s = sum ( ( x for x in xs ) , 1 ) NL"),
        "(= s (call sum (genexp x@10 (for x@10 xs)) 1))"
    );
    let parens = "generator expression must be parenthesized";
    assert_eq!(err("f ( 1 , x for x in xs ) NL"), parens);
    assert_eq!(err("f ( x for x in xs , 1 ) NL"), parens);
    assert_eq!(err("f ( x for x in xs , ) NL"), parens);
    assert_eq!(err("f ( a = x for x in xs ) NL"), parens);
    assert_eq!(err("( a , b for x in xs ) NL"), parens);
}

/// `{e for ...}` is a set comprehension and `{k: v for ...}` a dict one
/// (D116), with a list comprehension's own scope; `{}` stays a dict and
/// a `for` after a second item is a syntax error, as in CPython.
#[test]
fn set_and_dict_comprehensions() {
    assert_eq!(
        ok("s = { x for x in xs if x } NL"),
        "(= s (setcomp x@4 (for x@4 xs (if x@4))))"
    );
    assert_eq!(
        ok("d = { k : v for k , v in xs for w in k } NL"),
        "(= d (dictcomp k@4 v@4 (for (tuple k@4 v@4) xs) (for w@4 k@4)))"
    );
    assert_eq!(ok("d = { } NL"), "(= d (dict))");
    assert_eq!(ok("s = { x , y } NL"), "(= s (set x y))");
    assert_eq!(
        err("s = { x , y for y in z } NL"),
        "expected `}`, found `for`"
    );
    assert_eq!(
        err("d = { 1 : 2 , k : v for k in z } NL"),
        "expected `}`, found `for`"
    );
    assert_eq!(
        err("d = { k : v for k [ 0 ] in z } NL"),
        "a comprehension target other than a name or a tuple of names is not supported yet"
    );
}

#[test]
fn comprehension_iterables_and_filters_are_or_exprs() {
    assert_eq!(
        err("[ x for x in a if b else c ] NL"),
        "expected `]`, found `else`"
    );
    assert_eq!(err("[ x for x in a , b ] NL"), "expected `]`, found `,`");
    assert_eq!(
        ok("[ x for x in ( a if b else c ) if not x or y ] NL"),
        "(listcomp x@0 (for x@0 (ifexp b a c) (if (or (not x@0) y))))"
    );
    assert_eq!(
        err("[ x for x in xs ] = 1 NL"),
        "cannot assign to a list comprehension"
    );
}

#[test]
fn invalid_target_span_points_at_the_target() {
    let src = "a , f ( ) = x NL";
    assert_eq!(text(src, parse_err(src).span), "f ( )");
}

#[test]
fn annotated_assignment() {
    assert_eq!(ok("x : int = 1 NL"), "(: x int 1)");
    assert_eq!(
        ok("p : tuple [ int , list [ str ] ] = t NL"),
        "(: p tuple[int, list[str]] t)"
    );
    assert_eq!(ok("n : None = None NL"), "(: n None None)");
}

#[test]
fn annotated_assignment_errors() {
    assert_eq!(err("x : int NL"), "expected `=`, found newline");
    assert_eq!(
        err("a , b : int = 1 NL"),
        "only a single name can be annotated"
    );
    assert_eq!(
        err("x [ 0 ] : int = 1 NL"),
        "only a single name can be annotated"
    );
    assert_eq!(err("x : 1 = 1 NL"), "expected a type, found integer `1`");
    assert_eq!(err("x : list [ ] = 1 NL"), "expected a type, found `]`");
    assert_eq!(err("x : int = 1 , 2 NL"), "expected newline, found `,`");
}

#[test]
fn augmented_assignment() {
    assert_eq!(ok("x += 1 NL"), "(+= x 1)");
    assert_eq!(ok("x [ 0 ] **= 2 NL"), "(**= (sub x 0) 2)");
    assert_eq!(ok("x //= 2 NL"), "(//= x 2)");
    assert_eq!(
        err("a , b += 1 NL"),
        "augmented assignment needs a single target, not a tuple"
    );
    assert_eq!(err("f ( ) -= 1 NL"), "cannot assign to a function call");
}

// Expressions.

#[test]
fn arithmetic_precedence_and_associativity() {
    assert_eq!(ok("1 + 2 * 3 NL"), "(+ 1 (* 2 3))");
    assert_eq!(ok("1 - 2 - 3 NL"), "(- (- 1 2) 3)");
    assert_eq!(ok("a / b // c % d NL"), "(% (// (/ a b) c) d)");
    assert_eq!(ok("( 1 + 2 ) * 3 NL"), "(* (+ 1 2) 3)");
}

#[test]
fn power_binds_right_and_above_unary_on_its_left() {
    assert_eq!(ok("2 ** 3 ** 2 NL"), "(** 2 (** 3 2))");
    assert_eq!(ok("- x ** 2 NL"), "(neg (** x 2))");
    assert_eq!(ok("- 2 ** 2 NL"), "(neg (** 2 2))");
    assert_eq!(ok("2 ** - 1 NL"), "(** 2 -1)");
    assert_eq!(ok("- - x NL"), "(neg (neg x))");
    assert_eq!(ok("+ x NL"), "(pos x)");
}

#[test]
fn boolean_operators() {
    assert_eq!(ok("a or b and not c NL"), "(or a (and b (not c)))");
    assert_eq!(ok("a or b or c NL"), "(or (or a b) c)");
    assert_eq!(ok("not not a NL"), "(not (not a))");
    assert_eq!(ok("not a == b NL"), "(not (cmp a == b))");
}

#[test]
fn chained_comparisons() {
    assert_eq!(ok("a < b NL"), "(cmp a < b)");
    assert_eq!(
        ok("a < b <= c != d not in e in f NL"),
        "(cmp a < b <= c != d not-in e in f)"
    );
    assert_eq!(ok("a + 1 == b * 2 NL"), "(cmp (+ a 1) == (* b 2))");
    assert_eq!(err("a not b NL"), "expected newline, found `not`");
}

#[test]
fn conditional_expression() {
    assert_eq!(ok("a if b else c NL"), "(ifexp b a c)");
    assert_eq!(
        ok("a if b else c if d else e NL"),
        "(ifexp b a (ifexp d c e))"
    );
    assert_eq!(
        ok("a or b if c and d else e NL"),
        "(ifexp (and c d) (or a b) e)"
    );
    assert_eq!(err("a if b NL"), "expected `else`, found newline");
}

#[test]
fn postfix_chains() {
    assert_eq!(ok("f ( x ) [ 0 ] . y NL"), "(. (sub (call f x) 0) y)");
    assert_eq!(ok("xs . append ( 1 ) NL"), "(call (. xs append) 1)");
    assert_eq!(ok("f ( ) ( ) NL"), "(call (call f))");
    assert_eq!(
        err("a . 1 NL"),
        "expected an attribute name, found integer `1`"
    );
    assert_eq!(err("a [ 1 , 2 ] NL"), "expected `]`, found `,`");
    assert_eq!(err("a [ ] NL"), "expected an expression, found `]`");
}

#[test]
fn call_arguments() {
    assert_eq!(ok("f ( 1 , k = 2 , ) NL"), "(call f 1 (kw k 2))");
    assert_eq!(ok("f ( ( 1 , 2 ) ) NL"), "(call f (tuple 1 2))");
    assert_eq!(ok("f ( a == b ) NL"), "(call f (cmp a == b))");
    assert_eq!(
        err("f ( k = 1 , 2 ) NL"),
        "positional argument follows keyword argument"
    );
    assert_eq!(err("f ( = 1 ) NL"), "expected an expression, found `=`");
    assert_eq!(err("f ( 1 NL"), "expected `)`, found newline");
    assert_eq!(err("f ( , ) NL"), "expected an expression, found `,`");
}

#[test]
fn repeated_keyword_argument() {
    let msg = "keyword argument `x` repeated";
    assert_eq!(err("f ( x = 1 , x = 2 ) NL"), msg);
    assert_eq!(err("f ( 0 , x = 1 , y = 2 , x = 3 ) NL"), msg);
    assert_eq!(
        err("if False : NL INDENT f ( x = 1 , x = 2 ) NL DEDENT"),
        msg
    );
    assert_eq!(ok("f ( x = 1 , y = 2 ) NL"), "(call f (kw x 1) (kw y 2))");
    let src = "f ( x = 1 , x = 2 ) NL";
    let span = parse_err(src).span;
    assert_eq!((text(src, span), span.start), ("x", 12));
}

#[test]
fn tuples_and_parentheses() {
    assert_eq!(ok("( ) NL"), "(tuple)");
    assert_eq!(ok("( 1 , ) NL"), "(tuple 1)");
    assert_eq!(ok("( 1 , 2 , ) NL"), "(tuple 1 2)");
    assert_eq!(ok("( 1 ) NL"), "1");
    assert_eq!(ok("x = 1 , NL"), "(= x (tuple 1))");
    assert_eq!(err("( 1 , 2 NL"), "expected `)`, found newline");
}

#[test]
fn lists() {
    assert_eq!(ok("[ ] NL"), "(list)");
    assert_eq!(ok("[ 1 ] NL"), "(list 1)");
    assert_eq!(ok("[ 1 , 2 , ] NL"), "(list 1 2)");
    assert_eq!(ok("[ ( 1 , 2 ) ] NL"), "(list (tuple 1 2))");
    assert_eq!(err("[ 1 2 ] NL"), "expected `]`, found integer `2`");
}

#[test]
fn dict_and_set_displays() {
    assert_eq!(ok("{ } NL"), "(dict)");
    assert_eq!(ok("{ 'a' : 1 } NL"), "(dict ('a' 1))");
    assert_eq!(
        ok("{ 1 : x , k : f ( y ) , } NL"),
        "(dict (1 x) (k (call f y)))"
    );
    assert_eq!(ok("{ 1 } NL"), "(set 1)");
    assert_eq!(ok("{ 1 , ( 2 , 3 ) , } NL"), "(set 1 (tuple 2 3))");
    assert_eq!(
        ok("{ { 1 : 2 } : { 3 } } NL"),
        "(dict ((dict (1 2)) (set 3)))"
    );
    assert_eq!(err("{ 1 : 2 , 3 } NL"), "expected `:`, found `}`");
    assert_eq!(err("{ 1 , 2 : 3 } NL"), "expected `}`, found `:`");
    assert_eq!(err("{ 1 : } NL"), "expected an expression, found `}`");
    assert_eq!(err("{ , } NL"), "expected an expression, found `,`");
    assert_eq!(err("{ 1 2 } NL"), "expected `}`, found integer `2`");
    assert_eq!(err("{ } = 1 NL"), "cannot assign to a dict display");
    assert_eq!(err("{ 1 } = 1 NL"), "cannot assign to a set display");
}

#[test]
fn literals() {
    assert_eq!(
        ok("f ( 1 , 2.5 , 'hi' , True , False , None ) NL"),
        "(call f 1 2.5 'hi' True False None)"
    );
    assert_eq!(err("x = NL"), "expected an expression, found newline");
    assert_eq!(err("x = def NL"), "expected an expression, found `def`");
}

#[test]
fn int_literal_range() {
    assert_eq!(
        ok("x = 9223372036854775807 NL"),
        "(= x 9223372036854775807)"
    );
    assert_eq!(
        ok("x = - 9223372036854775808 NL"),
        "(= x -9223372036854775808)"
    );
    assert_eq!(ok("x = - 0 NL"), "(= x 0)");
    assert_eq!(ok("x = 1 - 2 NL"), "(= x (- 1 2))");
    assert_eq!(ok("x = - - 1 NL"), "(= x (neg -1))");
}

#[test]
fn int_literal_out_of_range() {
    let too_large = "integer literal is too large for int (i64)";
    assert_eq!(err("x = 9223372036854775808 NL"), too_large);
    assert_eq!(err("x = + 9223372036854775808 NL"), too_large);
    assert_eq!(err("x = - 9223372036854775808 ** 1 NL"), too_large);
    assert_eq!(err("x = 18446744073709551615 NL"), too_large);
    assert_eq!(
        err("x = - 9223372036854775809 NL"),
        "integer literal is too small for int (i64)"
    );
}

#[test]
fn folded_negative_literal_span_covers_the_minus() {
    let src = "x = - 5 NL";
    let StmtKind::Assign { value, .. } = &parse_src(src).body[0].kind else {
        panic!("not an assignment");
    };
    assert_eq!(value.kind, ExprKind::Int(-5));
    assert_eq!(text(src, value.span), "- 5");
}

// Compound statements.

#[test]
fn function_definitions() {
    assert_eq!(
        ok("def f ( ) : NL INDENT pass NL DEDENT"),
        "(def f () (pass))"
    );
    assert_eq!(
        ok("temper def f ( a : int , b = 1 , ) -> list [ int ] : NL INDENT return a , b NL DEDENT"),
        "(temper-def f ([a: int] [b = 1]) -> list[int] ((return (tuple a b))))"
    );
    assert_eq!(
        ok("def f ( a : int = 0 ) -> None : NL INDENT return NL DEDENT"),
        "(def f ([a: int = 0]) -> None ((return)))"
    );
}

#[test]
fn raises_clauses() {
    assert_eq!(
        ok("temper def f ( ) -> int raises ValueError : NL INDENT pass NL DEDENT"),
        "(temper-def f () -> int raises ValueError (pass))"
    );
    assert_eq!(
        ok("def f ( ) raises ValueError , Exception : NL INDENT pass NL DEDENT"),
        "(def f () raises ValueError Exception (pass))"
    );
    // Contextual: still an ordinary name everywhere else.
    assert_eq!(
        ok("def raises ( raises ) : NL INDENT return raises NL DEDENT"),
        "(def raises (raises) ((return raises)))"
    );
    assert_eq!(
        err("def f ( ) raises : NL"),
        "expected an exception name after `raises`, found `:`"
    );
    assert_eq!(
        err("def f ( ) raises ValueError , : NL"),
        "expected an exception name after `,`, found `:`"
    );
    assert_eq!(
        err("def f ( ) -> int raises ValueError IndexError : NL"),
        "expected `:`, found name `IndexError`"
    );
}

#[test]
fn parameter_modes() {
    assert_eq!(
        ok("def f ( mut xs , owned ys : list [ int ] , zs ) : NL INDENT pass NL DEDENT"),
        "(def f ([mut xs] [owned ys: list[int]] zs) (pass))"
    );
    assert_eq!(
        ok("temper def f ( mut xs : list [ int ] = [ ] ) -> None : NL INDENT pass NL DEDENT"),
        "(temper-def f ([mut xs: list[int] = (list)]) -> None (pass))"
    );
    // Contextual: a mode only before another name.
    assert_eq!(
        ok("def f ( mut , owned : int , read = 1 ) : NL INDENT mut = owned NL DEDENT"),
        "(def f (mut [owned: int] [read = 1]) ((= mut owned)))"
    );
    assert_eq!(
        err("def f ( read xs ) : NL"),
        "parameters are read unless marked; drop `read`"
    );
    assert_eq!(
        err("def f ( mut owned xs ) : NL"),
        "expected `)`, found name `xs`"
    );
    let src = "def f ( mut xs ) : NL INDENT pass NL DEDENT";
    let StmtKind::FuncDef(f) = &parse_src(src).body[0].kind else {
        panic!("not a def");
    };
    assert_eq!(text(src, f.params[0].span), "mut xs");
}

#[test]
fn function_definition_errors() {
    assert_eq!(err("temper f ( ) NL"), "expected `def`, found name `f`");
    assert_eq!(err("def ( ) : NL"), "expected a function name, found `(`");
    assert_eq!(
        err("def f ( 1 ) : NL"),
        "expected a parameter name, found integer `1`"
    );
    assert_eq!(err("def f ( ) NL"), "expected `:`, found newline");
    assert_eq!(err("def f ( ) : pass NL"), "expected newline, found `pass`");
    assert_eq!(
        err("def f ( ) : NL pass NL"),
        "expected indent, found `pass`"
    );
    assert_eq!(
        err("def f ( a , a ) : NL INDENT pass NL DEDENT"),
        "duplicate parameter `a`"
    );
    assert_eq!(
        err("def f ( a = 1 , b ) : NL INDENT pass NL DEDENT"),
        "parameter `b` without a default follows one with a default"
    );
}

#[test]
fn def_only_at_top_level() {
    let msg = "`def` is only allowed at module top level";
    assert_eq!(
        err("def f ( ) : NL INDENT def g ( ) : NL INDENT pass NL DEDENT DEDENT"),
        msg
    );
    assert_eq!(
        err("if x : NL INDENT temper def g ( ) : NL INDENT pass NL DEDENT DEDENT"),
        msg
    );
}

#[test]
fn return_only_inside_function() {
    assert_eq!(err("return NL"), "`return` outside function");
    assert_eq!(
        err("if x : NL INDENT return 1 NL DEDENT"),
        "`return` outside function"
    );
    let after = "def f ( ) : NL INDENT pass NL DEDENT return NL";
    assert_eq!(err(after), "`return` outside function");
}

#[test]
fn break_and_continue_only_inside_loop() {
    assert_eq!(err("break NL"), "`break` outside loop");
    assert_eq!(
        err("if x : NL INDENT continue NL DEDENT"),
        "`continue` outside loop"
    );
    let in_else = "while x : NL INDENT pass NL DEDENT else : NL INDENT break NL DEDENT";
    assert_eq!(err(in_else), "`break` outside loop");
    let after = "for x in y : NL INDENT pass NL DEDENT break NL";
    assert_eq!(err(after), "`break` outside loop");
    let in_func = "def f ( ) : NL INDENT break NL DEDENT";
    assert_eq!(err(in_func), "`break` outside loop");
    assert_eq!(
        ok("while x : NL INDENT if y : NL INDENT break NL DEDENT continue NL DEDENT"),
        "(while x ((if y (break)) continue))"
    );
}

#[test]
fn raise_statement() {
    assert_eq!(
        ok("raise ValueError ( 'bad' ) NL"),
        "(raise (call ValueError 'bad'))"
    );
    assert_eq!(ok("raise e NL"), "(raise e)");
    assert!(err("raise NL").starts_with("bare `raise` is not supported yet"));
    assert_eq!(err("raise a , b NL"), "expected newline, found `,`");
}

#[test]
fn del_statement() {
    assert_eq!(ok("del xs [ i ] NL"), "(del xs i)");
    assert_eq!(ok("del d [ k ] [ 0 ] NL"), "(del (sub d k) 0)");
    assert_eq!(err("del x NL"), "`del` of a name is not supported yet");
    assert_eq!(
        err("del a [ 0 ] , b [ 0 ] NL"),
        "`del` of several targets is not supported yet"
    );
    assert_eq!(
        err("del a . b NL"),
        "`del` of an attribute is not supported yet"
    );
    assert!(err("del f ( ) NL").starts_with("cannot delete this"));
    assert!(err("del NL").starts_with("expected"));
}

#[test]
fn try_except() {
    assert_eq!(
        ok("try : NL INDENT x NL DEDENT except ValueError : NL INDENT y NL DEDENT"),
        "(try (x) (except ValueError (y)))"
    );
    let two = "try : NL INDENT x NL DEDENT except ValueError as e : NL INDENT y NL DEDENT \
               except IndexError : NL INDENT z NL DEDENT w NL";
    assert_eq!(
        ok(two),
        "(try (x) (except ValueError as e (y)) (except IndexError (z))) w"
    );
}

#[test]
fn try_finally() {
    let body = "try : NL INDENT x NL DEDENT";
    let fin = "finally : NL INDENT z NL DEDENT";
    assert_eq!(
        ok(&format!("{body} {fin} w NL")),
        "(try (x) (finally (z))) w"
    );
    assert_eq!(
        ok(&format!(
            "{body} except ValueError : NL INDENT y NL DEDENT {fin}"
        )),
        "(try (x) (except ValueError (y)) (finally (z)))"
    );
    let src = format!("{body} {fin}");
    let stmt = &parse_src(&src).body[0];
    assert_eq!(
        text(&src, stmt.span),
        &src[..src.len() - " NL DEDENT".len()]
    );
    // `finally` ends the statement: no handler may follow it.
    assert_eq!(
        err(&format!(
            "{body} {fin} except ValueError : NL INDENT y NL DEDENT"
        )),
        "expected an expression, found `except`"
    );
    assert_eq!(
        err(&format!("{body} finally NL")),
        "expected `:`, found newline"
    );
    // No jump may leave a `finally` block (D89); one inside a loop the
    // block holds stays legal.
    let def = "def f ( ) : NL INDENT";
    assert_eq!(
        err(&format!(
            "{def} {body} finally : NL INDENT return NL DEDENT DEDENT"
        )),
        "`return` inside `finally` is not supported"
    );
    let looped = "while x : NL INDENT";
    for word in ["break", "continue"] {
        assert_eq!(
            err(&format!(
                "{looped} {body} finally : NL INDENT {word} NL DEDENT DEDENT"
            )),
            format!("`{word}` inside `finally` is not supported")
        );
        let inner =
            format!("{body} finally : NL INDENT while y : NL INDENT {word} NL DEDENT DEDENT");
        assert_eq!(
            ok(&inner),
            format!("(try (x) (finally ((while y ({word})))))")
        );
    }
}

#[test]
fn try_spans_cover_every_handler() {
    let src = "try : NL INDENT x NL DEDENT except ValueError as e : NL INDENT y NL DEDENT";
    let stmt = &parse_src(src).body[0];
    assert_eq!(text(src, stmt.span), &src[..src.len() - " NL DEDENT".len()]);
    let StmtKind::Try { handlers, .. } = &stmt.kind else {
        panic!("not a try");
    };
    let binding = handlers[0].binding.as_ref().map(|b| text(src, b.span));
    assert_eq!(binding, Some("e"));
    assert!(text(src, handlers[0].span).starts_with("except ValueError"));
}

#[test]
fn tuple_handlers() {
    let body = "try : NL INDENT x NL DEDENT";
    let cases = [
        ("( ValueError , IndexError )", "ValueError IndexError"),
        ("( ValueError , IndexError , )", "ValueError IndexError"),
        ("( ValueError , )", "ValueError"),
        ("( ValueError )", "ValueError"),
    ];
    for (classes, sx) in cases {
        assert_eq!(
            ok(&format!(
                "{body} except {classes} as e : NL INDENT y NL DEDENT"
            )),
            format!("(try (x) (except {sx} as e (y)))")
        );
    }
}

#[test]
fn try_except_errors() {
    let body = "try : NL INDENT x NL DEDENT";
    assert_eq!(
        err(&format!("{body} y NL")),
        "expected `except` or `finally`, found name `y`"
    );
    assert!(err(&format!("{body} except : NL INDENT y NL DEDENT"))
        .starts_with("bare `except:` is not supported yet"));
    assert_eq!(
        err(&format!(
            "{body} except ValueError , IndexError : NL INDENT y NL DEDENT"
        )),
        "multiple exception types must be parenthesized, \
         as in `except (ValueError, IndexError):`"
    );
    assert_eq!(
        err(&format!("{body} except ( ) : NL INDENT y NL DEDENT")),
        "`except ()` catches nothing and is not supported; name at least one exception class"
    );
    for nested in ["( ( A , B ) , C )", "( A , ( B , ) )"] {
        assert!(
            err(&format!("{body} except {nested} : NL INDENT y NL DEDENT"))
                .starts_with("nested tuples of exception classes are not supported"),
            "{nested}"
        );
    }
    assert_eq!(
        err(&format!(
            "{body} except ( ValueError , , ) : NL INDENT y NL DEDENT"
        )),
        "expected an exception name, such as `ValueError`, found `,`"
    );
    assert_eq!(
        err(&format!(
            "{body} except ( ValueError IndexError ) : NL INDENT y NL DEDENT"
        )),
        "expected `)`, found name `IndexError`"
    );
    assert_eq!(
        err(&format!(
            "{body} except ValueError as : NL INDENT y NL DEDENT"
        )),
        "expected a name to bind the exception to, found `:`"
    );
    let with_else =
        format!("{body} except ValueError : NL INDENT y NL DEDENT else : NL INDENT z NL DEDENT");
    assert_eq!(err(&with_else), "`else` on `try` is not supported yet");
    let reserved = "'as' is allowed only in `except Name as e` and `with value as name`";
    assert_eq!(err("as = 1 NL"), reserved);
    assert_eq!(err("x = y as z NL"), reserved);
    assert_eq!(
        err("except ValueError : NL INDENT y NL DEDENT"),
        "expected an expression, found `except`"
    );
}

#[test]
fn if_elif_else() {
    assert_eq!(ok("if a : NL INDENT x NL DEDENT"), "(if a (x))");
    assert_eq!(
        ok("if a : NL INDENT x NL DEDENT elif b : NL INDENT y NL DEDENT elif c : NL INDENT z NL DEDENT else : NL INDENT w NL DEDENT"),
        "(if a (x) (elif b (y)) (elif c (z)) (else (w)))"
    );
    assert_eq!(err("else : NL"), "expected an expression, found `else`");
    assert_eq!(
        err("if a : NL INDENT DEDENT"),
        "expected an expression, found dedent"
    );
}

#[test]
fn while_loops() {
    assert_eq!(
        ok("while a : NL INDENT x NL y NL DEDENT else : NL INDENT z NL DEDENT"),
        "(while a (x y) (else (z)))"
    );
}

#[test]
fn for_loops() {
    assert_eq!(
        ok("for x in xs : NL INDENT f ( x ) NL DEDENT"),
        "(for x xs ((call f x)))"
    );
    assert_eq!(
        ok("for i , ( a , b ) in pairs ( ) : NL INDENT pass NL DEDENT else : NL INDENT pass NL DEDENT"),
        "(for (tuple i (tuple a b)) (call pairs) (pass) (else (pass)))"
    );
    assert_eq!(
        ok("for x , in y : NL INDENT pass NL DEDENT"),
        "(for (tuple x) y (pass))"
    );
    assert_eq!(
        ok("for x [ 0 ] in y : NL INDENT pass NL DEDENT"),
        "(for (sub x 0) y (pass))"
    );
}

#[test]
fn for_loop_errors() {
    assert_eq!(
        err("for f ( ) in y : NL"),
        "cannot assign to a function call"
    );
    assert_eq!(err("for x y : NL"), "expected `in`, found name `y`");
    assert_eq!(err("for in y : NL"), "expected an expression, found `in`");
    assert_eq!(err("for x in a , b : NL"), "expected `:`, found `,`");
}

#[test]
fn nested_blocks_and_dedent_to_end() {
    let src = "def f ( n ) : NL INDENT while n : NL INDENT if n : NL INDENT n -= 1 NL DEDENT DEDENT return n NL DEDENT f ( 3 ) NL";
    assert_eq!(
        ok(src),
        "(def f (n) ((while n ((if n ((-= n 1))))) (return n))) (call f 3)"
    );
}

// Spans.

#[test]
fn statement_spans_exclude_layout() {
    let src = "x = a + b NL if c : NL INDENT pass NL DEDENT";
    let m = parse_src(src);
    assert_eq!(text(src, m.body[0].span), "x = a + b");
    assert_eq!(text(src, m.body[1].span), "if c : NL INDENT pass");
    assert_eq!(
        m.span,
        Span {
            start: 0,
            end: src.len()
        }
    );
}

#[test]
fn expression_spans() {
    let src = "f ( a , k = ( b + c ) ) [ 0 ] NL";
    let StmtKind::Expr(e) = &parse_src(src).body[0].kind else {
        panic!("not an expression statement");
    };
    assert_eq!(text(src, e.span), "f ( a , k = ( b + c ) ) [ 0 ]");
    let ExprKind::Subscript { value, .. } = &e.kind else {
        panic!("not a subscript");
    };
    let ExprKind::Call { args, .. } = &value.kind else {
        panic!("not a call");
    };
    assert_eq!(text(src, args[1].span), "k = ( b + c )");
    assert_eq!(text(src, args[1].value.span), "( b + c )");
}

#[test]
fn function_spans() {
    let src = "temper def f ( a : int = 1 ) -> int : NL INDENT return a NL DEDENT";
    let StmtKind::FuncDef(f) = &parse_src(src).body[0].kind else {
        panic!("not a def");
    };
    assert_eq!(
        text(src, f.span),
        "temper def f ( a : int = 1 ) -> int : NL INDENT return a"
    );
    assert_eq!(text(src, f.name.span), "f");
    assert_eq!(text(src, f.params[0].span), "a : int = 1");
}

#[test]
fn error_span_points_at_offending_token() {
    let src = "x = ( 1 + ) NL";
    let d = parse_err(src);
    assert_eq!(d.message, "expected an expression, found `)`");
    assert_eq!(text(src, d.span), ")");
}

// Nesting limit.

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let deep = format!("x = {}1{} NL", "( ".repeat(500), " )".repeat(500));
    assert_eq!(err(&deep), "too deeply nested");
    let unary = format!("x = {}1 NL", "- ".repeat(500));
    assert_eq!(err(&unary), "too deeply nested");
    let shallow = format!("x = {}1{} NL", "( ".repeat(50), " )".repeat(50));
    assert_eq!(ok(&shallow), "(= x 1)");
}

// Optional types and `is`.

#[test]
fn optional_types() {
    assert_eq!(ok("x : int | None = None NL"), "(: x int | None None)");
    assert_eq!(
        ok("x : list [ str | None ] | None = None NL"),
        "(: x list[str | None] | None None)"
    );
    assert_eq!(
        ok("def f ( a : int | None = None ) -> tuple [ int , str | None ] : NL INDENT return a NL DEDENT"),
        "(def f ([a: int | None = None]) -> tuple[int, str | None] ((return a)))"
    );
}

#[test]
fn optional_type_errors() {
    assert_eq!(
        err("x : None | None = None NL"),
        "`None | None` is just `None`"
    );
    assert_eq!(
        err("x : int | str = 1 NL"),
        "expected `None` (only `T | None` unions exist), found name `str`"
    );
    assert_eq!(
        err("x : int | None | None = 1 NL"),
        "expected `=`, found `|`"
    );
    assert_eq!(err("x : | None = 1 NL"), "expected a type, found `|`");
}

#[test]
fn optional_type_span_covers_the_suffix() {
    let src = "x : list [ int | None ] | None = y NL";
    let StmtKind::AnnAssign { annotation, .. } = &parse_src(src).body[0].kind else {
        panic!("not an annotated assignment");
    };
    assert!(annotation.optional);
    assert_eq!(text(src, annotation.span), "list [ int | None ] | None");
    assert_eq!(text(src, annotation.name.span), "list");
    assert_eq!(text(src, annotation.args[0].span), "int | None");
}

#[test]
fn is_and_is_not_compare() {
    assert_eq!(ok("x is None NL"), "(cmp x is None)");
    assert_eq!(ok("x is not None NL"), "(cmp x is-not None)");
    assert_eq!(ok("not x is None NL"), "(not (cmp x is None))");
    assert_eq!(ok("a < b is not None NL"), "(cmp a < b is-not None)");
    assert_eq!(ok("x + 1 is None NL"), "(cmp (+ x 1) is None)");
    assert_eq!(
        ok("a is None and b is not None NL"),
        "(and (cmp a is None) (cmp b is-not None))"
    );
    assert_eq!(err("x is NL"), "expected an expression, found newline");
    assert_eq!(err("x not is None NL"), "expected newline, found `not`");
}

// Source level: lexer, parser and inference together. Mirrors the
// Optional and narrowing sections of `infer/tests.rs`.

fn check(src: &str) -> diag::Result<Types> {
    infer(&parse(&lex(src)?)?)
}

fn types(src: &str) -> Types {
    check(src).unwrap_or_else(|d| panic!("{src:?}: {d}"))
}

fn type_err(src: &str) -> String {
    match check(src) {
        Ok(_) => panic!("{src:?} type-checked"),
        Err(d) => d.message,
    }
}

/// Span of the last occurrence of `needle` in `src`.
fn last(src: &str, needle: &str) -> Span {
    let start = src.rfind(needle).unwrap();
    Span {
        start,
        end: start + needle.len(),
    }
}

fn int_or_none() -> Type {
    Type::Optional(Box::new(Type::Int))
}

#[test]
fn source_optional_variable_holds_none_or_its_type() {
    let src = "x: int | None = None\nx = 5\nx = None\n";
    let t = types(src);
    assert_eq!(t.at(Span { start: 21, end: 22 }), Some(&int_or_none()));
}

#[test]
fn source_optional_is_not_its_inner_type() {
    assert_eq!(
        type_err("x: int | None = 1\nx + 1\n"),
        "unsupported operand types for +: int | None and int"
    );
    let msg = type_err("x: int | None = 1\ny: int = x\n");
    assert!(msg.ends_with("expected int, found int | None"), "{msg}");
    assert_eq!(
        type_err("x: int | None = 1\nx < 2\n"),
        "cannot order int | None against int"
    );
}

#[test]
fn source_list_of_optional_takes_mixed_literal_and_appends() {
    let src = "xs: list[int | None] = [None, 1]\nxs.append(None)\nxs.append(2)\nys = xs\n";
    let t = types(src);
    let list = Type::List(Box::new(int_or_none()));
    assert_eq!(t.at(last(src, "ys")), Some(&list));
}

#[test]
fn source_optional_return_and_parameter() {
    let src = "def f(x) -> int | None:\n    if x:\n        return 1\n";
    let ret = types(src).func("f").map(|f| f.ret.clone());
    assert_eq!(ret, Some(int_or_none()));
    let src = "def g(x: int | None = None):\n    return x\ng()\ng(3)\n";
    types(src);
}

#[test]
fn source_none_or_none_is_a_parse_error() {
    let d = check("x: None | None = None\n").unwrap_err();
    assert_eq!(d.kind, DiagKind::Parse);
    assert_eq!(d.message, "`None | None` is just `None`");
}

#[test]
fn source_is_not_none_narrows_the_branch() {
    let src = "x: int | None = None\nif x is not None:\n    y = x + 1\n";
    assert_eq!(types(src).at(last(src, "y")), Some(&Type::Int));
}

#[test]
fn source_is_none_narrows_the_else_branch() {
    types("x: int | None = None\nif x is None:\n    pass\nelse:\n    x + 1\n");
}

#[test]
fn source_narrowing_does_not_leak_or_survive_assignment() {
    let leak = "x: int | None = None\nif x is not None:\n    pass\nx + 1\n";
    let msg = "unsupported operand types for +: int | None and int";
    assert_eq!(type_err(leak), msg);
    let reassigned = "x: int | None = 1\nif x is not None:\n    x = None\n    x + 1\n";
    assert_eq!(type_err(reassigned), msg);
}

#[test]
fn source_narrowed_parameter_and_while() {
    let src =
        "def f(x: int | None = None):\n    if x is not None:\n        return x * 2\n    return 0\n";
    let ret = types(src).func("f").map(|f| f.ret.clone());
    assert_eq!(ret, Some(Type::Int));
    types("x: int | None = 1\nwhile x is not None:\n    x + 1\n    break\n");
}

#[test]
fn source_is_with_a_non_none_operand_is_a_type_error() {
    assert_eq!(
        type_err("1 is 1\n"),
        "`is` compares only with None in phase 2; use `==` for values"
    );
}

#[test]
fn source_guard_narrows_what_follows() {
    let src =
        "def f(x: int | None = None):\n    if x is None:\n        return 0\n    return x + 1\n";
    let ret = types(src).func("f").map(|f| f.ret.clone());
    assert_eq!(ret, Some(Type::Int));
    let src = "ys: list[int | None] = [None, 1]\ntotal = 0\nfor y in ys:\n    if y is None:\n        continue\n    total += y\n";
    types(src);
    let falls = "x: int | None = None\nif x is None:\n    pass\nx + 1\n";
    let msg = "unsupported operand types for +: int | None and int";
    assert_eq!(type_err(falls), msg);
}

#[test]
fn source_guard_with_break_narrows_the_loop_body() {
    types(
        "x: int | None = 1\nwhile True:\n    if x is None:\n        break\n    x + 1\n    break\n",
    );
}

#[test]
fn source_guard_exits_through_a_loop_else() {
    let head = "ys: list[int | None] = [None, 1]\ntotal = 0\nfor x in ys:\n    if x is None:\n";
    let tail = "    total += x\n";
    let msg = "unsupported operand types for +: int and int | None";
    let exits = [
        "        while False:\n            pass\n        else:\n            continue\n",
        "        for z in ys:\n            pass\n        else:\n            break\n",
        "        while True:\n            pass\n",
    ];
    for inner in exits {
        types(&format!("{head}{inner}{tail}"));
    }
    let stays = [
        "        for z in ys:\n            continue\n",
        "        for z in ys:\n            break\n        else:\n            continue\n",
        "        while True:\n            break\n",
    ];
    for inner in stays {
        assert_eq!(type_err(&format!("{head}{inner}{tail}")), msg, "{inner}");
    }
}

#[test]
fn source_loop_else_return_does_not_fall_through() {
    let src = "def f(xs):\n    for x in xs:\n        pass\n    else:\n        return 1\n";
    let ret = types(src).func("f").map(|f| f.ret.clone());
    assert_eq!(ret, Some(Type::Int));
}

#[test]
fn source_for_target_narrows_before_its_element_type_is_settled() {
    let src = "def first_present(xs: list[int | None]) -> int:\n    for x in xs:\n        if x is not None:\n            return x\n    return -1\n";
    let ret = types(src).func("first_present").map(|f| f.ret.clone());
    assert_eq!(ret, Some(Type::Int));
    let src = "def f(xs: list[int | None]):\n    for x in xs:\n        if x is None:\n            return 0\n        return x\n    return 1\n";
    types(src);
}

// Imports (M6a): source level, so `import` and dots lex as written.

fn src_parse(src: &str) -> diag::Result<Module> {
    parse(&lex(src)?)
}

fn src_err(src: &str) -> String {
    match src_parse(src) {
        Ok(m) => panic!("{src:?} parsed: {}", sx_module(&m)),
        Err(d) => d.message,
    }
}

#[test]
fn import_binds_each_module_variable_from_its_initializer_once() {
    let m = src_parse("import sys, sys\nimport sys\nprint(sys.argv)\n").unwrap();
    let names: Vec<String> = m.body.iter().map(sx_stmt).collect();
    assert_eq!(
        names,
        ["(= sys.argv (call sys.__argv__))", "(call print sys.argv)"]
    );
}

#[test]
fn a_module_name_spans_its_attributes() {
    let src = "import sys\nn = len(sys.argv)\n";
    let m = src_parse(src).unwrap();
    let StmtKind::Assign { value, .. } = &m.body[1].kind else {
        panic!("an assignment")
    };
    let ExprKind::Call { args, .. } = &value.kind else {
        panic!("a call")
    };
    assert_eq!(text(src, args[0].value.span), "sys.argv");
    assert_eq!(text(src, m.body[0].span), "import sys");
}

#[test]
fn import_comes_first_at_module_level() {
    let late = "`import` must come at module top level, before other statements";
    assert_eq!(src_err("x = 1\nimport sys\n"), late);
    assert_eq!(src_err("def f():\n    import sys\n"), late);
    assert_eq!(
        src_err("import json\n"),
        "the standard module `json` is not supported yet; \
         the standard modules are: concurrent.futures, hashlib, os, sys"
    );
    assert_eq!(
        src_err("from sys import argv\nx = 1\nfrom os import path\n"),
        late
    );
    assert_eq!(
        src_err("import sys.argv\n"),
        "`sys.argv` is not a module; the `sys` names are: sys.argv"
    );
}

/// D84: `as`, dotted and `from` forms on standard modules, each use
/// resolving to the module's own dotted name.
#[test]
fn standard_modules_take_from_dotted_and_as() {
    let cases = [
        ("import sys as s\nprint(s.argv)\n", "(call print sys.argv)"),
        (
            "import os.path\nprint(os.path.isdir('a'))\n",
            "(call print (call os.path.isdir 'a'))",
        ),
        (
            "import os.path as p\nprint(p.isdir('a'))\n",
            "(call print (call os.path.isdir 'a'))",
        ),
        (
            "from os import path\nprint(path.isdir('a'))\n",
            "(call print (call os.path.isdir 'a'))",
        ),
        (
            "from os.path import join as j\nprint(j('a', 'b'))\n",
            "(call print (call os.path.join 'a' 'b'))",
        ),
        (
            "from concurrent.futures import ThreadPoolExecutor\nprint(ThreadPoolExecutor)\n",
            "(call print concurrent.futures.ThreadPoolExecutor)",
        ),
    ];
    for (src, want) in cases {
        let m = src_parse(src).unwrap();
        let last = m.body.last().map(sx_stmt).unwrap_or_default();
        assert_eq!(last, want, "{src}");
    }
}

/// `from` copies a module variable into a global, after the module's
/// own initializer, so both name one object (D83).
#[test]
fn from_copies_a_module_variable() {
    let m = src_parse("from sys import argv as a\nprint(a)\n").unwrap();
    let names: Vec<String> = m.body.iter().map(sx_stmt).collect();
    assert_eq!(
        names,
        [
            "(= sys.argv (call sys.__argv__))",
            "(= a sys.argv)",
            "(call print a)"
        ]
    );
}

#[test]
fn from_and_as_refuse_what_cpython_would_not_bind_here() {
    let cases = [
        (
            "from . import x\n",
            "relative imports are not supported yet",
        ),
        (
            "from .a import x\n",
            "relative imports are not supported yet",
        ),
        (
            "from os import *\n",
            "`import *` is not supported; import the names you use",
        ),
        (
            "from sys import platform\n",
            "`sys.platform` is not supported; the `sys` names are: sys.argv",
        ),
        (
            "from os import path\npath = 1\n",
            "`os.path` is a module, not a value",
        ),
        (
            "from os import path\ndef f(path):\n    pass\n",
            "`path` names an imported module; choose another name",
        ),
        (
            "from os.path import join\ndef join():\n    pass\n",
            "`join` names an imported function; choose another name",
        ),
        (
            "import sys as os\nprint(os.listdir())\n",
            "`sys.listdir` is not supported",
        ),
        ("import concurrent\n", "write `import concurrent.futures`"),
        (
            "from hashlib import sha256\nx = sha256(b)\n",
            "is supported only as `hashlib.sha256(...).hexdigest()` yet",
        ),
    ];
    for (src, want) in cases {
        let got = src_err(src);
        assert!(got.contains(want), "{src}: {got}");
    }
}

#[test]
fn future_annotations_start_a_module_and_make_its_annotations_lazy() {
    let later = "def f(n: Node) -> Node:\n    return n\nclass Node:\n    pass\n";
    let rec =
        "from dataclasses import dataclass\n@dataclass(frozen=True)\nclass P:\n    nxt: P | None\n";
    for src in [
        format!("from __future__ import annotations\n{later}"),
        format!("from __future__ import annotations\nfrom __future__ import (annotations,)\n{rec}"),
    ] {
        assert!(src_parse(&src).is_ok(), "{src}");
    }
    assert!(src_err(later).contains("`Node` is not defined yet"));
    assert!(src_err(rec).contains("`P` is not defined yet"));
    let cases = [
        (
            "import sys\nfrom __future__ import annotations\n",
            "`from __future__` imports must occur at the beginning of the file",
        ),
        (
            "x = 1\nfrom __future__ import division\n",
            "`from __future__` imports must occur at the beginning of the file",
        ),
        (
            "from __future__ import annotations, division\n",
            "`from __future__ import division` is not supported yet",
        ),
        (
            "from __future__ import braces\n",
            "future feature `braces` is not defined",
        ),
        (
            "from __future__ import *\n",
            "future feature `*` is not defined",
        ),
        (
            "from __future__ import annotations as a\n",
            "`as` on a `__future__` import is not supported yet",
        ),
        (
            "import __future__\n",
            "the standard module `__future__` is not supported yet",
        ),
    ];
    for (src, want) in cases {
        let got = src_err(src);
        assert!(got.contains(want), "{src}: {got}");
    }
}

#[test]
fn a_module_is_not_a_value_and_exposes_only_its_names() {
    assert_eq!(
        src_err("import sys\nx = sys\n"),
        "`sys` is a module, not a value; use one of its names: sys.argv"
    );
    assert_eq!(
        src_err("import sys\nprint(sys.path)\n"),
        "`sys.path` is not supported; the `sys` names are: sys.argv"
    );
    // Without the import, `sys` is an ordinary name.
    assert_eq!(ok("sys = 1 NL"), "(= sys 1)");
}

#[test]
fn a_module_name_cannot_be_rebound_or_assigned_through() {
    let taken = "`sys` names an imported module; choose another name";
    assert_eq!(src_err("import sys\ndef sys():\n    pass\n"), taken);
    assert_eq!(src_err("import sys\ndef f(sys):\n    pass\n"), taken);
    let handler = "import sys\ntry:\n    pass\nexcept ValueError as sys:\n    pass\n";
    assert_eq!(src_err(handler), taken);
    assert!(src_err("import sys\nsys = 1\n").contains("is a module, not a value"));
    let assign = "assigning to a module's name (`sys.argv`) is not supported yet";
    assert_eq!(src_err("import sys\nsys.argv = []\n"), assign);
    assert_eq!(src_err("import sys\nsys.argv += []\n"), assign);
    assert_eq!(src_err("import sys\nsys.argv: list[str] = []\n"), assign);
    let in_def = "import sys\ndef f():\n    sys.argv: list[str] = []\n";
    assert_eq!(src_err(in_def), assign);
    assert_eq!(
        src_err("import sys\nfor sys.argv in [[]]:\n    pass\n"),
        assign
    );
}

#[test]
fn sha256_is_one_call_with_its_hexdigest() {
    let src = "import hashlib\nh = hashlib.sha256(d).hexdigest()\nprint(h)\n";
    let m = src_parse(src).unwrap_or_else(|d| panic!("{d}"));
    let StmtKind::Assign { value, .. } = &m.body[0].kind else {
        panic!("an assignment")
    };
    let ExprKind::Call { func, args } = &value.kind else {
        panic!("a call")
    };
    assert!(matches!(&func.kind, ExprKind::Name(n) if n == "hashlib.sha256"));
    assert_eq!(args.len(), 1);
    assert_eq!(text(src, value.span), "hashlib.sha256(d).hexdigest()");
    let only = "`hashlib.sha256(...)` is supported only as `hashlib.sha256(...).hexdigest()` yet; \
                its object is not a value";
    for bad in [
        "h = hashlib.sha256\n",
        "h = hashlib.sha256(d)\n",
        "h = hashlib.sha256(d).digest()\n",
        "h = hashlib.sha256(d).hexdigest\n",
        "h = hashlib.sha256(d).hexdigest(1)\n",
    ] {
        assert_eq!(src_err(&format!("import hashlib\n{bad}")), only, "{bad}");
    }
}

// Rust binding stubs (M6b).

fn stub(src: &str) -> diag::Result<Binding> {
    super::parse_stub(&lex(src)?, "fastfs")
}

fn stub_err(src: &str) -> String {
    match stub(src) {
        Ok(b) => panic!("{src:?} parsed: {b:?}"),
        Err(d) => {
            assert_eq!(d.kind, DiagKind::Parse, "{d}");
            d.message
        }
    }
}

const FASTFS: &str = "\
# A stub.
temper def file_xxh3(path: str) -> str raises OSError: ...

temper def add(a: int, b: int) -> int:
    ...
temper def nothing() -> None: ...
";

fn fastfs() -> Binding {
    stub(FASTFS).unwrap_or_else(|d| panic!("{d}"))
}

#[test]
fn stub_signatures_get_dotted_names() {
    let b = fastfs();
    assert_eq!(b.name, "fastfs");
    let names: Vec<&str> = b.funcs.iter().map(|f| f.name.name.as_str()).collect();
    assert_eq!(names, ["fastfs.file_xxh3", "fastfs.add", "fastfs.nothing"]);
    let xxh3 = &b.funcs[0];
    assert_eq!(xxh3.params[0].name.name, "path");
    assert_eq!(xxh3.params[0].annotation.name.name, "str");
    assert_eq!(xxh3.returns.name.name, "str");
    assert_eq!(xxh3.raises[0].name, "OSError");
    assert!(b.funcs[1].raises.is_empty());
    for empty in ["", "# only a comment\n"] {
        assert_eq!(
            stub_err(empty),
            "the stub for `fastfs` declares no functions"
        );
    }
}

#[test]
fn stub_rules_are_named_errors() {
    let only = "a stub holds only `temper def` signatures whose bodies are `...`";
    assert_eq!(stub_err("def f() -> int: ...\n"), only);
    assert_eq!(stub_err("x = 1\n"), only);
    assert_eq!(stub_err("import os\n"), only);
    assert_eq!(stub_err("temper def f() -> int:\n    pass\n"), only);
    assert_eq!(stub_err("temper def f() -> int:\n    ...\n    ...\n"), only);
    assert_eq!(
        stub_err("temper def f(mut x: int) -> int: ...\n"),
        "`mut` and `owned` are not supported in stubs yet; drop it from `x`"
    );
    assert!(
        stub_err("temper def f(owned x: str) -> int: ...\n").contains("not supported in stubs yet")
    );
    assert_eq!(
        stub_err("temper def f(x: int = 1) -> int: ...\n"),
        "stub parameter `x` cannot have a default"
    );
    assert_eq!(
        stub_err("temper def f(x) -> int: ...\n"),
        "stub parameter `x` must be annotated"
    );
    assert_eq!(
        stub_err("temper def f(x: int): ...\n"),
        "stub `f` must annotate its return type"
    );
    assert_eq!(
        stub_err("temper def f() -> int: ...\ntemper def f() -> int: ...\n"),
        "`fastfs.f` is defined twice"
    );
    assert_eq!(
        stub_err("temper def f(x: int, x: int) -> int: ...\n"),
        "duplicate parameter `x`"
    );
}

#[test]
fn stub_types_are_those_that_cross() {
    for ty in ["int", "float", "bool", "str", "bytes", "list[str]"] {
        let src = format!("temper def f(x: {ty}) -> {ty}: ...\n");
        assert!(stub(&src).is_ok(), "{ty}");
    }
    assert!(stub("temper def f() -> None: ...\n").is_ok());
    let param = "a stub parameter must be one of int, float, bool, str, bytes, list[str];";
    for bad in [
        "None",
        "list[int]",
        "list[str | None]",
        "list[list[str]]",
        "list[str] | None",
        "int | None",
        "dict[str, int]",
        "Exception",
    ] {
        let src = format!("temper def f(x: {bad}) -> int: ...\n");
        assert!(stub_err(&src).starts_with(param), "{bad}");
    }
    let result = "a stub result must be one of int, float, bool, str, bytes, list[str], or None;";
    for bad in ["list[bytes]", "str | None", "tuple[int, int]"] {
        let src = format!("temper def f() -> {bad}: ...\n");
        assert!(stub_err(&src).starts_with(result), "{bad}");
    }
}

#[test]
fn stub_raises_only_os_error_or_exception() {
    assert!(stub("temper def f() -> int raises Exception: ...\n").is_ok());
    assert!(stub("temper def f() -> int raises OSError, Exception: ...\n").is_ok());
    let msg = stub_err("temper def f() -> int raises ValueError: ...\n");
    assert!(
        msg.starts_with("a stub raises only `OSError` or `Exception`"),
        "{msg}"
    );
    let msg = stub_err("temper def f() -> int raises FileNotFoundError: ...\n");
    assert!(msg.ends_with("not `FileNotFoundError`"), "{msg}");
    assert_eq!(
        stub_err("temper def f() -> int raises OSError, OSError: ...\n"),
        "`OSError` is listed twice in `raises`"
    );
}

#[test]
fn ellipsis_outside_a_stub_says_use_pass() {
    let stmt = "`...` is for stub bodies only; use pass";
    assert_eq!(src_err("def f():\n    ...\n"), stmt);
    assert_eq!(src_err("if True:\n    ...\n"), stmt);
    // No body shares the `:` line, so `pass` moves down too.
    let inline = "`...` is for stub bodies only; use pass on its own indented line";
    assert_eq!(src_err("def f() -> None: ...\n"), inline);
    assert_eq!(src_err("if True: ...\n"), inline);
    assert!(src_parse("def f() -> None:\n    pass\nif True:\n    pass\n").is_ok());
    assert_eq!(src_err("...\n"), stmt);
    // As a value, `pass` would not parse: no fix-it.
    let value = "`...` (Ellipsis) is not supported; it is legal only as a stub body";
    assert_eq!(src_err("x = ...\n"), value);
    assert_eq!(src_err("print(...)\n"), value);
    assert_eq!(src_err("x = [1, ...]\n"), value);
}

fn with_fastfs(src: &str) -> diag::Result<Module> {
    super::parse_with(&lex(src)?, &[fastfs()])
}

#[test]
fn importing_a_binding_resolves_its_functions() {
    let m = with_fastfs("import sys, fastfs\nx = fastfs.add(1, b=2)\n")
        .unwrap_or_else(|d| panic!("{d}"));
    let stubs: Vec<&str> = m.stubs.iter().map(|f| f.name.name.as_str()).collect();
    assert_eq!(stubs, ["fastfs.file_xxh3", "fastfs.add", "fastfs.nothing"]);
    let StmtKind::Assign { value, .. } = &m.body[1].kind else {
        panic!("{:?}", m.body[1]);
    };
    let ExprKind::Call { func, .. } = &value.kind else {
        panic!("{value:?}");
    };
    assert_eq!(func.kind, ExprKind::Name("fastfs.add".to_string()));
    // Imported twice, its functions come once; given but not imported,
    // none come.
    // A binding built by hand with no functions cannot be imported.
    let empty = Binding {
        name: "empty".to_string(),
        funcs: Vec::new(),
    };
    let tokens = lex("import empty\nprint(1)\n").unwrap_or_else(|d| panic!("{d}"));
    let err = super::parse_with(&tokens, &[empty])
        .err()
        .map(|d| d.message);
    assert_eq!(
        err.as_deref(),
        Some("the stub for `empty` declares no functions")
    );
    let twice = with_fastfs("import fastfs\nimport fastfs\n").map(|m| m.stubs.len());
    assert_eq!(twice, Ok(3));
    assert_eq!(with_fastfs("x = 1\n").map(|m| m.stubs.len()), Ok(0));
}

#[test]
fn a_binding_name_is_not_a_value() {
    let err = |src: &str| {
        with_fastfs(src)
            .err()
            .map(|d| d.message)
            .unwrap_or_default()
    };
    let funcs = "fastfs.file_xxh3, fastfs.add, fastfs.nothing";
    assert_eq!(
        err("import fastfs\nx = fastfs\n"),
        format!("`fastfs` is a Rust binding, not a value; use one of its functions: {funcs}")
    );
    assert_eq!(
        err("import fastfs\nfastfs.nope()\n"),
        format!("the stub for `fastfs` has no `nope`; its functions are: {funcs}")
    );
    assert_eq!(
        err("import fastfs\ndef fastfs():\n    pass\n"),
        "`fastfs` names an imported Rust binding; choose another name"
    );
    let msg = "`fastfs` is not a standard module, and no pythrust.toml names it; \
               the standard modules are: concurrent.futures, hashlib, os, sys";
    assert_eq!(src_err("import fastfs\n"), msg);
}

/// `pause` covers the whole expression to its right, conditional
/// included, and leaves the tree as it is without it (M6c).
#[test]
fn pause_covers_the_expression_to_its_right() {
    let src = "x = pause f(pause g()) if c else d\n";
    let m = parse(&lex(src).unwrap()).unwrap();
    let plain = parse(&lex("x = f(g()) if c else d\n").unwrap()).unwrap();
    assert_eq!(sx_module(&m), sx_module(&plain));
    let covered: Vec<&str> = m
        .pauses
        .iter()
        .map(|p| &src[p.covers.start..p.covers.end])
        .collect();
    assert_eq!(covered, ["g()", "f(pause g()) if c else d"]);
    let keyword = m
        .pauses
        .iter()
        .map(|p| &src[p.keyword.start..p.keyword.end]);
    assert!(keyword.into_iter().all(|k| k == "pause"));
    let bare = parse(&lex("x = pause\n").unwrap()).unwrap_err();
    assert_eq!(bare.kind, DiagKind::Parse);
    let ret = parse(&lex("def h():\n    return pause f(), 1\n").unwrap()).unwrap();
    assert_eq!(ret.pauses.len(), 1);
    for src in ["x = 1 + pause f()\n", "x = not pause f()\n"] {
        let after = parse(&lex(src).unwrap()).unwrap_err();
        assert_eq!(after.kind, DiagKind::Parse);
        assert!(after.message.contains("put it first, or wrap"), "{after}");
    }
    let wrapped = "x = not (pause f())\n";
    let m = parse(&lex(wrapped).unwrap()).unwrap();
    let covered = &wrapped[m.pauses[0].covers.start..m.pauses[0].covers.end];
    assert_eq!(covered, "(pause f())");
}

/// `with open(...)` desugars to a hidden temp closed by `finally`
/// (D109-D113); other values stay executor `with`s.
#[test]
fn with_open_is_try_finally_on_a_hidden_file() {
    let body = ": NL INDENT y NL DEDENT";
    assert_eq!(
        ok(&format!("with open ( p ) as f {body}")),
        "(= with#0 (call open p)) (= f with#0) (try (y) (finally ((call (. with#0 close)))))"
    );
    assert_eq!(
        ok(&format!("with open ( p ) {body}")),
        "(= with#0 (call open p)) (try (y) (finally ((call (. with#0 close)))))"
    );
    assert_eq!(
        ok(&format!(
            "with open ( a ) as f , ThreadPoolExecutor ( ) as e , open ( b ) {body}"
        )),
        "(= with#0 (call open a)) (= f with#0) (try ((with (call ThreadPoolExecutor) e \
         ((= with#1 (call open b)) (try (y) (finally ((call (. with#1 close)))))))) \
         (finally ((call (. with#0 close)))))"
    );
    assert_eq!(
        ok(&format!("with ( open ( p ) ) as f {body}")),
        "(= with#0 (call open p)) (= f with#0) (try (y) (finally ((call (. with#0 close)))))"
    );
    assert_eq!(
        ok(&format!("with ThreadPoolExecutor ( ) as e {body}")),
        "(with (call ThreadPoolExecutor) e (y))"
    );
    assert_eq!(
        err(&format!("with ThreadPoolExecutor ( ) {body}")),
        "`with` needs a name: write `with value as name:`"
    );
    assert_eq!(
        err(&format!(
            "with ( open ( a ) as f , open ( b ) as g ) {body}"
        )),
        "parenthesized `with` items are not supported yet"
    );
    assert_eq!(
        err(&format!("with open ( p ) as f [ 0 ] {body}")),
        "expected `:`, found `[`"
    );
}

/// A program's own `def open`, before or after the `with`, makes an
/// `open(...)` item an ordinary object item, as CPython calls it (D115,
/// D153).
#[test]
fn with_over_a_user_open_is_an_object_item() {
    let body = ": NL INDENT y NL DEDENT";
    let def = "def open ( n ) : NL INDENT return n NL DEDENT";
    let enter = "(= e (call (. with#0 __enter__)))";
    let before = ok(&format!("{def} with open ( p ) as e {body}"));
    assert!(before.contains(enter), "{before}");
    let after = ok(&format!("with open ( p ) as e {body} {def}"));
    assert!(after.contains(enter), "{after}");
    let tempered = ok(&format!("temper {def} with open ( p ) as e {body}"));
    assert!(tempered.contains(enter), "{tempered}");
    let bare = ok(&format!("{def} with open ( p ) {body}"));
    assert!(bare.contains("(call (. with#0 __enter__))"), "{bare}");
}

/// An object item is PEP 343 over try/finally: `__enter__`, the body,
/// and one `__exit__` on every way out (D153).
#[test]
fn with_an_object_enters_and_exits_once() {
    let got = ok("with m ( ) as e : NL INDENT y NL DEDENT");
    let exit = "(call (. with#0 __exit__) with#0.exc with#0.exc with#0.exc)";
    assert!(
        got.starts_with("(= with#0 (call m)) (= e (call (. with#0 __enter__)))"),
        "{got}"
    );
    assert!(
        got.contains(&format!("(if (not {exit}) ((raise with#0.err)))")),
        "{got}"
    );
    assert!(
        got.contains(&format!("(if (not with#0.done) ({exit}))")),
        "{got}"
    );
    assert!(ok("with m as e : NL INDENT y NL DEDENT").contains("__enter__"));
    let items = ok("with m ( ) as a , n ( ) : NL INDENT y NL DEDENT");
    assert!(items.contains("with#1"), "{items}");
}
