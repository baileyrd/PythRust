//! The substitution: type-variable bindings and unification.

use std::collections::{BTreeSet, HashMap};

use super::ty::{FuncType, ParamType, Type, TypeVar};

/// Why two types failed to unify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Clash {
    Mismatch,
    /// Binding would make a type contain itself (`xs.append(xs)`).
    Infinite,
}

/// Bindings for every variable created so far. A variable is bound at
/// most once; `resolve` follows chains.
#[derive(Debug, Default, Clone)]
pub(super) struct Subst {
    bindings: Vec<Option<Type>>,
}

impl Subst {
    pub(super) fn fresh(&mut self) -> Type {
        self.bindings.push(None);
        Type::Var(TypeVar(self.bindings.len() - 1))
    }

    /// Follows variable bindings until the head is known or unbound.
    pub(super) fn resolve(&self, ty: &Type) -> Type {
        let mut ty = ty.clone();
        while let Type::Var(v) = ty {
            match self.binding(v) {
                Some(bound) => ty = bound.clone(),
                None => return ty,
            }
        }
        ty
    }

    /// Resolves every variable, deeply.
    pub(super) fn zonk(&self, ty: &Type) -> Type {
        match self.resolve(ty) {
            Type::List(elem) => Type::List(Box::new(self.zonk(&elem))),
            Type::Dict(k, v) => Type::Dict(Box::new(self.zonk(&k)), Box::new(self.zonk(&v))),
            Type::Set(elem) => Type::Set(Box::new(self.zonk(&elem))),
            Type::View(view, item) => Type::View(view, Box::new(self.zonk(&item))),
            Type::Optional(inner) => Type::Optional(Box::new(self.zonk(&inner))),
            Type::Future(r) => Type::Future(Box::new(self.zonk(&r))),
            Type::Results(r) => Type::Results(Box::new(self.zonk(&r))),
            Type::Tuple(elems) => Type::Tuple(elems.iter().map(|t| self.zonk(t)).collect()),
            other => other,
        }
    }

    pub(super) fn zonk_func(&self, ft: &FuncType) -> FuncType {
        map_func(ft, |t| self.zonk(t))
    }

    pub(super) fn unify(&mut self, a: &Type, b: &Type) -> Result<(), Clash> {
        let (a, b) = (self.resolve(a), self.resolve(b));
        match (&a, &b) {
            (Type::Var(x), Type::Var(y)) if x == y => Ok(()),
            (Type::Var(v), other) | (other, Type::Var(v)) => self.bind(*v, other),
            (Type::List(x), Type::List(y))
            | (Type::Set(x), Type::Set(y))
            | (Type::Optional(x), Type::Optional(y))
            | (Type::Future(x), Type::Future(y))
            | (Type::Results(x), Type::Results(y)) => self.unify(x, y),
            (Type::View(v, x), Type::View(w, y)) if v == w => self.unify(x, y),
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                self.unify(k1, k2)?;
                self.unify(v1, v2)
            }
            (Type::Tuple(xs), Type::Tuple(ys)) if xs.len() == ys.len() => {
                xs.iter().zip(ys).try_for_each(|(x, y)| self.unify(x, y))
            }
            _ if a == b => Ok(()),
            _ => Err(Clash::Mismatch),
        }
    }

    /// Like `unify`, but a value may flow into a wider target: `T` and
    /// `None` into `T | None`, and tuples element-wise (they are
    /// immutable). Lists, dicts and sets stay invariant, since they are
    /// mutable.
    ///
    /// A value still unknown under a `T | None` target could be `T`,
    /// `None` or `T | None`, so that pair is returned unsolved rather than
    /// guessed; the caller defers it until the value is known.
    pub(super) fn assign(
        &mut self,
        target: &Type,
        value: &Type,
    ) -> Result<Vec<(Type, Type)>, Clash> {
        match (self.resolve(target), self.resolve(value)) {
            (t @ Type::Optional(_), v @ Type::Var(_)) => Ok(vec![(t, v)]),
            (Type::Optional(_), Type::None) => Ok(vec![]),
            (Type::Optional(inner), v) if !matches!(v, Type::Optional(_)) => {
                self.assign(&inner, &v)
            }
            (Type::Tuple(ts), Type::Tuple(vs)) if ts.len() == vs.len() => {
                let mut deferred = Vec::new();
                for (t, v) in ts.iter().zip(&vs) {
                    deferred.extend(self.assign(t, v)?);
                }
                Ok(deferred)
            }
            (t, v) => self.unify(&t, &v).map(|()| vec![]),
        }
    }

    /// Adds the unbound variables of `ty` to `out`.
    pub(super) fn free_vars(&self, ty: &Type, out: &mut BTreeSet<TypeVar>) {
        match self.resolve(ty) {
            Type::Var(v) => {
                out.insert(v);
            }
            Type::List(elem)
            | Type::Set(elem)
            | Type::Optional(elem)
            | Type::View(_, elem)
            | Type::Future(elem)
            | Type::Results(elem) => self.free_vars(&elem, out),
            Type::Dict(k, v) => {
                self.free_vars(&k, out);
                self.free_vars(&v, out);
            }
            Type::Tuple(elems) => elems.iter().for_each(|t| self.free_vars(t, out)),
            _ => {}
        }
    }

    /// Replaces the variables in `map`, leaving all others shared.
    pub(super) fn rename(&self, ty: &Type, map: &HashMap<TypeVar, Type>) -> Type {
        match self.resolve(ty) {
            Type::Var(v) => map.get(&v).cloned().unwrap_or(Type::Var(v)),
            Type::List(elem) => Type::List(Box::new(self.rename(&elem, map))),
            Type::Dict(k, v) => Type::Dict(
                Box::new(self.rename(&k, map)),
                Box::new(self.rename(&v, map)),
            ),
            Type::Set(elem) => Type::Set(Box::new(self.rename(&elem, map))),
            Type::View(view, item) => Type::View(view, Box::new(self.rename(&item, map))),
            Type::Optional(inner) => Type::Optional(Box::new(self.rename(&inner, map))),
            Type::Future(r) => Type::Future(Box::new(self.rename(&r, map))),
            Type::Results(r) => Type::Results(Box::new(self.rename(&r, map))),
            Type::Tuple(elems) => Type::Tuple(elems.iter().map(|t| self.rename(t, map)).collect()),
            other => other,
        }
    }

    fn binding(&self, v: TypeVar) -> Option<&Type> {
        self.bindings.get(v.0)?.as_ref()
    }

    fn bind(&mut self, v: TypeVar, ty: &Type) -> Result<(), Clash> {
        let mut free = BTreeSet::new();
        self.free_vars(ty, &mut free);
        if free.contains(&v) {
            return Err(Clash::Infinite);
        }
        match self.bindings.get_mut(v.0) {
            Some(slot) => {
                *slot = Some(ty.clone());
                Ok(())
            }
            None => Err(Clash::Mismatch),
        }
    }
}

