//! Mid-level IR between the typed AST and a backend. Contract type.
//!
//! A control-flow graph of basic blocks over typed, monomorphic locals;
//! not SSA, locals are mutable slots (`docs/research/ir.md`). Checks that
//! can fail at run time are explicit: a failing `Rvalue` carries the span
//! of its `Inst`, and a `Term::Fail` ends the program with a diagnostic.
//!
//! Covers ints, floats, bools, `None`, strings, lists, tuples,
//! optionals (`T | None`) and exceptions. List, tuple and optional types are interned
//! in [`Program::types`], so [`Ty`] stays `Copy`.
//!
//! Locals other than parameters start zeroed. A local of a
//! reference-counted type ([`Ty::is_rc`]: strings, lists, tuples) is a
//! pointer and, at every instruction boundary, either null or the owner
//! of one reference: lowering makes ownership explicit with `Retain`,
//! `Release` and `Forget`, so the backend never decides it. Such
//! operands are borrowed; calls take them owned and return an owned
//! result, and rvalues giving one give a new reference.
//!
//! Exceptions (D6, D8) are a tag plus a value without a second return
//! register: [`Inst::Raise`] makes an exception the runtime's one
//! *pending* exception, and the code after anything that can raise (a
//! call to a function whose `raises` set is not empty, `int(s)`,
//! `float(s)`) tests [`Rvalue::Raised`] and branches to a handler or
//! unwinds. A function unwinds by releasing its references and
//! returning, its result ignored, with the exception still pending; the
//! module's code ends with [`Term::Uncaught`]. A handler tests the kind
//! with [`Rvalue::PendingIs`] and takes the exception with
//! [`Rvalue::Catch`]. Faults (D7) stay [`Term::Fail`]s.

use std::collections::BTreeMap;

use crate::diag::Diagnostic;
use crate::span::Span;

/// A value type. Every local and operand has exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// An immutable, reference-counted exception: its [`ExcKind`] and
    /// at most one argument. The native backend rejects one holding a
    /// list (`codegen`); the analyses see it.
    Exc,
}

impl Ty {
    /// Whether values of this type are reference-counted pointers.
    pub fn is_rc(self) -> bool {
        matches!(
            self,
            Ty::Str | Ty::List(_) | Ty::Tuple(_) | Ty::Opt(_) | Ty::Range | Ty::Exc
        )
    }
}

/// Index into [`Program::types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub usize);

/// What a list, tuple or optional type holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Compound {
    List(Ty),
    Tuple(Vec<Ty>),
    /// The present type: never `None` or itself optional.
    Opt(Ty),
}

/// A builtin exception (D6). The runtime knows each by its index in
/// [`ExcKind::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExcKind {
    Value,
    Index,
    ZeroDivision,
    Overflow,
    Recursion,
}

impl ExcKind {
    pub const ALL: [ExcKind; 5] = [
        ExcKind::Value,
        ExcKind::Index,
        ExcKind::ZeroDivision,
        ExcKind::Overflow,
        ExcKind::Recursion,
    ];

    /// The builtin exception named `name`.
    pub fn from_name(name: &str) -> Option<ExcKind> {
        ExcKind::ALL.into_iter().find(|k| k.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            ExcKind::Value => "ValueError",
            ExcKind::Index => "IndexError",
            ExcKind::ZeroDivision => "ZeroDivisionError",
            ExcKind::Overflow => "OverflowError",
            ExcKind::Recursion => "RecursionError",
        }
    }

    /// The runtime's code for this kind.
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Index into [`Program::funcs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub usize);

/// Index into [`Program::globals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalId(pub usize);

/// Index into [`Func::locals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// The globals that hold module variables, by the variable's name;
    /// for diagnostics.
    pub global_names: BTreeMap<GlobalId, String>,
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
    /// The locals that are source variables, parameters included, by
    /// name; the rest are temporaries. For diagnostics about the body.
    pub names: BTreeMap<LocalId, String>,
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
    /// `list.append(value)` at `span`: the list adds its own reference.
    Append {
        list: Operand,
        value: Operand,
        span: Span,
    },
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
    /// `raise exc`: makes the `Exc` pending, with its own reference, as
    /// raised at `span`. Nothing may be pending.
    Raise { exc: Operand, span: Span },
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
    /// `float(s)`. Unless `s` is a float literal, raises `ValueError`
    /// (pending, at the `Inst`'s span) and gives 0.
    ParseFloat(Operand),
    /// `int(s)` in base 10. Unless `s` is an int literal, raises
    /// `ValueError` as [`Rvalue::ParseFloat`] does; fails out of range.
    ParseInt(Operand),
    /// A new exception of this kind holding the operand, if any, which
    /// gains a reference.
    NewExc(ExcKind, Option<Operand>),
    /// Whether an exception is pending: a `Bool`.
    Raised,
    /// Whether the pending exception, which there is, is of this kind.
    PendingIs(ExcKind),
    /// The pending exception, which there is, with its reference; none
    /// is pending after.
    Catch,
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
    /// The pending exception escaped the module: print it as a runtime
    /// error at the span it was raised at (`ValueError: bad`) and exit 1.
    Uncaught,
    /// Proven unreachable, such as the end of a function that always
    /// returns.
    Unreachable,
}

/// Whether a value of type `ty` is, or may hold, a value of a type
/// `leaf` picks; `types` are [`Program::types`].
pub fn holds(types: &[Compound], ty: Ty, leaf: fn(Ty) -> bool) -> bool {
    if leaf(ty) {
        return true;
    }
    let inner = match ty {
        Ty::List(id) | Ty::Tuple(id) | Ty::Opt(id) => types.get(id.0),
        _ => None,
    };
    match inner {
        Some(Compound::List(t) | Compound::Opt(t)) => holds(types, *t, leaf),
        Some(Compound::Tuple(fields)) => fields.iter().any(|t| holds(types, *t, leaf)),
        None => false,
    }
}

/// Whether `ty` is an exception: with [`holds`], whether rendering a
/// value can nest without bound.
pub fn is_exc(ty: Ty) -> bool {
    ty == Ty::Exc
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
