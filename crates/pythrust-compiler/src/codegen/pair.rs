//! Bounded, backend-private two-int return/unpack specialization (#806).
//!
//! The IR and its boxed functions remain intact. A private clone carries
//! only a proven return construction/copy chain in two scalar variables.
//! Callers use it only for an unnamed, single-definition call temporary
//! read exactly once per field, with no other use except cleanup.

use std::collections::BTreeSet;

use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::Variable;
use cranelift_module::{Linkage, Module};

use super::{backend, module_err, Codegen, FnGen};
use crate::diag::Result;
use crate::ir::{Compound, Func, Inst, LocalId, Operand, Program, Rvalue, Term, Ty};

#[derive(Default)]
pub(super) struct Plan {
    pub returns: BTreeSet<LocalId>,
    pub calls: BTreeSet<LocalId>,
}

fn is_pair(program: &Program, ty: Ty) -> bool {
    matches!(program.compound(ty), Some(Compound::Tuple(ts)) if ts == &[Ty::Int, Ty::Int])
}

/// Follow only copies backwards from the common return slot. Reject the
/// entire clone if any of those locals can originate elsewhere or escape.
fn returns(program: &Program, f: &Func) -> BTreeSet<LocalId> {
    if !is_pair(program, f.ret) {
        return BTreeSet::new();
    }
    let mut locals: BTreeSet<_> = f
        .blocks
        .iter()
        .filter_map(|b| match b.term {
            Term::Return(Operand::Local(l)) => Some(l),
            _ => None,
        })
        .collect();
    loop {
        let old = locals.len();
        for inst in f.blocks.iter().flat_map(|b| &b.insts) {
            if let Inst::Assign {
                dest,
                value: Rvalue::Use(Operand::Local(src)),
                ..
            } = inst
            {
                if locals.contains(dest) {
                    locals.insert(*src);
                }
            }
        }
        if locals.len() == old {
            break;
        }
    }
    if locals
        .iter()
        .any(|l| f.params.contains(l) || !is_pair(program, f.locals[l.0]))
    {
        return BTreeSet::new();
    }
    let mut constructed = false;
    for inst in f.blocks.iter().flat_map(|b| &b.insts) {
        let valid = match inst {
            Inst::Assign {
                dest,
                value: Rvalue::NewSeq(items),
                ..
            } if locals.contains(dest) => {
                constructed = true;
                items.len() == 2 && items.iter().all(|v| f.ty(*v) == Ty::Int)
            }
            Inst::Assign {
                dest,
                value: Rvalue::Use(Operand::Local(src)),
                ..
            } if locals.contains(dest) => locals.contains(src),
            Inst::Retain(l) | Inst::Release(l) | Inst::Forget(l) => locals.contains(l),
            _ => false,
        };
        if !valid && locals.iter().any(|l| inst.mentions(*l)) {
            return BTreeSet::new();
        }
    }
    if !constructed {
        locals.clear();
    }
    locals
}

fn unpacked(f: &Func, dest: LocalId) -> bool {
    if f.names.contains_key(&dest) || f.params.contains(&dest) {
        return false;
    }
    let mut definitions = 0;
    let mut fields = [0, 0];
    for b in &f.blocks {
        if match b.term {
            Term::Return(v) | Term::Branch { cond: v, .. } => v == Operand::Local(dest),
            _ => false,
        } {
            return false;
        }
        for inst in &b.insts {
            if !inst.mentions(dest) {
                continue;
            }
            match inst {
                Inst::Assign {
                    dest: d,
                    value: Rvalue::Call(_, args),
                    ..
                } if *d == dest && !args.contains(&Operand::Local(dest)) => definitions += 1,
                Inst::Assign {
                    dest: d,
                    value: Rvalue::Field(Operand::Local(l), k),
                    ..
                } if *l == dest && *d != dest && *k < 2 => fields[*k] += 1,
                Inst::Release(l) | Inst::Forget(l) if *l == dest => {}
                _ => return false,
            }
        }
    }
    definitions == 1 && fields == [1, 1]
}