/// Applies `f` to every type in a signature.
pub(super) fn map_func(ft: &FuncType, f: impl Fn(&Type) -> Type) -> FuncType {
    let params = ft
        .params
        .iter()
        .map(|p| ParamType {
            name: p.name.clone(),
            ty: f(&p.ty),
            has_default: p.has_default,
        })
        .collect();
    FuncType {
        params,
        ret: f(&ft.ret),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(t: Type) -> Type {
        Type::List(Box::new(t))
    }

    #[test]
    fn unify_binds_through_structure() {
        let mut s = Subst::default();
        let a = s.fresh();
        assert_eq!(s.unify(&list(a.clone()), &list(Type::Int)), Ok(()));
        assert_eq!(s.zonk(&a), Type::Int);
    }

    #[test]
    fn unify_follows_chains() {
        let mut s = Subst::default();
        let (a, b) = (s.fresh(), s.fresh());
        assert_eq!(s.unify(&a, &b), Ok(()));
        assert_eq!(s.unify(&b, &Type::Str), Ok(()));
        assert_eq!(s.zonk(&a), Type::Str);
    }

    #[test]
    fn unify_rejects_mismatch_and_tuple_arity() {
        let mut s = Subst::default();
        assert_eq!(s.unify(&Type::Int, &Type::Float), Err(Clash::Mismatch));
        let one = Type::Tuple(vec![Type::Int]);
        let two = Type::Tuple(vec![Type::Int, Type::Int]);
        assert_eq!(s.unify(&one, &two), Err(Clash::Mismatch));
    }

    #[test]
    fn unify_rejects_infinite_type() {
        let mut s = Subst::default();
        let a = s.fresh();
        assert_eq!(s.unify(&a, &list(a.clone())), Err(Clash::Infinite));
    }

    #[test]
    fn assign_widens_into_optional_but_not_back() {
        let mut s = Subst::default();
        let opt = Type::Optional(Box::new(Type::Int));
        assert_eq!(s.assign(&opt, &Type::None), Ok(vec![]));
        assert_eq!(s.assign(&opt, &Type::Int), Ok(vec![]));
        assert_eq!(s.assign(&opt, &Type::Str), Err(Clash::Mismatch));
        assert_eq!(s.assign(&Type::Int, &opt), Err(Clash::Mismatch));
        let pair = |t: Type| Type::Tuple(vec![t, Type::Str]);
        assert_eq!(s.assign(&pair(opt.clone()), &pair(Type::None)), Ok(vec![]));
        assert_eq!(
            s.assign(&list(opt), &list(Type::None)),
            Err(Clash::Mismatch)
        );
    }

    #[test]
    fn assign_defers_an_unknown_value_under_optional() {
        let mut s = Subst::default();
        let opt = Type::Optional(Box::new(Type::Int));
        let a = s.fresh();
        let pair = |t: Type| Type::Tuple(vec![t, Type::Str]);
        let deferred = s.assign(&pair(opt.clone()), &pair(a.clone()));
        assert_eq!(deferred, Ok(vec![(opt, a.clone())]));
        assert_eq!(s.resolve(&a), a);
    }

    #[test]
    fn rename_replaces_only_mapped_vars() {
        let mut s = Subst::default();
        let (a, b) = (s.fresh(), s.fresh());
        let Type::Var(va) = a else { panic!() };
        let map = HashMap::from([(va, Type::Int)]);
        let renamed = s.rename(&Type::Tuple(vec![a, b.clone()]), &map);
        assert_eq!(renamed, Type::Tuple(vec![Type::Int, b]));
    }
}
