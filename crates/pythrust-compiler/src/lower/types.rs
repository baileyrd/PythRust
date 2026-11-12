//! List, tuple, optional, dict, set and view types, interned so that
//! [`Ty`] stays `Copy`.

use std::collections::HashMap;

use crate::infer::Type;
use crate::ir::{Compound, Ty, TypeId, View};

/// Every list, tuple and optional type of the program, each once.
#[derive(Default)]
pub(super) struct SeqTypes {
    all: Vec<Compound>,
    ids: HashMap<Compound, TypeId>,
    /// Whether a future is a [`Ty::Future`], as `build` runs tasks, or
    /// its task's result, as analysis does ([`super::tasks`]).
    pub(super) futures: bool,
    /// Every record's fields with their declared types, by its qualified
    /// name ([`crate::infer::Types::record`]).
    records: HashMap<String, Vec<(String, Type)>>,
    /// Each record type made so far, by its qualified name: records are
    /// nominal, and one may reach itself through its fields (D162).
    named: HashMap<String, TypeId>,
    /// The records of [`Self::records`] that are mutable classes, whose
    /// values are objects (D150).
    mutable: HashMap<String, bool>,
}

impl SeqTypes {
    /// No types yet; `futures` as [`SeqTypes::futures`]; `records` the
    /// program's record and class declarations, `mutable` its classes'.
    pub(super) fn new(
        futures: bool,
        records: HashMap<String, Vec<(String, Type)>>,
        mutable: HashMap<String, bool>,
    ) -> Self {
        SeqTypes {
            futures,
            records,
            mutable,
            ..SeqTypes::default()
        }
    }

    /// Whether `name` is a mutable class, whose values are objects.
    pub(super) fn is_class(&self, name: &str) -> bool {
        self.mutable.contains_key(name)
    }

    /// The declared fields of record `name`.
    pub(super) fn declared(&self, name: &str) -> Option<&[(String, Type)]> {
        self.records.get(name).map(Vec::as_slice)
    }

    /// Record or class `name`'s type if it has been made.
    pub(super) fn named(&self, name: &str) -> Option<Ty> {
        let id = self.named.get(name).copied()?;
        Some(self.nominal(name, id))
    }

    /// The type `id` of record or class `name`.
    fn nominal(&self, name: &str, id: TypeId) -> Ty {
        match self.is_class(name) {
            true => Ty::Obj(id),
            false => Ty::Record(id),
        }
    }

    /// Record `name`'s type, its fields to come from [`Self::fill`], so
    /// that a field's type may name it.
    pub(super) fn reserve(&mut self, name: &str) -> Ty {
        let names = self
            .declared(name)
            .map(|d| d.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        let shown = crate::ast::source_name(crate::ast::unqualified(name)).to_string();
        let id = TypeId(self.all.len());
        let name = name.to_string();
        self.all.push(match self.is_class(&name) {
            true => Compound::Object {
                name: name.clone(),
                shown,
                plain: self.mutable.get(&name).copied().unwrap_or(false),
                names,
                fields: Vec::new(),
            },
            false => Compound::Record {
                name: name.clone(),
                shown,
                names,
                fields: Vec::new(),
            },
        });
        self.named.insert(name.clone(), id);
        self.nominal(&name, id)
    }

    /// The field types of `record`, a type [`Self::reserve`] made.
    pub(super) fn fill(&mut self, record: Ty, types: Vec<Ty>) {
        let (Ty::Record(id) | Ty::Obj(id)) = record else {
            return;
        };
        if let Some(Compound::Record { fields, .. } | Compound::Object { fields, .. }) =
            self.all.get_mut(id.0)
        {
            *fields = types;
        }
    }

    /// The qualified name of record or class type `ty`.
    pub(super) fn record_name(&self, ty: Ty) -> Option<&str> {
        match self.compound(ty)? {
            Compound::Record { name, .. } | Compound::Object { name, .. } => Some(name),
            _ => None,
        }
    }

    /// The index of record or class type `ty`'s field `field`.
    pub(super) fn field_of(&self, ty: Ty, field: &str) -> Option<usize> {
        match self.compound(ty)? {
            Compound::Record { names, .. } | Compound::Object { names, .. } => {
                names.iter().position(|n| n == field)
            }
            _ => None,
        }
    }

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

    /// A tuple's or record's field types.
    pub(super) fn fields(&self, ty: Ty) -> Option<&[Ty]> {
        match self.compound(ty)? {
            Compound::Tuple(fields)
            | Compound::Record { fields, .. }
            | Compound::Object { fields, .. } => Some(fields),
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
        crate::ir::compound(&self.all, ty)
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