pub(super) fn plans(program: &Program) -> Vec<Plan> {
    let mut plans: Vec<_> = program
        .funcs
        .iter()
        .map(|f| Plan {
            returns: returns(program, f),
            calls: BTreeSet::new(),
        })
        .collect();
    for (i, f) in program.funcs.iter().enumerate() {
        for inst in f.blocks.iter().flat_map(|b| &b.insts) {
            if let Inst::Assign {
                dest,
                value: Rvalue::Call(id, _),
                ..
            } = inst
            {
                if !plans[id.0].returns.is_empty() && unpacked(f, *dest) {
                    plans[i].calls.insert(*dest);
                }
            }
        }
    }
    plans
}

impl Codegen<'_> {
    pub(super) fn declare_pairs(&mut self, program: &Program, plans: &[Plan]) -> Result<()> {
        let used: BTreeSet<_> = program
            .funcs
            .iter()
            .zip(plans)
            .flat_map(|(f, p)| {
                f.blocks
                    .iter()
                    .flat_map(|b| &b.insts)
                    .filter_map(|inst| match inst {
                        Inst::Assign {
                            dest,
                            value: Rvalue::Call(id, _),
                            ..
                        } if p.calls.contains(dest) => Some(*id),
                        _ => None,
                    })
            })
            .collect();
        for id in used {
            let sig = self.pair_signature(&program.funcs[id.0]);
            let name = format!("pt_scalar_pair_{}", id.0);
            let clif = self
                .module
                .declare_function(&name, Linkage::Local, &sig)
                .map_err(module_err)?;
            self.pairs.insert(id, clif);
        }
        Ok(())
    }

    pub(super) fn pair_signature(&self, f: &Func) -> cranelift_codegen::ir::Signature {
        let mut sig = self.signature(f);
        // Private caller and clone share Cranelift's internal convention;
        // do not expose multiple results through a platform C ABI.
        sig.call_conv = CallConv::Fast;
        sig.returns = vec![AbiParam::new(types::I64); 2];
        sig
    }
}

