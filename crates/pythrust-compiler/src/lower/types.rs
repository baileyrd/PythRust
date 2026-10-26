//! List and tuple types, interned so that [`Ty`] stays `Copy`.

use std::collections::HashMap;

use crate::ir::{Compound, Ty, TypeId};

/// Every list and tuple type of the program, each once.
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

    fn intern(&mut self, c: Compound) -> TypeId {
        if let Some(id) = self.ids.get(&c) {
            return *id;
        }
        let id = TypeId(self.all.len());
        self.all.push(c.clone());
        self.ids.insert(c, id);
        id
    }

    /// A list's element type.
    pub(super) fn elem(&self, ty: Ty) -> Option<Ty> {
        match self.compound(ty)? {
            Compound::List(elem) => Some(*elem),
            Compound::Tuple(_) => None,
        }
    }

    /// A tuple's field types.
    pub(super) fn fields(&self, ty: Ty) -> Option<&[Ty]> {
        match self.compound(ty)? {
            Compound::Tuple(fields) => Some(fields),
            Compound::List(_) => None,
        }
    }

    fn compound(&self, ty: Ty) -> Option<&Compound> {
        match ty {
            Ty::List(id) | Ty::Tuple(id) => self.all.get(id.0),
            _ => None,
        }
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
}
