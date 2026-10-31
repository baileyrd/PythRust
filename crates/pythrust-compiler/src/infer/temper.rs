//! What a `temper def` must spell out (grammar.md D1, D5): every
//! parameter and the return carry a complete annotation, and the body
//! calls only tempered functions and builtins. Checked after inference,
//! so a missing annotation can suggest the type that was inferred.
//!
//! D1's "no `dyn`" holds trivially until `dyn` lands with the Python
//! bridge: inference rejects it as an unknown type everywhere today.

use std::collections::HashMap;

use crate::ast::{FuncDef, Mode, TypeExpr};
use crate::diag::Result;

use super::{error, scope, Type, Types};

/// Checks every tempered function in `defs` against `types`.
pub(super) fn check(defs: &[&FuncDef], types: &Types) -> Result<()> {
    let modes: HashMap<&str, Mode> = defs
        .iter()
        .map(|f| (f.name.name.as_str(), f.mode))
        .collect();
    for f in defs.iter().filter(|f| f.mode == Mode::Tempered) {
        signature(f, types)?;
        calls(f, &modes)?;
    }
    Ok(())
}

/// D1: each parameter, then the return, has a complete annotation.
fn signature(f: &FuncDef, types: &Types) -> Result<()> {
    let name = &f.name.name;
    for p in &f.params {
        let param = &p.name.name;
        let what = match &p.annotation {
            None => format!("tempered `{name}` must annotate parameter `{param}`"),
            Some(t) if !complete(t) => {
                format!("tempered `{name}` must spell the element types of `{param}`")
            }
            Some(_) => continue,
        };
        let fix = suggest(&format!("{param}: "), types.at(p.name.span));
        return Err(error(p.name.span, format!("{what}{fix}")));
    }
    let what = match &f.returns {
        None => format!("tempered `{name}` must annotate its return type"),
        Some(t) if !complete(t) => {
            format!("tempered `{name}` must spell the element types of its return type")
        }
        Some(_) => return Ok(()),
    };
    let fix = suggest("-> ", types.func(name).map(|sig| &sig.ret));
    Err(error(f.name.span, format!("{what}{fix}")))
}

/// Whether `t` names every element type: no bare `list`, `dict` or
/// `set`.
fn complete(t: &TypeExpr) -> bool {
    let container = matches!(t.name.name.as_str(), "list" | "dict" | "set");
    let bare = container && t.args.is_empty();
    !bare && t.args.iter().all(complete)
}

/// "; write `{prefix}{t}`" when the inferred `t` can be spelled, else "".
fn suggest(prefix: &str, inferred: Option<&Type>) -> String {
    match inferred {
        Some(t) if spellable(t) => format!("; write `{prefix}{t}`"),
        _ => String::new(),
    }
}

/// Whether `t` can be written as an annotation: no inference variables
/// (generic or unconstrained), no `range` and no empty tuple.
fn spellable(t: &Type) -> bool {
    match t {
        Type::Var(_) | Type::Range => false,
        Type::List(inner) | Type::Set(inner) | Type::Optional(inner) => spellable(inner),
        Type::Dict(key, value) => spellable(key) && spellable(value),
        Type::Tuple(items) => !items.is_empty() && items.iter().all(spellable),
        Type::Int | Type::Float | Type::Str | Type::Bool | Type::None | Type::Exception => true,
    }
}

/// D5: the first call from `f` to a loose function is an error. In
/// phase 2 a function is only ever named to be called, so every
/// non-local read of a `def`'s name is a call.
fn calls(f: &FuncDef, modes: &HashMap<&str, Mode>) -> Result<()> {
    let loose = scope::name_uses(f)
        .into_iter()
        .find(|(n, _)| modes.get(n) == Some(&Mode::Loose));
    let Some((callee, span)) = loose else {
        return Ok(());
    };
    let msg = format!(
        "tempered `{}` calls loose `{callee}`; temper `{callee}` first",
        f.name.name
    );
    Err(error(span, msg))
}

#[cfg(test)]
mod tests {
    use crate::{infer, lexer, parser};

    /// `check`'s verdict on `src`: `Ok` or the first diagnostic's message.
    fn check(src: &str) -> Result<(), String> {
        let tokens = lexer::lex(src).map_err(|d| d.message)?;
        let module = parser::parse(&tokens).map_err(|d| d.message)?;
        infer::infer(&module).map(drop).map_err(|d| d.message)
    }

