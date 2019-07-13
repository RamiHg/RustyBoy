use crate::cpu::alu;
use crate::cpu::micro_code::{Condition, MicroCode};
use crate::cpu::register::Register;

use super::compiler;
use super::{Arg, Command, MaybeArg, Op};

#[derive(Clone, Debug)]
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
            .map(compiler::compile_op_list)
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
        self.map_args(mapper)
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
        self.map_args(mapper)
    }

    pub fn remap_alu_placeholder(&self, with: alu::Op) -> MCycleList {
        let mapper = |cmd: &Command| {
            if let Command::AluPlaceholder = cmd {
                Command::AluOp(with)
            } else {
                *cmd
            }
        };
        self.map_cmds(mapper)
    }

    pub fn remap_cond(&self, with: Condition) -> MCycleList {
        let mapper = |x: &MaybeArg| {
            if let Some(Arg::CCPlaceholder) = x.0 {
                MaybeArg(Some(Arg::CC(with)))
            } else {
                x.clone()
            }
        };
        self.map_args(mapper)
    }

    pub fn remap_i32_placeholder(&self, with: i32) -> MCycleList {
        let mapper = |x: &MaybeArg| {
            if let Some(Arg::IntegerPlaceholder) = x.0 {
                MaybeArg(Some(Arg::Integer(with)))
            } else {
                x.clone()
            }
        };
        self.map_args(mapper)
    }

    pub fn prune_ccend(&self) -> MCycleList {
        self.map_cmds(self.pruner(Command::CCEND))
    }
    pub fn prune_ei(&self) -> MCycleList {
        self.map_cmds(self.pruner(Command::EI))
    }
    pub fn prune_bit(&self) -> MCycleList {
        self.map_cmds(self.pruner(Command::BIT))
    }

    fn map_ops(
        &self,
        arg_mapper: impl Fn(&MaybeArg) -> MaybeArg,
        cmd_mapper: impl Fn(&Command) -> Command,
    ) -> MCycleList {
        let op_list_mutater = |ops: &[Op]| {
            SourceOpList(
                ops.iter()
                    .map(|op| Op {
                        cmd: cmd_mapper(&op.cmd),
                        lhs: arg_mapper(&op.lhs),
                        rhs: arg_mapper(&op.rhs),
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

    fn map_args(&self, mapper: impl Fn(&MaybeArg) -> MaybeArg) -> MCycleList {
        self.map_ops(mapper, |cmd| *cmd)
    }

    fn map_cmds(&self, mapper: impl Fn(&Command) -> Command) -> MCycleList {
        self.map_ops(|arg| arg.clone(), mapper)
    }

    fn pruner(&self, prune: Command) -> impl Fn(&Command) -> Command {
        move |cmd: &Command| if *cmd == prune { Command::NOP } else { *cmd }
    }
}
