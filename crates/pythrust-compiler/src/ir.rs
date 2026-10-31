//! Mid-level IR between the typed AST and a backend. Contract type.
//!
//! A control-flow graph of basic blocks over typed, monomorphic locals;
//! not SSA, locals are mutable slots (`docs/research/ir.md`). Checks that
//! can fail at run time are explicit: a failing `Rvalue` carries the span
//! of its `Inst`, and a `Term::Fail` ends the program with a diagnostic.
//!
//! Covers ints, floats, bools, `None`, strings, lists, tuples,
//! optionals (`T | None`), exceptions, dicts, sets and dict views.
//! List, tuple, optional, dict, set and view types are interned
//! in [`Program::types`], so [`Ty`] stays `Copy`.
//!
//! Locals other than parameters start zeroed. A local of a
//! reference-counted type ([`Ty::is_rc`]: strings, lists, tuples) is a
//! pointer and, at every instruction boundary, either null or the owner
//! of one reference: lowering makes ownership explicit with `Retain`,
//! `Release` and `Forget`, so the backend never decides it. Such
//! operands are borrowed; calls take them owned, but for a parameter in
//! [`Func::borrowed`], and return an owned result, and rvalues giving
//! one give a new reference.
//!
//! After `rc::elide` two kinds of local hold a pointer without owning
//! it. A borrowed parameter points at its caller's reference, which
//! outlives the call; the function never releases, moves or reassigns
//! it. A copy `a = x` whose `Retain` was cancelled borrows `x`'s
//! reference until the `Forget(a)`s that replaced its `Release`s; it is
//! read only while `x` still holds that reference, and is null again
//! after its `Forget`. Neither is ever released. (A copy that took `x`'s
//! reference at `x`'s last use, `Forget(x)` in place of its `Retain`,
//! owns it as any other local does.)
//!
//! Exceptions (D6, D8) are a tag plus a value without a second return
//! register: [`Inst::Raise`] makes an exception the runtime's one
//! *pending* exception, and the code after anything that can raise (a
//! call to a function whose `raises` set is not empty or that may fault
//! catchably, `int(s)`, `float(s)`, a division, an index) tests
//! [`Rvalue::Raised`] and branches to a handler or unwinds. A function unwinds by releasing its references and
//! returning, its result ignored, with the exception still pending; the
//! module's code ends with [`Term::Uncaught`]. A handler tests the class
//! with [`Rvalue::PendingIs`] and takes the exception with
//! [`Rvalue::Catch`]. The catchable faults (division by zero, an index
//! out of range, unpacking arity) raise this way; the others (overflow,
//! recursion depth, out-of-memory) stay [`Term::Fail`]s and failing
//! rvalues.

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Diagnostic;
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

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
    /// list that may hold an exception (`codegen`); the analyses see it.
    Exc,
    /// A mutable, reference-counted dict in insertion order; `types[id]`
    /// is a [`Compound::Dict`].
    Dict(TypeId),
    /// A mutable, reference-counted set in insertion order; `types[id]`
    /// is a [`Compound::Set`].
    Set(TypeId),
    /// An immutable, reference-counted snapshot of a dict's keys, values
    /// or items; `types[id]` is a [`Compound::View`]. It indexes like a
    /// list, for `for` (the language has no view subscripts).
    View(TypeId),
}

