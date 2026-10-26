//! Mid-level IR between the typed AST and a backend. Contract type.
//!
//! A control-flow graph of basic blocks over typed, monomorphic locals;
//! not SSA, locals are mutable slots (`docs/research/ir.md`). Checks that
//! can fail at run time are explicit: a failing `Rvalue` carries the span
//! of its `Inst`, and a `Term::Fail` ends the program with a diagnostic.
//!
//! This first cut covers ints, bools and `None`; strings, lists, tuples
//! and floats follow, with refcounting, as `docs/native.md` lays out.

use crate::diag::Diagnostic;
use crate::span::Span;

/// A value type. Every local and operand has exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    Bool,
    None,
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operand {
    Local(LocalId),
    Int(i64),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    /// Checked int arithmetic: overflow and division by zero fail.
    Int(IntOp, Operand, Operand),
    /// Checked negation: `-i64::MIN` fails.
    Neg(Operand),
    /// Both operands have the same type, `Int` or `Bool`. Gives a `Bool`.
    Cmp(CmpOp, Operand, Operand),
    /// Whether int `a + b` overflows. Never fails; ends `range` loops.
    AddOverflows(Operand, Operand),
    Call(FuncId, Vec<Operand>),
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
        Operand::Bool(_) => Ty::Bool,
        Operand::None => Ty::None,
    }
}
