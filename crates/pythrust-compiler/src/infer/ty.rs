//! Types as inference produces them.

use std::fmt;

use crate::stdlib::Kind;

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
    /// `dict[K, V]`: insertion-ordered, keyed by a hashable `K`.
    Dict(Box<Type>, Box<Type>),
    /// `set[T]`: insertion-ordered, of a hashable `T`.
    Set(Box<Type>),
    /// What `d.keys()`, `d.values()` or `d.items()` returns, with the
    /// type its iteration yields: the key, the value, or a `(key,
    /// value)` tuple for `items`.
    View(View, Box<Type>),
    /// Fixed length, positionally typed.
    Tuple(Vec<Type>),
    /// `T | None`. `T` is never `None` or itself optional.
    Optional(Box<Type>),
    /// A builtin exception value (`ValueError("bad")`), whichever its
    /// name: one type, so `except ValueError as e` and `except IndexError
    /// as e` can share `e`.
    Exception,
    /// Immutable bytes, as `f.read()` returns (M6a).
    Bytes,
    /// What `open(p, mode)` returns (D91, D99): whether it is written,
    /// and what it reads or writes, `str` or `bytes`, which together are
    /// its [`Kind`]. The methods it has depend on it (D92). The data is a
    /// type, so a generic function's `f.read()` is typed by its caller.
    File(bool, Box<Type>),
    /// `concurrent.futures.ThreadPoolExecutor()` (M6c).
    Executor,
    /// What `ex.submit(f, ...)` returns: `f`'s result, once it ends.
    Future(Box<Type>),
    /// What `ex.map(f, xs)` returns: `f`'s results, in order, for `for`.
    Results(Box<Type>),
    /// A frozen record (step 4, D129), by its qualified name: nominal,
    /// so two records with the same fields are different types.
    Record(String),
    Var(TypeVar),
}

/// Which view of a dict a [`Type::View`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Keys,
    Values,
    Items,
}

impl View {
    /// Python's type name: `dict_keys`, `dict_values`, `dict_items`.
    pub fn name(self) -> &'static str {
        match self {
            View::Keys => "dict_keys",
            View::Values => "dict_values",
            View::Items => "dict_items",
        }
    }
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
    /// Whether values of this type are heap values (D2): a `str`, list,
    /// dict, set or exception, or a tuple or optional holding one. The rest are
    /// copied. Defined on the language-level type, so no backend's
    /// representation changes which modes a program may write. `None`
    /// while it depends on a generic type.
    pub fn holds_heap(&self) -> Option<bool> {
        match self {
            Type::Str
            | Type::List(_)
            | Type::Dict(..)
            | Type::Set(_)
            | Type::View(..)
            | Type::Exception
            | Type::Bytes
            | Type::File(..)
            | Type::Executor
            | Type::Future(_)
            | Type::Results(_)
            | Type::Record(_) => Some(true),
            _ => self.any_part(Type::holds_heap),
        }
    }

    /// The file of `kind`.
    pub fn file(kind: Kind) -> Type {
        let data = match kind.is_text() {
            true => Type::Str,
            false => Type::Bytes,
        };
        Type::File(kind.writes(), Box::new(data))
    }

    /// A file's kind, once what it holds is known.
    pub fn file_kind(&self) -> Option<Kind> {
        let Type::File(writes, data) = self else {
            return None;
        };
        match (**data == Type::Str, **data == Type::Bytes, writes) {
            (true, _, false) => Some(Kind::TextReader),
            (true, _, true) => Some(Kind::TextWriter),
            (_, true, false) => Some(Kind::BytesReader),
            (_, true, true) => Some(Kind::BytesWriter),
            _ => None,
        }
    }

    /// Whether the type's own operations reach a mutable heap value, so
    /// a value of it can change in place (D2 `mut`): a list, dict or
    /// set, or a tuple, optional or dict view holding one (a view itself
    /// cannot change), or a file, which reading moves and `close` closes.
    /// Not an exception: no operation reaches its argument. `None` while
    /// it depends on a generic type.
    pub fn reaches_mutable(&self) -> Option<bool> {
        match self {
            Type::List(_) | Type::Dict(..) | Type::Set(_) | Type::File(..) | Type::Executor => {
                Some(true)
            }
            // Whether a field reaches one is the record's, not known here.
            Type::Record(_) => None,
            _ => self.any_part(Type::reaches_mutable),
        }
    }

    /// Whether `pred` holds of a tuple field, an optional's value, a
    /// view's items or a task's result; false for any other type.
    fn any_part(&self, pred: fn(&Type) -> Option<bool>) -> Option<bool> {
        let parts: Vec<Option<bool>> = match self {
            Type::Tuple(fields) => fields.iter().map(pred).collect(),
            Type::Optional(present)
            | Type::View(_, present)
            | Type::Future(present)
            | Type::Results(present) => vec![pred(present)],
            Type::Var(_) => vec![None],
            _ => Vec::new(),
        };
        if parts.contains(&Some(true)) {
            return Some(true);
        }
        match parts.contains(&None) {
            true => None,
            false => Some(false),
        }
    }

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
            Type::Exception => f.write_str("Exception"),
            Type::Bytes => f.write_str("bytes"),
            Type::File(..) => f.write_str(match self.file_kind() {
                Some(Kind::TextReader) => "TextIOWrapper (reading)",
                Some(Kind::TextWriter) => "TextIOWrapper (writing)",
                Some(other) => other.class(),
                None => "file",
            }),
            Type::Executor => f.write_str("ThreadPoolExecutor"),
            Type::Future(result) => write!(f, "Future[{result}]"),
            Type::Results(result) => write!(f, "Iterator[{result}]"),
            Type::List(elem) => write!(f, "list[{elem}]"),
            Type::Dict(key, value) => write!(f, "dict[{key}, {value}]"),
            Type::Set(elem) => write!(f, "set[{elem}]"),
            Type::View(view, item) => match (view, &**item) {
                (View::Items, Type::Tuple(pair)) => write!(f, "dict_items[{}]", join(pair)),
                _ => write!(f, "{}[{item}]", view.name()),
            },
            Type::Tuple(elems) if elems.is_empty() => f.write_str("tuple[()]"),
            Type::Tuple(elems) => write!(f, "tuple[{}]", join(elems)),
            Type::Optional(inner) => write!(f, "{inner} | None"),
            Type::Record(name) => f.write_str(name),
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