    fn err(src: &str) -> String {
        match check(src) {
            Ok(()) => panic!("expected an error for:\n{src}"),
            Err(msg) => msg,
        }
    }

    #[test]
    fn fully_annotated_tempered_code_passes() {
        let src = "\
temper def total(xs: list[int], start: int = 0) -> int:
    t = start
    for x in xs:
        t += x
    return t

temper def show(xs: list[int]) -> None:
    print(len(xs), total(xs), str(total(xs, 1)))

show([1, 2, 3])
";
        assert_eq!(check(src), Ok(()));
    }

    #[test]
    fn loose_code_needs_no_annotations() {
        assert_eq!(check("def f(n):\n    return n + 1\nprint(f(1))\n"), Ok(()));
    }

    #[test]
    fn missing_parameter_annotation_suggests_the_inferred_type() {
        let src = "temper def f(a: int, n) -> int:\n    for i in range(n):\n        a += i\n    return a\n";
        assert_eq!(
            err(src),
            "tempered `f` must annotate parameter `n`; write `n: int`"
        );
    }

    #[test]
    fn missing_return_annotation_suggests_the_inferred_type() {
        let ret = "temper def f(n: int):\n    return [n]\n";
        assert_eq!(
            err(ret),
            "tempered `f` must annotate its return type; write `-> list[int]`"
        );
        let none = "temper def f(n: int):\n    print(n)\n";
        assert_eq!(
            err(none),
            "tempered `f` must annotate its return type; write `-> None`"
        );
    }

    #[test]
    fn unspellable_types_get_no_suggestion() {
        let generic = "temper def f(x):\n    return x\n";
        assert_eq!(err(generic), "tempered `f` must annotate parameter `x`");
        let range = "temper def f(n: int):\n    return range(n)\n";
        assert_eq!(err(range), "tempered `f` must annotate its return type");
        let empty = "temper def f(n: int):\n    return ()\n";
        assert_eq!(err(empty), "tempered `f` must annotate its return type");
    }

    #[test]
    fn bare_list_annotations_are_incomplete() {
        let param = "temper def f(xs: list) -> int:\n    return xs[0]\n";
        assert_eq!(
            err(param),
            "tempered `f` must spell the element types of `xs`; write `xs: list[int]`"
        );
        let nested = "temper def f(n: int) -> tuple[list, int]:\n    return ([n], n)\n";
        assert_eq!(
            err(nested),
            "tempered `f` must spell the element types of its return type; \
             write `-> tuple[list[int], int]`"
        );
    }

    #[test]
    fn bare_dict_and_set_annotations_are_incomplete() {
        let dict = "temper def f(d: dict) -> int:\n    return d[\"a\"]\n";
        assert_eq!(
            err(dict),
            "tempered `f` must spell the element types of `d`; write `d: dict[str, int]`"
        );
        let set = "temper def f(n: int) -> set:\n    return {n}\n";
        assert_eq!(
            err(set),
            "tempered `f` must spell the element types of its return type; write `-> set[int]`"
        );
        let full = "temper def f(d: dict[str, set[int]]) -> int:\n    return len(d)\n";
        assert_eq!(check(full), Ok(()));
    }

    #[test]
    fn tempered_may_not_call_loose() {
        let src = "\
def helper(n: int) -> int:
    return n

temper def f(n: int) -> int:
    return helper(n)
";
        assert_eq!(
            err(src),
            "tempered `f` calls loose `helper`; temper `helper` first"
        );
    }

    #[test]
    fn a_default_calling_loose_counts() {
        let src = "\
def seed() -> int:
    return 1

temper def f(n: int = seed()) -> int:
    return n
";
        assert_eq!(
            err(src),
            "tempered `f` calls loose `seed`; temper `seed` first"
        );
    }

    #[test]
    fn loose_may_call_tempered_and_tempered_may_recurse() {
        let src = "\
temper def even(n: int) -> bool:
    return n == 0 or odd(n - 1)

temper def odd(n: int) -> bool:
    return n != 0 and even(n - 1)

def main():
    print(even(10))

main()
";
        assert_eq!(check(src), Ok(()));
    }

    #[test]
    fn signature_errors_come_before_call_errors() {
        let src = "def g():\n    pass\ntemper def f(n):\n    g()\n";
        assert_eq!(err(src), "tempered `f` must annotate parameter `n`");
    }
}
