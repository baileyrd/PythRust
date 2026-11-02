//! A `for` loop's hold on the list it walks (atomics cost A, Nano
//! 2026-11-02). Each `len` and read of a list borrows it, and once a
//! task may reach the list each borrow is two atomic read-modify-writes
//! on one word every reader shares (`docs/benchmarks.md`). A loop whose
//! statements cannot change the list takes one read borrow for the
//! whole loop instead ([`Ty::Held`]), and its reads skip the word.
//!
//! "Cannot change" is decided on the loop's lowered IR, conservatively:
//! no statement changes any list of the walked list's type (another name
//! for it has that type), and nothing runs code this function cannot
//! see (a call, a binding, a task). A loop that fails the test borrows
//! per read, as before. Only `build` holds: the analyses see the plain
//! list.

use crate::ir::{BlockId, Inst, LocalId, Operand, Rvalue, Ty};
use crate::span::Span;

use super::{FnBuilder, Reach};

impl FnBuilder<'_, '_> {
    /// Makes the loop's own reference `s` to the list it walks a hold,
    /// `s = Hold(s)` just before the loop's `counter` starts in block
    /// `setup`, when no statement in the loop's blocks, `first` onwards,
    /// may change a list of its type.
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
        if insts.any(|i| self.may_change(i, list)) {
            return;
        }
        let block = &mut self.blocks[setup.0].insts;
        let starts = |i: &Inst| matches!(i, Inst::Assign { dest, .. } if *dest == counter);
        let Some(at) = block.iter().position(starts) else {
            return;
        };
        let value = Rvalue::Hold(s);
        block.insert(
            at,
            Inst::Assign {
                dest: t,
                value,
                span,
            },
        );
        self.locals[t.0] = Ty::Held(id);
    }

    /// Whether `inst` may change a list of type `list`: it changes one
    /// in place, or runs code this function does not show.
    fn may_change(&self, inst: &Inst, list: Ty) -> bool {
        match inst {
            Inst::Append { list: l, .. } | Inst::SetItem { list: l, .. } => self.ty(*l) == list,
            Inst::Sort(l) | Inst::Reverse(l) => self.ty(*l) == list,
            Inst::Assign { value, .. } => matches!(
                value,
                Rvalue::Call(..)
                    | Rvalue::Rust(..)
                    | Rvalue::Submit { .. }
                    | Rvalue::TaskResult(_)
                    | Rvalue::NextResult(_)
            ),
            _ => false,
        }
    }
}
