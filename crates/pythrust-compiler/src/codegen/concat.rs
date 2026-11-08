//! Adjacent, single-use string prefixes only (#807, D64).
//! IR/ownership stay unchanged. Never move operand evaluation across an
//! allocation: the tail is an unchanged borrowed parameter, already live.

use super::{FnGen, Rt};
use crate::diag::Result;
use crate::ir::{Func, Inst, LocalId, Operand, Rvalue, Term, Ty};

/// Exactly two consecutive assignments; no look-through of checks or RC.
fn candidate(f: &Func, insts: &[Inst]) -> Option<(LocalId, LocalId, [Operand; 3])> {
    let [Inst::Assign {
        dest: prefix,
        value: Rvalue::Concat(a, b),
        ..
    }, Inst::Assign {
        dest,
        value: Rvalue::Concat(Operand::Local(p), c),
        ..
    }, ..] = insts
    else {
        return None;
    };
    let Operand::Local(tail) = c else {
        return None;
    };
    if p != prefix
        || dest == prefix
        || f.names.contains_key(prefix)
        || f.params.contains(prefix)
        || !f.params.contains(tail)
        || !f.borrowed.contains(tail)
        || [*a, *b, *c].contains(&Operand::Local(*prefix))
        || [*a, *b, *c].contains(&Operand::Local(*dest))
        || [*a, *b, *c].iter().any(|v| f.ty(*v) != Ty::Str)
    {
        return None;
    }
    let mut definitions = 0;
    let mut reads = 0;
    for block in &f.blocks {
        if matches!(block.term, Term::Return(v) | Term::Branch { cond: v, .. }
            if v == Operand::Local(*prefix))
        {
            return None;
        }
        for inst in &block.insts {
            if !inst.mentions(*prefix) {
                continue;
            }
            match inst {
                Inst::Assign {
                    dest: d,
                    value: Rvalue::Concat(x, y),
                    ..
                } if d == prefix && ![*x, *y].contains(&Operand::Local(*prefix)) => {
                    definitions += 1
                }
                Inst::Assign {
                    dest: d,
                    value: Rvalue::Concat(Operand::Local(l), rhs),
                    ..
                } if d == dest && l == prefix && rhs == c => reads += 1,
                Inst::Release(l) | Inst::Forget(l) if l == prefix => {}
                _ => return None,
            }
        }
    }
    (definitions == 1 && reads == 1).then_some((*prefix, *dest, [*a, *b, *c]))
}

impl FnGen<'_, '_, '_> {
    pub(super) fn concat_insts(&mut self, insts: &[Inst]) -> Result<()> {
        let mut i = 0;
        while i < insts.len() {
            if let Some((prefix, dest, args)) = candidate(self.func, &insts[i..]) {
                let value = self.rt_on(Rt::StrConcat3, &args)?;
                self.forget(prefix);
                self.b.def_var(self.vars[dest.0], value);
                i += 2;
            } else {
                self.inst(&insts[i])?;
                i += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infer, lexer, lower, parser, rc};

    fn program(src: &str) -> crate::ir::Program {
        let ast = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let tys = infer::infer(&ast).expect("infer");
        let p = rc::elide(lower::lower(&ast, &tys).expect("lower"));
        super::super::emit(&p, &|d| d.message.clone()).expect("emit");
        p
    }

    fn sites(p: &crate::ir::Program) -> usize {
        p.funcs
            .iter()
            .map(|f| {
                f.blocks
                    .iter()
                    .map(|b| {
                        let (mut i, mut count) = (0, 0);
                        while i < b.insts.len() {
                            if candidate(f, &b.insts[i..]).is_some() {
                                count += 1;
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        count
                    })
                    .sum::<usize>()
            })
            .sum()
    }

    #[test]
    fn only_adjacent_prefix_and_already_live_tail_are_eligible() {
        let head = "def f(a: str, b: str, c: str) -> str:\n";
        for body in [
            "    return a + b + c\n",
            "    return (a + b) + c\n",
            "    return a + b + c + a\n",
        ] {
            assert_eq!(
                sites(&program(&format!(
                    "{head}{body}print(f(\"a\", \"b\", \"c\"))\n"
                ))),
                1,
                "{body}"
            );
        }
        for body in [
            "    return a + (b + c)\n",
            "    return a + b + str(1)\n",
            "    return a + b + 'tail'\n",
            "    x = a + b\n    return x + c\n",
            "    c = 'changed'\n    return a + b + c\n",
            "    return a + b + (c if len(c) else a)\n",
        ] {
            assert_eq!(
                sites(&program(&format!(
                    "{head}{body}print(f(\"a\", \"b\", \"c\"))\n"
                ))),
                0,
                "{body}"
            );
        }
    }

    #[test]
    fn shared_prefix_or_additional_definition_cannot_be_eliminated() {
        let mut p = program("def f(a: str, b: str, c: str) -> str:\n    return a + b + c\nprint(f(\"a\", \"b\", \"c\"))\n");
        let f = p.funcs.iter_mut().find(|f| f.name == "f").expect("f");
        let (block, index, prefix) = f
            .blocks
            .iter()
            .enumerate()
            .find_map(|(bi, b)| {
                b.insts
                    .windows(2)
                    .enumerate()
                    .find_map(|(i, is)| candidate(f, is).map(|x| (bi, i, x.0)))
            })
            .expect("candidate");
        let original = f.blocks[block].insts[index].clone();
        f.blocks[block].insts.push(Inst::Retain(prefix));
        assert_eq!(sites(&p), 0);
        let f = p.funcs.iter_mut().find(|f| f.name == "f").expect("f");
        f.blocks[block].insts.pop();
        f.blocks[block].insts.push(original);
        assert_eq!(sites(&p), 0);
    }

    #[test]
    fn effectful_prefix_and_container_alias_keep_the_proven_tail_live() {
        let p = program(include_str!("../../../../examples/string_chain.pyr"));
        for name in ["effects", "overwritten"] {
            let f = p.funcs.iter().find(|f| f.name == name).expect("function");
            assert!(
                f.blocks
                    .iter()
                    .any(|b| b.insts.windows(2).any(|is| candidate(f, is).is_some())),
                "{name}"
            );
        }
        for name in ["later", "right", "last", "indexed", "branch"] {
            let f = p.funcs.iter().find(|f| f.name == name).expect("function");
            assert!(
                f.blocks
                    .iter()
                    .all(|b| b.insts.windows(2).all(|is| candidate(f, is).is_none())),
                "{name}"
            );
        }
    }

    #[test]
    fn existing_string_workload_keeps_its_operand_allocation_order() {
        let p = program(include_str!("../../../../bench/strings.pyr"));
        assert_eq!(sites(&p), 0);
    }
}
