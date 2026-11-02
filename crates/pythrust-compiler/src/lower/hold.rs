//! A `for` loop's hold on the list it walks (atomics cost A, Nano
//! 2026-11-02). Each `len` and read of a list borrows it, and once a
//! task may reach the list each borrow is two atomic read-modify-writes
//! on one word every reader shares (`docs/benchmarks.md`). A loop whose
//! statements cannot change the list takes one read borrow for the
//! whole loop instead ([`Ty::Held`]), and its reads skip the word.
//!
//! The hold is a local of its own, `h = Hold(s)`, which takes over the
//! loop's reference `s` just before the loop starts; the loop's reads
//! and releases of `s` become `h`'s. Code before that point (filling a
//! temporary the loop walks, which may raise) still sees a plain list.
//!
//! "Cannot change" is decided on the loop's lowered IR, conservatively:
//! no statement changes any list of the walked list's type (another name
//! for it has that type), and nothing runs code this function cannot
//! see (a call, a binding, a task). A loop that fails the test borrows
//! per read, as before. Only `build` holds: the analyses see the plain
//! list.

use crate::ir::{BlockId, Inst, LocalId, Operand, Rvalue, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

use super::{FnBuilder, Reach};

impl FnBuilder<'_, '_> {
    /// Moves the loop's own reference `s` to the list it walks into a
    /// new hold, `h = Hold(s)` then `Forget(s)`, just before the loop's
    /// `counter` starts in block `setup`, when no statement in the
    /// loop's blocks, `first` onwards, may change a list of its type and
    /// those blocks only read and release `s`. They and the open
    /// statement pools then name `h` instead.
    pub(super) fn hold_if_unchanged(
        &mut self,
        s: Operand,
        setup: BlockId,
        counter: LocalId,
        first: usize,
        span: Span,
    ) {
        if self.cx.reach != Reach::Called {
            return;
        }
        let Operand::Local(t) = s else {
            return;
        };
        let Ty::List(id) = self.locals[t.0] else {
            return;
        };
        let list = Ty::List(id);
        let mut insts = self.blocks[first..].iter().flat_map(|b| &b.insts);
        if insts.any(|i| self.may_change(i, list) || (i.mentions(t) && !renamable(i, t))) {
            return;
        }
        let starts = |i: &Inst| matches!(i, Inst::Assign { dest, .. } if *dest == counter);
        let Some(at) = self.blocks[setup.0].insts.iter().position(starts) else {
            return;
        };
        let h = self.new_local(Ty::Held(id));
        let hold = Inst::Assign {
            dest: h,
            value: Rvalue::Hold(s),
            span,
        };
        let block = &mut self.blocks[setup.0].insts;
        block.splice(at..at, [hold, Inst::Forget(t)]);
        let loop_insts = self.blocks[first..].iter_mut().flat_map(|b| &mut b.insts);
        loop_insts.for_each(|i| rename(i, t, h));
        let pooled = self.pools.iter_mut().flatten().filter(|l| **l == t);
        pooled.for_each(|l| *l = h);
    }

    /// Whether `inst` may change a list of type `list`: it changes one
    /// in place, or runs code this function does not show.
    fn may_change(&self, inst: &Inst, list: Ty) -> bool {
        match inst {
            Inst::Append { list: l, .. }
            | Inst::SetItem { list: l, .. }
            | Inst::Sort(l)
            | Inst::Reverse(l) => self.ty(*l) == list,
            Inst::Assign { value, .. } => runs_unseen(value),
            Inst::Print { .. }
            | Inst::Retain(_)
            | Inst::Release(_)
            | Inst::Forget(_)
            | Inst::SetGlobal { .. }
            | Inst::DropGlobal(_)
            | Inst::Raise { .. } => false,
        }
    }
}

/// Whether `value` runs code the function does not show, which may
/// change any list: a call, a Rust binding, a task, or a library
/// function that changes a list.
fn runs_unseen(value: &Rvalue) -> bool {
    match value {
        Rvalue::Call(..)
        | Rvalue::Rust(..)
        | Rvalue::Submit { .. }
        | Rvalue::TaskResult(_)
        | Rvalue::NextResult(_) => true,
        Rvalue::Std(f, _) => std_changes_lists(*f),
        Rvalue::Use(_)
        | Rvalue::Int(..)
        | Rvalue::Neg(_)
        | Rvalue::Cmp(..)
        | Rvalue::AddOverflows(..)
        | Rvalue::StrLit(_)
        | Rvalue::Concat(..)
        | Rvalue::Repeat(..)
        | Rvalue::Index(..)
        | Rvalue::Len(_)
        | Rvalue::Entry(..)
        | Rvalue::View(_)
        | Rvalue::Contains { .. }
        | Rvalue::ToStr(_)
        | Rvalue::CopyList(_)
        | Rvalue::NewSeq(_)
        | Rvalue::Field(..)
        | Rvalue::Global(_)
        | Rvalue::Wrap(_)
        | Rvalue::Unwrap(_)
        | Rvalue::IsNone(_)
        | Rvalue::Float(..)
        | Rvalue::IntPow(..)
        | Rvalue::ToFloat(_)
        | Rvalue::BoolToInt(_)
        | Rvalue::FloatToInt(_)
        | Rvalue::ParseFloat(_)
        | Rvalue::ParseInt(_)
        | Rvalue::NewExc(..)
        | Rvalue::Raised
        | Rvalue::PendingIs(_)
        | Rvalue::Catch
        | Rvalue::Hold(_) => false,
    }
}

/// Whether library function `f` changes a list it is given: none does
/// yet; each new one must say.
fn std_changes_lists(f: StdFn) -> bool {
    match f {
        StdFn::Argv
        | StdFn::Listdir
        | StdFn::Join
        | StdFn::Isdir
        | StdFn::Isfile
        | StdFn::Islink
        | StdFn::Getsize
        | StdFn::Open
        | StdFn::Read
        | StdFn::Close
        | StdFn::Sha256
        | StdFn::Executor => false,
    }
}

/// Whether `inst`, which names `t`, only reads the loop's list or
/// releases it: what the loop itself does with its reference.
fn renamable(inst: &Inst, t: LocalId) -> bool {
    let list = Operand::Local(t);
    match inst {
        Inst::Assign { dest, value, .. } if *dest != t => match value {
            Rvalue::Len(s) => *s == list,
            Rvalue::Index(s, i) => *s == list && *i != list,
            _ => false,
        },
        Inst::Release(l) => *l == t,
        _ => false,
    }
}

/// Renames the loop's list `t` to its hold `h` in `inst`, one of the
/// [`renamable`] forms.
fn rename(inst: &mut Inst, t: LocalId, h: LocalId) {
    let (from, to) = (Operand::Local(t), Operand::Local(h));
    match inst {
        Inst::Assign {
            value: Rvalue::Len(s) | Rvalue::Index(s, _),
            ..
        } if *s == from => *s = to,
        Inst::Release(l) if *l == t => *l = h,
        _ => {}
    }
}
