//! Bounded, backend-private constant-divisor call-site clones (#816, D69).
//!
//! A direct call that passes an int literal to a parameter its callee
//! only reads, and divides by with int `//` or `%`, may target a private
//! clone of the callee whose body binds that parameter to the literal at
//! entry. The call stays a real call with the same signature, arguments,
//! depth check and spans; only the divisor becomes a compile-time
//! constant the backend can strength-reduce. 0 and -1 never qualify, so
//! the zero-division and overflow fault paths stay exactly as they are.

use std::collections::{BTreeMap, BTreeSet};

use cranelift_module::{FuncId as ClifFunc, Linkage, Module};

use super::{module_err, pair, Codegen};
use crate::diag::Result;
use crate::ir::{Func, FuncId, Inst, IntOp, LocalId, Operand, Program, Rvalue, Ty};

/// At most this many clones of one callee, first come in program order.
const MAX_CLONES: usize = 4;
/// Only callees with at most this many instructions are cloned.
const MAX_INSTS: usize = 64;

/// One private clone: `callee` with its `arg`th parameter bound to
/// `value`, in its scalar-pair (D63) variant when `scalar`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Clone {
    pub callee: FuncId,
    pub arg: usize,
    pub value: i64,
    pub scalar: bool,
}

/// The parameters of `f` eligible for binding: `int`, never redefined,
/// and the divisor of at least one int `//` or `%`.
fn divisor_params(f: &Func) -> BTreeSet<LocalId> {
    let insts: Vec<&Inst> = f.blocks.iter().flat_map(|b| &b.insts).collect();
    if insts.len() > MAX_INSTS {
        return BTreeSet::new();
    }
    f.params
        .iter()
        .copied()
        .filter(|p| f.locals[p.0] == Ty::Int)
        .filter(|p| insts.iter().all(|i| !redefines(i, *p)))
        .filter(|p| f.tasks.iter().all(|t| t.dest != *p))
        .filter(|p| insts.iter().any(|i| divides_by(i, *p)))
        .collect()
}

fn redefines(inst: &Inst, l: LocalId) -> bool {
    match inst {
        Inst::Assign { dest, .. } => *dest == l,
        Inst::Release(x) | Inst::Forget(x) => *x == l,
        _ => false,
    }
}

fn divides_by(inst: &Inst, l: LocalId) -> bool {
    matches!(
        inst,
        Inst::Assign {
            value: Rvalue::Int(IntOp::FloorDiv | IntOp::Mod, _, Operand::Local(d)),
            ..
        } if *d == l
    )
}

/// The first eligible literal argument of a call to `callee`, if any.
fn bound(callee: &Func, eligible: &BTreeSet<LocalId>, args: &[Operand]) -> Option<(usize, i64)> {
    callee
        .params
        .iter()
        .zip(args)
        .enumerate()
        .find_map(|(k, (p, a))| match a {
            Operand::Int(c) if eligible.contains(p) && *c != 0 && *c != -1 => Some((k, *c)),
            _ => None,
        })
}

/// Every clone the program's direct calls select, capped per callee.
/// The scalar flag follows the call site's own D63 plan.
pub(super) fn plan(program: &Program, pairs: &[pair::Plan]) -> BTreeSet<Clone> {
    let eligible: Vec<_> = program.funcs.iter().map(divisor_params).collect();
    let mut chosen: BTreeMap<FuncId, BTreeSet<Clone>> = BTreeMap::new();
    for (f, plan) in program.funcs.iter().zip(pairs) {
        for inst in f.blocks.iter().flat_map(|b| &b.insts) {
            let Some(c) = site(program, &eligible, plan, inst) else {
                continue;
            };
            let clones = chosen.entry(c.callee).or_default();
            if clones.len() < MAX_CLONES || clones.contains(&c) {
                clones.insert(c);
            }
        }
    }
    chosen.into_values().flatten().collect()
}

fn site(
    program: &Program,
    eligible: &[BTreeSet<LocalId>],
    plan: &pair::Plan,
    inst: &Inst,
) -> Option<Clone> {
    let Inst::Assign {
        dest,
        value: Rvalue::Call(callee, args),
        ..
    } = inst
    else {
        return None;
    };
    let (arg, value) = bound(&program.funcs[callee.0], &eligible[callee.0], args)?;
    Some(Clone {
        callee: *callee,
        arg,
        value,
        scalar: plan.calls.contains(dest),
    })
}

