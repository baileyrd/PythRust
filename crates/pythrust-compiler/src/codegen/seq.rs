//! Lists and tuples: runtime objects (`pythrust-rt/src/seq.rs`) whose
//! 64-bit slots a descriptor types. Values cross into slots here: a
//! float as its bits, a bool or `None` widened, a pointer as is.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, InstBuilder, MemFlagsData, Value};

use super::{backend, FnGen, Rt};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::{CmpOp, Compound, LocalId, Operand, Ty, View};
use crate::span::Span;

const LIST_INDEX: &str = "list index out of range";
const LIST_STORE: &str = "list assignment index out of range";
const RANGE_INDEX: &str = "range object index out of range";
const STR_INDEX: &str = "string index out of range";

impl FnGen<'_, '_, '_> {
    /// A new sequence of type `ty` holding `items`.
    pub(super) fn new_seq(&mut self, ty: Ty, items: &[Operand]) -> Result<Value> {
        let (desc, len) = self.descriptor(ty)?;
        let cap = self.b.ins().iconst(types::I64, items.len() as i64);
        let s = self.call_rt_value(Rt::SeqNew, &[desc, len, cap])?;
        if let Ty::Dict(_) = ty {
            for pair in items.chunks(2) {
                let slots: Vec<Value> = pair.iter().map(|op| self.slot(*op)).collect();
                self.call_rt(Rt::DictSet, &[&[s], &slots[..]].concat());
            }
            return Ok(s);
        }
        let push = match ty {
            Ty::Set(_) => Rt::SetAdd,
            _ => Rt::SeqPush,
        };
        for item in items {
            let v = self.slot(*item);
            self.call_rt(push, &[s, v]);
        }
        Ok(s)
    }

    /// The descriptor of `ty` as read-only data: its address and length.
    pub(super) fn descriptor(&mut self, ty: Ty) -> Result<(Value, Value)> {
        let mut text = String::new();
        descriptor(self.types, ty, &mut text)?;
        self.data(text)
    }

    /// `d[k]` of a dict holding `k`, into `dest`.
    pub(super) fn dict_get(&mut self, d: Operand, k: Operand, dest: LocalId) -> Result<Value> {
        let (d, k) = (self.operand(d), self.slot(k));
        let (v, _) = self.with_status(Rt::DictGet, &[d, k])?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// Part `part` of entry `i` of dict or set `t`, into `dest`.
    pub(super) fn entry(
        &mut self,
        t: Operand,
        i: Operand,
        part: usize,
        dest: LocalId,
    ) -> Result<Value> {
        let (t, i) = (self.operand(t), self.operand(i));
        let part = self.b.ins().iconst(types::I64, part as i64);
        let v = self.call_rt_value(Rt::TableEntry, &[t, i, part])?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// A new view of type `ty` of dict `d`.
    pub(super) fn view(&mut self, d: Operand, ty: Ty) -> Result<Value> {
        let d = self.operand(d);
        let (desc, len) = self.descriptor(ty)?;
        self.call_rt_value(Rt::ViewNew, &[d, desc, len])
    }

    /// `list(x)` of a dict, set or view: a new list of type `ty`.
    pub(super) fn list_of(&mut self, x: Operand, ty: Ty) -> Result<Value> {
        let x = self.operand(x);
        let (desc, len) = self.descriptor(ty)?;
        self.call_rt_value(Rt::ListOf, &[x, desc, len])
    }

    /// `op` as a slot.
    pub(super) fn slot(&mut self, op: Operand) -> Value {
        let v = self.operand(op);
        match self.func.ty(op) {
            Ty::Float => self.b.ins().bitcast(types::I64, MemFlagsData::new(), v),
            Ty::Bool | Ty::None => self.b.ins().uextend(types::I64, v),
            ty if self.cg.clif_ty(ty) != types::I64 => self.b.ins().uextend(types::I64, v),
            _ => v,
        }
    }

    /// A slot as a value of `ty`.
    pub(super) fn unslot(&mut self, v: Value, ty: Ty) -> Value {
        let want = self.cg.clif_ty(ty);
        match want {
            types::I64 => v,
            types::F64 => self.b.ins().bitcast(types::F64, MemFlagsData::new(), v),
            _ => self.b.ins().ireduce(want, v),
        }
    }

    /// `s[i]` on a list, into `dest`.
    pub(super) fn item(
        &mut self,
        s: Operand,
        i: Operand,
        dest: LocalId,
        span: Span,
    ) -> Result<Value> {
        let (s, i) = (self.operand(s), self.operand(i));
        let (v, status) = self.with_status(Rt::SeqGet, &[s, i])?;
        self.raise_on(status, ExcKind::Index, LIST_INDEX, span)?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// Field `k` of tuple `t`, into `dest`. Lowering checked the range.
    pub(super) fn field(&mut self, t: Operand, k: usize, dest: LocalId) -> Result<Value> {
        let t = self.operand(t);
        let k = self.b.ins().iconst(types::I64, k as i64);
        let (v, _) = self.with_status(Rt::SeqGet, &[t, k])?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// `r[i]` of range `r`.
    pub(super) fn range_item(&mut self, r: Operand, i: Operand, span: Span) -> Result<Value> {
        let (r, i) = (self.operand(r), self.operand(i));
        let (v, status) = self.with_status(Rt::RangeGet, &[r, i])?;
        self.raise_on(status, ExcKind::Index, RANGE_INDEX, span)?;
        Ok(v)
    }

    /// `s[i]` of string `s`: a new one-char string, or null when it
    /// raises.
    pub(super) fn str_item(&mut self, s: Operand, i: Operand, span: Span) -> Result<Value> {
        let (s, i) = (self.operand(s), self.operand(i));
        let c = self.call_rt_value(Rt::StrIndex, &[s, i])?;
        let null = self.b.ins().icmp_imm_s(IntCC::Equal, c, 0);
        self.raise_if(null, ExcKind::Index, STR_INDEX, span)?;
        Ok(c)
    }

    /// `x in r` of range `r` and a number `x`.
    pub(super) fn range_contains(&mut self, r: Operand, x: Operand) -> Result<Value> {
        let r = self.operand(r);
        let (rt, x) = match self.func.ty(x) {
            Ty::Float => (Rt::RangeContainsFloat, self.operand(x)),
            _ => (Rt::RangeContains, self.slot(x)),
        };
        self.call_rt_value(rt, &[r, x])
    }

    pub(super) fn set_item(
        &mut self,
        list: Operand,
        index: Operand,
        value: Operand,
        span: Span,
    ) -> Result<()> {
        let (s, i, v) = (self.operand(list), self.operand(index), self.slot(value));
        let status = self.call_rt_value(Rt::ListSet, &[s, i, v])?;
        self.raise_on(status, ExcKind::Index, LIST_STORE, span)
    }

    /// Raises `kind` holding `message` when a runtime call's status is 1.
    fn raise_on(&mut self, status: Value, kind: ExcKind, message: &str, span: Span) -> Result<()> {
        let hit = self.b.ins().icmp_imm_s(IntCC::Equal, status, 1);
        self.raise_if(hit, kind, message, span)
    }

    /// Lexicographic `a op b`; an unordered pair (a NaN decides) is false
    /// but for `!=`.
    pub(super) fn seq_cmp(&mut self, op: CmpOp, a: Operand, b: Operand) -> Result<Value> {
        let (a, b) = (self.operand(a), self.operand(b));
        if let CmpOp::Eq | CmpOp::Ne = op {
            let eq = self.call_rt_value(Rt::SeqEq, &[a, b])?;
            let cc = match op {
                CmpOp::Eq => IntCC::NotEqual,
                _ => IntCC::Equal,
            };
            return Ok(self.b.ins().icmp_imm_s(cc, eq, 0));
        }
        // -1, 0 or 1 ordered, 2 unordered.
        let order = self.call_rt_value(Rt::SeqCmp, &[a, b])?;
        let (cc, k) = match op {
            CmpOp::Lt => (IntCC::Equal, -1),
            CmpOp::Le => (IntCC::SignedLessThanOrEqual, 0),
            CmpOp::Gt => (IntCC::Equal, 1),
            _ => (IntCC::UnsignedLessThanOrEqual, 1),
        };
        Ok(self.b.ins().icmp_imm_s(cc, order, k))
    }
}

/// Appends the runtime descriptor of `ty`: see `pythrust-rt/src/seq.rs`.
pub(super) fn descriptor(types: &[Compound], ty: Ty, out: &mut String) -> Result<()> {
    let code = match ty {
        Ty::Int => 'i',
        Ty::Float => 'f',
        Ty::Bool => 'b',
        Ty::None => 'n',
        Ty::Str => 's',
        Ty::Range => 'r',
        Ty::Exc => 'x',
        Ty::Bytes | Ty::File => 'y',
        Ty::List(id) | Ty::Tuple(id) | Ty::Opt(id) | Ty::Dict(id) | Ty::Set(id) | Ty::View(id) => {
            let compound = types
                .get(id.0)
                .ok_or_else(|| backend("an unknown sequence type".to_string()))?;
            return compound_descriptor(types, compound, out);
        }
    };
    out.push(code);
    Ok(())
}

fn compound_descriptor(types: &[Compound], c: &Compound, out: &mut String) -> Result<()> {
    match c {
        Compound::List(elem) => {
            out.push('l');
            descriptor(types, *elem, out)
        }
        Compound::Tuple(fields) => {
            // One byte, and ASCII so the descriptor stays a `String`.
            let n = u8::try_from(fields.len())
                .ok()
                .filter(u8::is_ascii)
                .ok_or_else(|| backend("a tuple of over 127 fields".to_string()))?;
            out.push('t');
            out.push(char::from(n));
            fields.iter().try_for_each(|f| descriptor(types, *f, out))
        }
        Compound::Opt(present) => {
            out.push('o');
            descriptor(types, *present, out)
        }
        Compound::Dict(key, value) => {
            out.push('d');
            descriptor(types, *key, out)?;
            descriptor(types, *value, out)
        }
        Compound::Set(elem) => {
            out.push('e');
            descriptor(types, *elem, out)
        }
        Compound::View(view, item) => {
            out.push('v');
            out.push(match view {
                View::Keys => 'k',
                View::Values => 'v',
                View::Items => 'i',
            });
            descriptor(types, *item, out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::TypeId;

    #[test]
    fn descriptors_nest_lists_and_tuples() {
        let types = vec![
            Compound::List(Ty::Float),
            Compound::Tuple(vec![Ty::Str, Ty::List(TypeId(0)), Ty::Bool]),
            Compound::List(Ty::Tuple(TypeId(1))),
        ];
        let mut out = String::new();
        descriptor(&types, Ty::List(TypeId(2)), &mut out).expect("descriptor");
        assert_eq!(out, "lt\u{3}slfb");
    }

    /// `pythrust_rt::abi` builds the `list[str]` a Rust binding returns
    /// with this descriptor, fixed there (`LIST_STR`).
    #[test]
    fn a_str_list_is_ls_as_the_runtime_assumes() {
        let mut out = String::new();
        descriptor(&[Compound::List(Ty::Str)], Ty::List(TypeId(0)), &mut out).expect("descriptor");
        assert_eq!(out, "ls");
    }

    #[test]
    fn optional_descriptors_name_their_present_type() {
        let types = vec![Compound::Opt(Ty::Int), Compound::List(Ty::Opt(TypeId(0)))];
        let mut out = String::new();
        descriptor(&types, Ty::List(TypeId(1)), &mut out).expect("descriptor");
        assert_eq!(out, "loi");
    }

    #[test]
    fn tuples_past_a_byte_of_fields_are_refused() {
        let types = vec![Compound::Tuple(vec![Ty::Int; 128])];
        let mut out = String::new();
        assert!(descriptor(&types, Ty::Tuple(TypeId(0)), &mut out).is_err());
    }
}
