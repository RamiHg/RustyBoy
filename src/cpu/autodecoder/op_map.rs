use crate::cpu::register::Register;

use super::asm::{Arg, MaybeArg, Op};
use super::asm_compiler;
use super::micro_code::MicroCode;

pub struct MCycleList(pub Vec<MCycle>);
pub type MCycleMap = std::collections::HashMap<String, MCycleList>;

#[derive(Debug, Clone)]
pub struct MCycle {
    pub t1: SourceOpList,
    pub t2: SourceOpList,
    pub t3: SourceOpList,
    pub t4: SourceOpList,
}

/// Uncompiled micro-code ops as parsed from the CSV. While in this high-level form, can support
/// operations like operand remapping.
#[derive(Debug, Clone)]
pub struct SourceOpList(pub Vec<Op>);

impl MCycleList {
    /// Compiles a list of MCycles down to MicroCodes. Each micro-code represents one TCycle.
    pub fn compile(&self) -> Vec<MicroCode> {
        self.0
            .iter()
            // Get an iterator to each TCycle's instructions
            .map(|x| vec![&x.t1.0, &x.t2.0, &x.t3.0, &x.t4.0])
            .flatten()
            // Replace all empty t-cycles with NOPs.
            .map(|x| {
                if x.is_empty() {
                    Op::nop().iter()
                } else {
                    x.iter()
                }
            })
            // Compile each TCycle.
            .map(|x| asm_compiler::compile_op_list(x))
            // Skip the first 2 nop tcycles.
            .skip(2)
            .collect()
    }

    pub fn remap_lhs_reg(&self, with: Register) -> MCycleList {
        let mapper = |x: &MaybeArg| {
            if let Some(Arg::Lhs) = x.0 {
                MaybeArg(Some(Arg::Register(with)))
            } else if let Some(Arg::LhsLow) = x.0 {
                let (high, low) = with.decompose_pair();
                MaybeArg(Some(Arg::Register(low)))
            } else if let Some(Arg::LhsHigh) = x.0 {
                let (high, low) = with.decompose_pair();
                MaybeArg(Some(Arg::Register(high)))
            } else {
                x.clone()
            }
        };
        self.map_ops(mapper)
    }
    pub fn remap_rhs_reg(&self, with: Register) -> MCycleList {
        let mapper = |x: &MaybeArg| {
            if let Some(Arg::Rhs) = x.0 {
                MaybeArg(Some(Arg::Register(with)))
            } else if let Some(Arg::RhsLow) = x.0 {
                let (high, low) = with.decompose_pair();
                MaybeArg(Some(Arg::Register(low)))
            } else if let Some(Arg::RhsHigh) = x.0 {
                let (high, low) = with.decompose_pair();
                MaybeArg(Some(Arg::Register(high)))
            } else {
                x.clone()
            }
        };
        self.map_ops(mapper)
    }

    fn map_ops(&self, mapper: impl Fn(&MaybeArg) -> MaybeArg) -> MCycleList {
        let op_list_mutater = |ops: &[Op]| {
            SourceOpList(
                ops.iter()
                    .map(|op| Op {
                        cmd: op.cmd.clone(),
                        lhs: mapper(&op.lhs),
                        rhs: mapper(&op.rhs),
                    })
                    .collect(),
            )
        };

        MCycleList(
            self.0
                .iter()
                .map(|x| MCycle {
                    t1: op_list_mutater(&x.t1.0),
                    t2: op_list_mutater(&x.t2.0),
                    t3: op_list_mutater(&x.t3.0),
                    t4: op_list_mutater(&x.t4.0),
                })
                .collect(),
        )
    }
}

// SourceOpList convenience functions to remap arguments.
impl SourceOpList {
    pub fn ops(&self) -> &[Op] {
        &self.0
    }

    fn remap_arg(arg: &MaybeArg, from: &Arg, to: &Arg) -> MaybeArg {
        if let Some(from) = &arg.0 {
            MaybeArg(Some(to.clone()))
        } else {
            arg.clone()
        }
    }

    fn remap_op<Extractor, Zipper>(
        &self,
        from: &Arg,
        with: &Arg,
        extract: Extractor,
        zipper: Zipper,
    ) -> SourceOpList
    where
        Extractor: Fn(&Op) -> &MaybeArg,
        Zipper: Fn((MaybeArg, &Op)) -> Op,
    {
        SourceOpList(
            self.0
                .iter()
                .map(extract)
                .map(|arg| SourceOpList::remap_arg(arg, from, with))
                .zip(self.0.iter())
                .map(zipper)
                .collect(),
        )
    }

    pub fn remap_lhs_reg(&self, with: Register) -> SourceOpList {
        self.remap_op(
            &Arg::Lhs,
            &Arg::Register(with),
            |op| &op.lhs,
            |(arg, op)| Op {
                lhs: arg,
                ..op.clone()
            },
        )
    }

    pub fn remap_rhs_reg(&self, with: Register) -> SourceOpList {
        self.remap_op(
            &Arg::Rhs,
            &Arg::Register(with),
            |op| &op.rhs,
            |(arg, op)| Op {
                rhs: arg,
                ..op.clone()
            },
        )
    }
}