/// The planned clone a call of `callee` with `args` may target: any
/// one binding a parameter to the literal this call passes it.
pub(super) fn target(
    clones: &BTreeMap<Clone, ClifFunc>,
    callee: FuncId,
    args: &[Operand],
    scalar: bool,
) -> Option<ClifFunc> {
    args.iter().enumerate().find_map(|(arg, a)| match a {
        Operand::Int(value) => clones
            .get(&Clone {
                callee,
                arg,
                value: *value,
                scalar,
            })
            .copied(),
        _ => None,
    })
}

impl Codegen<'_> {
    pub(super) fn declare_clones(
        &mut self,
        program: &Program,
        plan: BTreeSet<Clone>,
    ) -> Result<()> {
        for c in plan {
            let f = &program.funcs[c.callee.0];
            let sig = match c.scalar {
                true => self.pair_signature(f),
                false => self.signature(f),
            };
            let sign = if c.value < 0 { "m" } else { "" };
            let name = format!(
                "pt_const_{}_{}_{sign}{}{}",
                c.callee.0,
                c.arg,
                c.value.unsigned_abs(),
                if c.scalar { "_pair" } else { "" }
            );
            let id = self
                .module
                .declare_function(&name, Linkage::Local, &sig)
                .map_err(module_err)?;
            self.clones.insert(c, id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infer, lexer, lower, parser, rc};

    fn clones(src: &str) -> Vec<(String, i64, bool)> {
        let ast = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let tys = infer::infer(&ast).expect("infer");
        let p = rc::elide(lower::lower(&ast, &tys).expect("lower"));
        super::super::emit(&p, &|d| d.message.clone()).expect("emit");
        plan(&p, &pair::plans(&p))
            .into_iter()
            .map(|c| (p.funcs[c.callee.0].name.clone(), c.value, c.scalar))
            .collect()
    }

    #[test]
    fn the_tuple_workload_binds_its_literal_divisor_in_the_pair_clone() {
        let got = clones(include_str!("../../../../bench/tuples.pyr"));
        assert_eq!(got, [("div_mod".to_string(), 10, true)]);
    }

    #[test]
    fn literal_divisors_select_one_clone_per_value_and_variant() {
        let got = clones(
            "def r(a: int, b: int) -> int:\n    return a % b\n\
             def q(a: int, b: int) -> tuple[int, int]:\n    return a // b, a % b\n\
             print(r(7, 10), r(8, 10), r(9, 3), r(9, -10))\n\
             x, y = q(7, 10)\nt = q(7, 10)\nprint(x, y, t)\n",
        );
        assert_eq!(
            got,
            [
                ("r".to_string(), -10, false),
                ("r".to_string(), 3, false),
                ("r".to_string(), 10, false),
                ("q".to_string(), 10, false),
                ("q".to_string(), 10, true),
            ]
        );
    }

    #[test]
    fn uncertain_or_fault_preserving_sites_keep_the_original() {
        for src in [
            // Dynamic divisor, and the fault-path literals 0 and -1.
            "def r(a: int, b: int) -> int:\n    return a % b\nd = 10\nprint(r(7, d))\n",
            "def r(a: int, b: int) -> int:\n    return a // b\nprint(r(7, 0))\n",
            "def r(a: int, b: int) -> int:\n    return a // b\nprint(r(7, -1))\n",
            // A parameter that is redefined, or not a divisor.
            "def r(a: int, b: int) -> int:\n    b = b + 1\n    return a // b\nprint(r(7, 10))\n",
            "def r(a: int, b: int) -> int:\n    return a * b\nprint(r(7, 10))\n",
            // Float division, and a default argument.
            "def r(a: float, b: float) -> float:\n    return a // b\nprint(r(7.0, 10.0))\n",
            "def r(a: int, b: int = 10) -> int:\n    return a // b\nprint(r(7))\n",
        ] {
            assert!(clones(src).is_empty(), "{src}");
        }
    }

    #[test]
    fn the_corpus_fixture_clones_the_extreme_divisors() {
        let got = clones(include_str!("../../../../examples/const_divisor.pyr"));
        for extreme in [i64::MAX, i64::MIN] {
            for scalar in [false, true] {
                assert!(
                    got.iter().any(|(_, v, s)| *v == extreme && *s == scalar),
                    "{extreme} (pair: {scalar}) not cloned: {got:?}"
                );
            }
        }
    }

    #[test]
    fn clones_are_capped_per_callee() {
        let calls: String = (2..12).map(|d| format!("print(r(99, {d}))\n")).collect();
        let got = clones(&format!(
            "def r(a: int, b: int) -> int:\n    return a // b\n{calls}"
        ));
        assert_eq!(got.len(), MAX_CLONES);
    }
}
