//! Mid-level IR between the typed AST and a backend. Contract type.
//!
//! A control-flow graph of basic blocks over typed, monomorphic locals;
//! not SSA, locals are mutable slots (`docs/research/ir.md`). Checks that
//! can fail at run time are explicit: a failing `Rvalue` carries the span
//! of its `Inst`, and a `Term::Fail` ends the program with a diagnostic.
//!
//! Covers ints, floats, bools, `None` and strings; lists and tuples
//! follow, as `docs/native.md` lays out.
//!
//! Locals other than parameters start zeroed. A `Str` local is a
//! reference-counted pointer and, at every instruction boundary, either
//! null or the owner of one reference: lowering makes ownership explicit
//! with `Retain`, `Release` and `Forget`, so the backend never decides
//! it. Operands of `Str` type are borrowed; calls take their `Str`
//! arguments owned and return an owned result.

use crate::diag::Diagnostic;
use crate::span::Span;

/// A value type. Every local and operand has exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    /// An IEEE 754 double.
    Float,
    Bool,
    None,
    /// Immutable, reference-counted text.
    Str,
}

/// Index into [`Program::funcs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub usize);

/// Index into [`Func::locals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

/// Index into [`Func::blocks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// A whole program. `main` takes nothing and returns `None`: the
/// module's top-level code.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub main: FuncId,
}

/// One monomorphic function. `blocks[0]` is the entry; `params` are the
/// first locals, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    /// Source name, for symbols and debugging. Not unique: a generic
    /// function has one `Func` per instantiation.
    pub name: String,
    pub params: Vec<LocalId>,
    pub ret: Ty,
    pub locals: Vec<Ty>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub insts: Vec<Inst>,
    pub term: Term,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    /// `dest = value`. `span` is reported if `value` fails.
    Assign {
        dest: LocalId,
        value: Rvalue,
        span: Span,
    },
    /// `print(args...)` with the default separator and line end.
    Print(Vec<Operand>),
    /// Adds a reference to the `Str` in the local (null: nothing).
    Retain(LocalId),
    /// Drops the local's reference (null: nothing) and nulls the local.
    Release(LocalId),
    /// Nulls the local without dropping: its reference was moved.
    Forget(LocalId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operand {
    Local(LocalId),
    Int(i64),
    Float(f64),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    /// Checked int arithmetic: overflow and division by zero fail.
    Int(IntOp, Operand, Operand),
    /// Negation of an `Int` (checked: `-i64::MIN` fails) or a `Float`.
    Neg(Operand),
    /// Both operands have the same type: `Int`, `Float` (IEEE: false
    /// with a NaN, but for `Ne`), `Bool` or `Str` (code point order).
    /// Gives a `Bool`.
    Cmp(CmpOp, Operand, Operand),
    /// Whether int `a + b` overflows. Never fails; ends `range` loops.
    AddOverflows(Operand, Operand),
    Call(FuncId, Vec<Operand>),
    /// A new `Str` holding the literal.
    StrLit(String),
    /// `a + b` on two `Str`s.
    Concat(Operand, Operand),
    /// `s * n`, a `Str` and an `Int`. Fails when too large to allocate.
    Repeat(Operand, Operand),
    /// `s[i]`, a one-char `Str`. Fails when `i` is out of range.
    Index(Operand, Operand),
    /// `len(s)` in chars: an `Int`.
    Len(Operand),
    /// `needle in hay` on two `Str`s: a `Bool`.
    Contains {
        hay: Operand,
        needle: Operand,
    },
    /// `str(x)` for an `Int`, `Float`, `Bool` or `None`.
    ToStr(Operand),
    /// Float arithmetic with Python semantics: `/`, `//` and `%` by zero
    /// and `**` out of range fail.
    Float(FloatOp, Operand, Operand),
    /// Checked int `**`: overflow and a negative exponent fail.
    IntPow(Operand, Operand),
    /// An `Int` or `Bool` as a `Float`.
    ToFloat(Operand),
    /// `int(f)`, truncating. Fails on NaN and out of range.
    FloatToInt(Operand),
    /// `float(s)`. Fails unless `s` is a float literal.
    ParseFloat(Operand),
    /// `int(s)` in base 10. Fails unless `s` is an int literal in range.
    ParseInt(Operand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

/// Int arithmetic with Python semantics: `//` and `%` floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Goto(BlockId),
    /// `cond` is a `Bool`.
    Branch {
        cond: Operand,
        then: BlockId,
        orelse: BlockId,
    },
    Return(Operand),
    /// A runtime error: print it and exit 1.
    Fail(Diagnostic),
    /// Proven unreachable, such as the end of a function that always
    /// returns.
    Unreachable,
}

impl Func {
    /// The type of `op` in this function.
    pub fn ty(&self, op: Operand) -> Ty {
        operand_ty(&self.locals, op)
    }
}

/// The type of `op`, given the types of the locals it may name.
pub fn operand_ty(locals: &[Ty], op: Operand) -> Ty {
    match op {
        Operand::Local(LocalId(i)) => locals[i],
        Operand::Int(_) => Ty::Int,
        Operand::Float(_) => Ty::Float,
        Operand::Bool(_) => Ty::Bool,
        Operand::None => Ty::None,
    }
}
