//! List, tuple, optional, dict, set and view types, interned so that
//! [`Ty`] stays `Copy`.

use std::collections::HashMap;

use crate::ir::{Compound, Ty, TypeId, View};

/// Every list, tuple and optional type of the program, each once.
#[derive(Default)]
pub(super) struct SeqTypes {
    all: Vec<Compound>,
    ids: HashMap<Compound, TypeId>,
}

impl SeqTypes {
    pub(super) fn list(&mut self, elem: Ty) -> Ty {
        Ty::List(self.intern(Compound::List(elem)))
    }

    pub(super) fn tuple(&mut self, fields: Vec<Ty>) -> Ty {
        Ty::Tuple(self.intern(Compound::Tuple(fields)))
    }

    pub(super) fn dict(&mut self, key: Ty, value: Ty) -> Ty {
        Ty::Dict(self.intern(Compound::Dict(key, value)))
    }

    pub(super) fn set(&mut self, elem: Ty) -> Ty {
        Ty::Set(self.intern(Compound::Set(elem)))
    }

    pub(super) fn view(&mut self, view: View, item: Ty) -> Ty {
        Ty::View(self.intern(Compound::View(view, item)))
    }

    /// `present | None`: `None` itself when `present` is `None`, and
    /// `present` when already optional.
    pub(super) fn optional(&mut self, present: Ty) -> Ty {
        match present {
            Ty::None | Ty::Opt(_) => present,
            _ => Ty::Opt(self.intern(Compound::Opt(present))),
        }
    }

    fn intern(&mut self, c: Compound) -> TypeId {
        if let Some(id) = self.ids.get(&c) {
            return *id;
        }
        let id = TypeId(self.all.len());
        self.all.push(c.clone());
        self.ids.insert(c, id);
        id
    }

    /// A list's or set's element type, or a view's item type.
    pub(super) fn elem(&self, ty: Ty) -> Option<Ty> {
        match self.compound(ty)? {
            Compound::List(elem) | Compound::Set(elem) | Compound::View(_, elem) => Some(*elem),
            _ => None,
        }
    }

    /// Which view a view type is.
    pub(super) fn view_kind(&self, ty: Ty) -> Option<View> {
        match self.compound(ty)? {
            Compound::View(view, _) => Some(*view),
            _ => None,
        }
    }

    /// A dict's key and value types.
    pub(super) fn key_value(&self, ty: Ty) -> Option<(Ty, Ty)> {
        match self.compound(ty)? {
            Compound::Dict(key, value) => Some((*key, *value)),
            _ => None,
        }
    }

    /// A tuple's field types.
    pub(super) fn fields(&self, ty: Ty) -> Option<&[Ty]> {
        match self.compound(ty)? {
            Compound::Tuple(fields) => Some(fields),
            _ => None,
        }
    }

    /// An optional's present type.
    pub(super) fn present(&self, ty: Ty) -> Option<Ty> {
        match self.compound(ty)? {
            Compound::Opt(present) => Some(*present),
            _ => None,
        }
    }

    fn compound(&self, ty: Ty) -> Option<&Compound> {
        match ty {
            Ty::List(id)
            | Ty::Tuple(id)
            | Ty::Opt(id)
            | Ty::Dict(id)
            | Ty::Set(id)
            | Ty::View(id) => self.all.get(id.0),
            _ => None,
        }
    }

    /// Every type so far, by [`TypeId`].
    pub(super) fn all(&self) -> &[Compound] {
        &self.all
    }

    pub(super) fn into_vec(self) -> Vec<Compound> {
        self.all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_types_share_an_id() {
        let mut t = SeqTypes::default();
        let a = t.list(Ty::Int);
        let b = t.tuple(vec![a, Ty::Str]);
        assert_eq!(t.list(Ty::Int), a);
        assert_ne!(t.list(Ty::Float), a);
        assert_eq!(t.elem(a), Some(Ty::Int));
        assert_eq!(t.fields(b), Some(&[a, Ty::Str][..]));
        assert_eq!(t.fields(a), None);
    }

    #[test]
    fn optionals_flatten_and_hold_their_present_type() {
        let mut t = SeqTypes::default();
        let o = t.optional(Ty::Int);
        assert_eq!(t.optional(o), o);
        assert_eq!(t.optional(Ty::None), Ty::None);
        assert_eq!(t.present(o), Some(Ty::Int));
        assert_eq!(t.present(Ty::Int), None);
        assert_eq!(t.elem(o), None);
    }
}
