//! Mid-level IR between the typed AST and a backend. Contract type.
//!
//! A control-flow graph of basic blocks over typed, monomorphic locals;
//! not SSA, locals are mutable slots (`docs/research/ir.md`). Checks that
//! can fail at run time are explicit: a failing `Rvalue` carries the span
//! of its `Inst`, and a `Term::Fail` ends the program with a diagnostic.
//!
//! Covers ints, floats, bools, `None`, strings, lists, tuples and
//! optionals (`T | None`). List, tuple and optional types are interned
//! in [`Program::types`], so [`Ty`] stays `Copy`.
//!
//! Locals other than parameters start zeroed. A local of a
//! reference-counted type ([`Ty::is_rc`]: strings, lists, tuples) is a
//! pointer and, at every instruction boundary, either null or the owner
//! of one reference: lowering makes ownership explicit with `Retain`,
//! `Release` and `Forget`, so the backend never decides it. Such
//! operands are borrowed; calls take them owned and return an owned
//! result, and rvalues giving one give a new reference.

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
    /// A mutable, reference-counted list; `types[id]` is a
    /// [`Compound::List`].
    List(TypeId),
    /// An immutable, reference-counted tuple; `types[id]` is a
    /// [`Compound::Tuple`].
    Tuple(TypeId),
    /// `T | None`; `types[id]` is a [`Compound::Opt`]. Null is `None`;
    /// otherwise a reference to the value, or for an `Int`, `Float` or
    /// `Bool` to a one-field tuple boxing it.
    Opt(TypeId),
    /// An immutable, reference-counted `range`: a runtime sequence of
    /// three ints, `start`, `stop` and a nonzero `step` ([`Rvalue::Field`]
    /// reads them), whose elements are the ints they produce.
    Range,
}

impl Ty {
    /// Whether values of this type are reference-counted pointers.
    pub fn is_rc(self) -> bool {
        matches!(
            self,
            Ty::Str | Ty::List(_) | Ty::Tuple(_) | Ty::Opt(_) | Ty::Range
        )
    }
}

/// Index into [`Program::types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub usize);

/// What a list, tuple or optional type holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Compound {
    List(Ty),
    Tuple(Vec<Ty>),
    /// The present type: never `None` or itself optional.
    Opt(Ty),
}

/// Index into [`Program::funcs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub usize);

/// Index into [`Program::globals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalId(pub usize);

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
    pub types: Vec<Compound>,
    /// Module-level storage functions can read: zeroed at start, like
    /// locals.
    pub globals: Vec<Ty>,
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
    /// `print(args..., sep=sep, end=end)`; `sep` and `end` are `Str`s,
    /// or `None` for `' '` and `'\n'`.
    Print {
        args: Vec<Operand>,
        sep: Option<Operand>,
        end: Option<Operand>,
    },
    /// Adds a reference to the `Str` in the local (null: nothing).
    Retain(LocalId),
    /// Drops the local's reference (null: nothing) and nulls the local.
    Release(LocalId),
    /// Nulls the local without dropping: its reference was moved.
    Forget(LocalId),
    /// `list.append(value)`: the list adds its own reference.
    Append { list: Operand, value: Operand },
    /// Stores a new reference to `value` in the global, dropping the
    /// one it held (null: nothing).
    SetGlobal { global: GlobalId, value: Operand },
    /// Drops the global's reference (null: nothing) and nulls it: the
    /// end of the module's code.
    DropGlobal(GlobalId),
    /// `list[index] = value`; fails at `span` when out of range.
    SetItem {
        list: Operand,
        index: Operand,
        value: Operand,
        span: Span,
    },
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
    /// with a NaN, but for `Ne`), `Bool`, `Str` (code point order), or a
    /// list or tuple (lexicographic). Gives a `Bool`.
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
    /// `s[i]` on a `Str` (a one-char `Str`) or a list (an element).
    /// Fails when `i` is out of range.
    Index(Operand, Operand),
    /// `len(s)` of a `Str` (in chars), list, tuple or range: an `Int`.
    /// For a range whose length does not fit in one, -1.
    Len(Operand),
    /// `needle in hay`: a substring of a `Str`, or an element of a list.
    /// Gives a `Bool`.
    Contains {
        hay: Operand,
        needle: Operand,
    },
    /// `str(x)` for anything but a `Str`.
    ToStr(Operand),
    /// `list(xs)`: a new list holding a list's elements (a shallow copy).
    CopyList(Operand),
    /// A new list or tuple, of the destination's type, holding `items`.
    NewSeq(Vec<Operand>),
    /// Field `k` of a tuple.
    Field(Operand, usize),
    /// The global's value; a new reference if reference-counted.
    Global(GlobalId),
    /// A value of an optional's present type, or `None`, as that
    /// optional (the destination's type).
    Wrap(Operand),
    /// An optional's present value; lowering proved it is not `None`.
    Unwrap(Operand),
    /// Whether an optional is `None`: a `Bool`.
    IsNone(Operand),
    /// Float arithmetic with Python semantics: `/`, `//` and `%` by zero
    /// and `**` out of range fail.
    Float(FloatOp, Operand, Operand),
    /// Checked int `**`: overflow and a negative exponent fail.
    IntPow(Operand, Operand),
    /// An `Int` or `Bool` as a `Float`.
    ToFloat(Operand),
    /// A `Bool` as an `Int`: 0 or 1.
    BoolToInt(Operand),
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

impl Program {
    /// What list, tuple or optional type `ty` holds; `None` for other
    /// types.
    pub fn compound(&self, ty: Ty) -> Option<&Compound> {
        match ty {
            Ty::List(id) | Ty::Tuple(id) | Ty::Opt(id) => self.types.get(id.0),
            _ => None,
        }
    }
}