impl Ty {
    /// Whether values of this type are reference-counted pointers.
    pub fn is_rc(self) -> bool {
        matches!(
            self,
            Ty::Str
                | Ty::List(_)
                | Ty::Tuple(_)
                | Ty::Opt(_)
                | Ty::Range
                | Ty::Exc
                | Ty::Dict(_)
                | Ty::Set(_)
                | Ty::View(_)
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
    /// Key and value types.
    Dict(Ty, Ty),
    Set(Ty),
    /// Which view, and its item type (a key, a value, or a `(key,
    /// value)` tuple).
    View(View, Ty),
}

/// What a dict view holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Keys,
    Values,
    Items,
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
    /// The parameters the caller keeps its reference to for the call:
    /// the function never drops them. Lowering borrows none;
    /// `rc::elide` decides.
    pub borrowed: BTreeSet<LocalId>,
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
    /// `list[index] = value`; raises `IndexError` at `span`, changing
    /// nothing, when out of range. On a dict, `dict[key] = value`, which
    /// adds or replaces and never raises. On a set, `Append` adds.
    SetItem {
        list: Operand,
        index: Operand,
        value: Operand,
        span: Span,
    },
    /// Sorts a list in place by `<`, stably.
    Sort(Operand),
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
    /// Checked int arithmetic: overflow fails; `//` and `%` by zero raise
    /// `ZeroDivisionError` as [`Rvalue::Float`] does.
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
    /// A module function's result (M6a), a new value of the
    /// destination's type; the operands are its arguments, borrowed.
    Std(StdFn, Vec<Operand>),
    /// `a + b` on two `Str`s.
    Concat(Operand, Operand),
    /// `s * n`, a `Str` and an `Int`. Fails when too large to allocate.
    Repeat(Operand, Operand),
    /// `s[i]` on a `Str` (a one-char `Str`) or a list or view (an
    /// element). Raises `IndexError` when `i` is out of range, giving
    /// null or 0. On a dict, the value of key `i`, which lowering proved
    /// present.
    Index(Operand, Operand),
    /// `len(s)` of a `Str` (in chars), list, tuple, range, dict, set or
    /// view: an `Int`. For a range whose length does not fit in one, -1.
    Len(Operand),
    /// Part `k` (0 the key, 1 the value) of entry `i` of a dict or set,
    /// in insertion order; lowering keeps `i` below the length.
    Entry(Operand, Operand, usize),
    /// A new view of a dict, of the destination's type.
    View(Operand),
    /// `needle in hay`: a substring of a `Str`, an element of a list or
    /// view, or a key of a dict or set. Gives a `Bool`.
    Contains {
        hay: Operand,
        needle: Operand,
    },
    /// `str(x)` for anything but a `Str`.
    ToStr(Operand),
    /// `list(xs)`: a new list holding a list's elements (a shallow copy),
    /// or a dict's keys, a set's elements or a view's items, of the
    /// destination's type.
    CopyList(Operand),
    /// A new list, tuple, set or dict, of the destination's type,
    /// holding `items`: for a dict, keys and values alternating.
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
    /// raise `ZeroDivisionError` (pending, at the `Inst`'s span) and give
    /// a placeholder; `**` out of range fails.
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
    /// Whether the pending exception, which there is, is one of these
    /// classes: of one or of a class derived from one, as `except` tests.
    /// Never empty.
    PendingIs(Vec<ExcKind>),
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
    /// `/` of two ints, computed in floats: `Div`, but dividing by zero
    /// says so as Python's int division does.
    IntDiv,
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
        Ty::List(id) | Ty::Tuple(id) | Ty::Opt(id) | Ty::Dict(id) | Ty::Set(id) | Ty::View(id) => {
            types.get(id.0)
        }
        _ => None,
    };
    match inner {
        Some(Compound::List(t) | Compound::Opt(t) | Compound::Set(t) | Compound::View(_, t)) => {
            holds(types, *t, leaf)
        }
        Some(Compound::Dict(k, v)) => holds(types, *k, leaf) || holds(types, *v, leaf),
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
    /// What list, tuple, optional, dict, set or view type `ty` holds;
    /// `None` for other types.
    pub fn compound(&self, ty: Ty) -> Option<&Compound> {
        match ty {
            Ty::List(id)
            | Ty::Tuple(id)
            | Ty::Opt(id)
            | Ty::Dict(id)
            | Ty::Set(id)
            | Ty::View(id) => self.types.get(id.0),
            _ => None,
        }
    }
}
