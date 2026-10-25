//! Parser tests over hand-built token streams.
//!
//! `toks` turns space-separated words into tokens whose spans are the
//! words' byte offsets: `NL`, `INDENT`, `DEDENT` are layout, `'abc'` is a
//! string, words with a `.` (other than `.` itself) are floats. `sx`
//! prints an AST as an s-expression so expected trees stay readable.

use super::parse;
use crate::ast::*;
use crate::diag::{DiagKind, Diagnostic};
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
    }
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
    format!(
        "({word} {} ({params}){returns} {})",
        f.name.name,
        sx_block(&f.body)
    )
}

fn sx_param(p: &Param) -> String {
    let ann = p.annotation.as_ref().map(|t| format!(": {}", sx_type(t)));
    let default = p.default.as_ref().map(|e| format!(" = {}", sx(e)));
    match (ann, default) {
        (None, None) => p.name.name.clone(),
        (a, d) => format!(
            "[{}{}{}]",
            p.name.name,
            a.unwrap_or_default(),
            d.unwrap_or_default()
        ),
    }
}

fn sx_type(t: &TypeExpr) -> String {
    if t.args.is_empty() {
        return t.name.name.clone();
    }
    let args = t.args.iter().map(sx_type).collect::<Vec<_>>().join(", ");
    format!("{}[{args}]", t.name.name)
}

fn sx_target(t: &Target) -> String {
    match &t.kind {
        TargetKind::Name(n) => n.clone(),
        TargetKind::Subscript { value, index } => format!("(sub {} {})", sx(value), sx(index)),
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
        ExprKind::Attr { value, attr } => format!("(. {} {})", sx(value), attr.name),
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
    assert_eq!(
        err("a . b = 1 NL"),
        "attribute assignment is not supported yet"
    );
    assert_eq!(err("a = b = 1 NL"), "chained assignment is not supported");
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