impl FnGen<'_, '_, '_> {
    pub(super) fn pair_values(&mut self, l: LocalId) -> Result<[Value; 2]> {
        let vars = self
            .pair_vars
            .get(&l)
            .copied()
            .ok_or_else(|| backend("missing scalar pair".into()))?;
        Ok(vars.map(|v| self.b.use_var(v)))
    }

    pub(super) fn set_pair(&mut self, l: LocalId, vals: [Value; 2]) -> Result<()> {
        let vars = self
            .pair_vars
            .get(&l)
            .copied()
            .ok_or_else(|| backend("missing scalar pair".into()))?;
        for (var, val) in vars.into_iter().zip(vals) {
            self.b.def_var(var, val);
        }
        Ok(())
    }

    /// Return true only when this instruction was fully handled here.
    pub(super) fn pair_inst(&mut self, inst: &Inst) -> Result<bool> {
        match inst {
            Inst::Assign { dest, value, .. } if self.pair_vars.contains_key(dest) => {
                let vals = match value {
                    Rvalue::NewSeq(items) => [self.operand(items[0]), self.operand(items[1])],
                    Rvalue::Use(Operand::Local(src)) => self.pair_values(*src)?,
                    // Calls retain the ordinary call-depth guard and are
                    // emitted by call(), which also defines both scalars.
                    Rvalue::Call(..) => return Ok(false),
                    _ => return Err(backend("unproven scalar pair assignment".into())),
                };
                self.set_pair(*dest, vals)?;
            }
            Inst::Assign {
                dest,
                value: Rvalue::Field(Operand::Local(src), k),
                ..
            } if self.pair_vars.contains_key(src) => {
                let vals = self.pair_values(*src)?;
                self.b.def_var(self.vars[dest.0], vals[*k]);
            }
            Inst::Retain(l) if self.pair_vars.contains_key(l) => {}
            Inst::Release(l) | Inst::Forget(l) if self.pair_vars.contains_key(l) => {
                let zero = self.b.ins().iconst(types::I64, 0);
                self.set_pair(*l, [zero; 2])?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

pub(super) fn variables(
    b: &mut cranelift_frontend::FunctionBuilder<'_>,
    locals: BTreeSet<LocalId>,
) -> std::collections::HashMap<LocalId, [Variable; 2]> {
    locals
        .into_iter()
        .map(|l| {
            let vars = [b.declare_var(types::I64), b.declare_var(types::I64)];
            let zero = b.ins().iconst(types::I64, 0);
            for var in vars {
                b.def_var(var, zero);
            }
            (l, vars)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infer, lexer, lower, parser, rc};

    fn program(src: &str) -> Program {
        let ast = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let tys = infer::infer(&ast).expect("infer");
        let p = rc::elide(lower::lower(&ast, &tys).expect("lower"));
        super::super::emit(&p, &|d| d.message.clone()).expect("emit both entries");
        p
    }

    #[test]
    fn eligible_and_escaping_sites_share_the_original_function() {
        let p = program("def pair(a: int, b: int) -> tuple[int, int]:\n    return a, b\nx, y = pair(1, 2)\nt = pair(3, 4)\nprint(x, y, t)\n");
        let plans = plans(&p);
        assert_eq!(plans.iter().filter(|p| !p.returns.is_empty()).count(), 1);
        assert_eq!(plans.iter().map(|p| p.calls.len()).sum::<usize>(), 1);
        let calls = p
            .funcs
            .iter()
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.insts)
            .filter(|i| {
                matches!(
                    i,
                    Inst::Assign {
                        value: Rvalue::Call(..),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(calls, 2, "boxed IR remains unchanged");
    }

    #[test]
    fn uncertain_return_origins_or_uses_fall_back() {
        for src in [
            "def f(t: tuple[int, int]) -> tuple[int, int]:\n    return t\na, b = f((1, 2))\n",
            "def f() -> tuple[int, int]:\n    t = (1, 2)\n    print(t)\n    return t\na, b = f()\n",
            "def f() -> tuple[int, bool]:\n    return 1, True\na, b = f()\n",
            "def f() -> tuple[int | None, int]:\n    return None, 1\na, b = f()\n",
            "def f() -> tuple[tuple[int, int], int]:\n    return (1, 2), 3\na, b = f()\n",
            "def f() -> tuple[int, int] | None:\n    return 1, 2\nt = f()\n",
        ] {
            let p = program(src);
            assert!(plans(&p).iter().all(|p| p.calls.is_empty()), "{src}");
        }
    }

    #[test]
    fn a_named_tuple_or_single_projection_keeps_boxing() {
        for tail in ["t = f()\na, b = t\n", "a = f()[0]\n", "print(f())\n"] {
            let p = program(&format!(
                "def f() -> tuple[int, int]:\n    return 1, 2\n{tail}"
            ));
            assert!(plans(&p).iter().all(|p| p.calls.is_empty()), "{tail}");
        }
    }

    #[test]
    fn workload_and_multi_return_exception_paths_are_eligible() {
        let p = program(include_str!("../../../../bench/tuples.pyr"));
        let ps = plans(&p);
        assert_eq!(ps.iter().filter(|p| !p.returns.is_empty()).count(), 2);
        assert_eq!(ps.iter().map(|p| p.calls.len()).sum::<usize>(), 2);
        let p = program("def f(n: int) -> tuple[int, int]:\n    try:\n        if n < 0:\n            return -9223372036854775808, 9223372036854775807\n        return n // n, n % n\n    except ZeroDivisionError:\n        return 0, 0\na, b = f(0)\n");
        assert_eq!(plans(&p).iter().map(|p| p.calls.len()).sum::<usize>(), 1);
    }
}
