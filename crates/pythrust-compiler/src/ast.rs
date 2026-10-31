//! Abstract syntax tree. Mirrors `docs/grammar.md` one-to-one.
//!
//! Owned and immutable once built. Every node carries a `Span`.
//! Invariants the types do not encode are noted on the field.

use crate::span::Span;

/// `module = (stmt)* ENDMARKER`
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub body: Vec<Stmt>,
    /// The functions of the Rust bindings the module imports (M6b).
    pub stubs: Vec<StubDef>,
    pub span: Span,
}

/// A Rust binding (M6b): the name `import` brings in and the functions
/// its stub declares (`docs/research/rust-interop.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub name: String,
    pub funcs: Vec<StubDef>,
}

/// A stub's `'temper' 'def' NAME '(' [params] ')' '->' type [raises] ':'
/// '...'`: a signature with no body, implemented in Rust. Every type is
/// one that crosses the boundary, checked by the parser.
#[derive(Debug, Clone, PartialEq)]
pub struct StubDef {
    /// The dotted name calls resolve to: `fastfs.file_xxh3`.
    pub name: Ident,
    pub params: Vec<StubParam>,
    pub returns: TypeExpr,
    /// `OSError` or `Exception`, at most once each; empty when none.
    pub raises: Vec<Ident>,
    pub span: Span,
}

/// A stub parameter: always `read`, annotated, and without a default.
#[derive(Debug, Clone, PartialEq)]
pub struct StubParam {
    pub name: Ident,
    pub annotation: TypeExpr,
}

/// `block = NEWLINE INDENT stmt+ DEDENT`. Never empty when it comes from
/// the parser; an empty `orelse` means the clause is absent.
pub type Block = Vec<Stmt>;

/// A name with its own span, where a `NAME` appears outside an
/// `Expr` or `Target` (function, parameter, keyword-arg, attribute, type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `target_list '=' expr_list`. Unpacking is a `TargetKind::Tuple`.
    Assign {
        target: Target,
        value: Expr,
    },
    /// `NAME ':' type '=' expr`
    AnnAssign {
        target: Ident,
        annotation: TypeExpr,
        value: Expr,
    },
    /// `target op= expr`. `target` is never a `TargetKind::Tuple`.
    AugAssign {
        target: Target,
        op: BinOp,
        value: Expr,
    },
    /// `'return' [expr_list]`
    Return(Option<Expr>),
    Break,
    Continue,
    Pass,
    /// A bare `expr_list` evaluated for effect.
    Expr(Expr),
    FuncDef(FuncDef),
    /// `'if' expr ':' block ('elif' expr ':' block)* ['else' ':' block]`
    If {
        branch: Branch,
        elifs: Vec<Branch>,
        orelse: Block,
    },
    /// `'while' expr ':' block ['else' ':' block]`
    While {
        branch: Branch,
        orelse: Block,
    },
    /// `'for' target_list 'in' expr ':' block ['else' ':' block]`
    For {
        target: Target,
        iter: Expr,
        body: Block,
        orelse: Block,
    },
    /// `'raise' expr`
    Raise(Expr),
    /// `'try' ':' block handler+`. `handlers` is never empty.
    Try {
        body: Block,
        handlers: Vec<Handler>,
    },
}

/// `'except' classes ['as' NAME] ':' block`, where `classes` is a name
/// or a parenthesized tuple of them. Each names a builtin exception
/// (checked by inference); `classes` is never empty. `binding` is the
/// `as` name.
#[derive(Debug, Clone, PartialEq)]
pub struct Handler {
    pub classes: Vec<Ident>,
    pub binding: Option<Ident>,
    pub body: Block,
    pub span: Span,
}

/// A condition and the block it guards.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub test: Expr,
    pub body: Block,
    pub span: Span,
}

/// `['temper'] 'def' NAME '(' [params] ')' ['->' type] [raises] ':' block`
#[derive(Debug, Clone, PartialEq)]
pub struct FuncDef {
    pub mode: Mode,
    pub name: Ident,
    pub params: Vec<Param>,
    pub returns: Option<TypeExpr>,
    /// The `raises` clause's exception names (D8); empty when there is
    /// none. Names are checked by inference.
    pub raises: Vec<Ident>,
    pub body: Block,
    pub span: Span,
}

/// Tempering level of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Loose,
    Tempered,
}

/// `[mode] NAME [':' type] ['=' expr]`
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mode: ParamMode,
    pub name: Ident,
    pub annotation: Option<TypeExpr>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// A parameter's written ownership mode (D2): `mode = 'mut' | 'owned'`,
/// contextual words. `read` is the default and is never written, so
/// [`ParamMode::Read`] means none was; checked against the body by `own`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamMode {
    Read,
    Mut,
    Owned,
}

/// `type = type_atom ['|' 'None']`, with
/// `type_atom = (NAME | 'None') ['[' type (',' type)* ']']`. `None` is
/// spelled `name: "None"`. `args` empty means no brackets. `optional` is
/// the `| None` suffix; `span` covers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeExpr {
    pub name: Ident,
    pub args: Vec<TypeExpr>,
    pub optional: bool,
    pub span: Span,
}

/// An assignment target, validated after parsing. Calls, literals and
/// attributes cannot be represented here.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub kind: TargetKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TargetKind {
    Name(String),
    /// `postfix` ending in `subscript_op`: `value[index]`.
    Subscript {
        value: Box<Expr>,
        index: Box<Expr>,
    },
    /// `target_list` with a comma, or `'(' target_list ')'`: unpacks.
    Tuple(Vec<Target>),
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Name(String),
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    /// `expr_list` with a comma, `()` or a parenthesized tuple.
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    /// `'{' [expr ':' expr (',' expr ':' expr)* [',']] '}'`; `{}` is an
    /// empty dict.
    Dict(Vec<(Expr, Expr)>),
    /// `'{' expr (',' expr)* [','] '}'`: never empty (`{}` is a dict).
    Set(Vec<Expr>),
    /// `'-' factor`, `'+' factor` or `'not' not_expr`.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// Arithmetic: `sum`, `term`, `power`.
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `and` / `or`: short-circuit, returns an operand.
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Chained comparison `left op1 a op2 b ...`. `rest` is never empty.
    Compare {
        left: Box<Expr>,
        rest: Vec<(CmpOp, Expr)>,
    },
    /// `body 'if' test 'else' orelse`
    IfExp {
        test: Box<Expr>,
        body: Box<Expr>,
        orelse: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Arg>,
    },
    Subscript {
        value: Box<Expr>,
        index: Box<Expr>,
    },
    Attr {
        value: Box<Expr>,
        attr: Ident,
    },
}

/// `[NAME '='] expr`
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// `/`, true division.
    Div,
    /// `//`, floor division.
    FloorDiv,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
    NotIn,
    /// `is` / `is not`. Phase 2 allows them only with `None` (checked by
    /// inference); they narrow `T | None` to `T` in a branch.
    Is,
    IsNot,
}
