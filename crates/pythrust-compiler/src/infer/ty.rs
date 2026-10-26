//! Types as inference produces them.

use std::fmt;

/// An inference variable. Unresolved ones left in the output are generic
/// (or unconstrained, such as the element type of a never-filled `[]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub usize);

/// A phase-2 value type. Spelled as in Python annotations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Str,
    Bool,
    None,
    /// What `range(...)` returns: iterates, indexes and sizes as ints.
    Range,
    List(Box<Type>),
    /// Fixed length, positionally typed.
    Tuple(Vec<Type>),
    /// `T | None`. `T` is never `None` or itself optional.
    Optional(Box<Type>),
    Var(TypeVar),
}

/// One parameter of a function signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamType {
    pub name: String,
    pub ty: Type,
    pub has_default: bool,
}

/// A top-level function's signature. Free `Var`s are generic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<ParamType>,
    pub ret: Type,
}

impl Type {
    /// `int`, `float` and `bool` mix in arithmetic and comparison.
    pub(super) fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Bool)
    }

    /// `int` or `bool`: valid as an index or repeat count.
    pub(super) fn is_intlike(&self) -> bool {
        matches!(self, Type::Int | Type::Bool)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => f.write_str("int"),
            Type::Float => f.write_str("float"),
            Type::Str => f.write_str("str"),
            Type::Bool => f.write_str("bool"),
            Type::None => f.write_str("None"),
            Type::Range => f.write_str("range"),
            Type::List(elem) => write!(f, "list[{elem}]"),
            Type::Tuple(elems) if elems.is_empty() => f.write_str("tuple[()]"),
            Type::Tuple(elems) => write!(f, "tuple[{}]", join(elems)),
            Type::Optional(inner) => write!(f, "{inner} | None"),
            Type::Var(TypeVar(n)) => write!(f, "T{n}"),
        }
    }
}

impl fmt::Display for FuncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty))
            .collect();
        write!(f, "({}) -> {}", params.join(", "), self.ret)
    }
}

fn join(types: &[Type]) -> String {
    let parts: Vec<String> = types.iter().map(Type::to_string).collect();
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_python_annotations() {
        let t = Type::Tuple(vec![Type::List(Box::new(Type::Int)), Type::Str]);
        assert_eq!(t.to_string(), "tuple[list[int], str]");
        assert_eq!(Type::Tuple(vec![]).to_string(), "tuple[()]");
        assert_eq!(Type::Var(TypeVar(3)).to_string(), "T3");
        let opt = Type::Optional(Box::new(Type::Int));
        assert_eq!(Type::List(Box::new(opt)).to_string(), "list[int | None]");
    }

    #[test]
    fn display_func_type() {
        let ft = FuncType {
            params: vec![ParamType {
                name: "n".to_string(),
                ty: Type::Int,
                has_default: false,
            }],
            ret: Type::None,
        };
        assert_eq!(ft.to_string(), "(n: int) -> None");
    }
}
