use crate::cpu::decoder;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Result};
use crate::memory::Memory;

use super::alu;

pub enum SideEffect {
    Write {
        raw_address: i32,
        value: i32,
    },
    Decode(Vec<MicroCode>),
    /// Triggers an early instruction end by dropping the rest of the microcodes in the stack.
    /// Happens during conditional calls/returns.
    DropRemainingMicroCodes,
}

/// The output of a micro code execution.
/// Contains any possible side effects, as well as a flag signifying the macro-instruction is
/// complete.
pub struct Output {
    pub side_effect: Option<SideEffect>,
    pub is_done: bool,
}

/// Stages:
/// Each instruction is modeled as a series of micro-codes; one for each machine cycle.
/// The micro code itself is composed of 4 stages, each corresponding to a system tick.
/// The stages are:
/// 1. Memory stage. An abstraction over the read/write and address signal stages (i.e., the first
///     two ticks).
/// 2. Register-control stage. Miscellaneous simple register move. Can operate on results of memory
///     reads. This is the third tick.
/// 3. The last tick. There is where most of the logic and ALU happens. It is modeled as separate
///     components in the following order:
///     a-1) Decoder stage: If set, will run the decoder logic.
///     a) IncrementerStage: Simple 16-bit incrementer. Probably exists in real hardware. Does not
///         affect or real flags.
///     b) ALU Stage. Various ALU operations happen here.
///     c) Post-ALU register control: Convenience stage that can do simple register moves/sets.

/// TODO: A lot of the register control and "post-alu" stages can be remodeled as PC, TEMP, HL, and
/// SP controls.

#[derive(Debug)]
pub enum MemoryStage {
    Read {
        destination: Register,
        address: Register,
    },
    Write {
        address: Register,
        value: Register,
    },
}

pub enum AluOp {
    BinaryOp {
        op: alu::BinaryOp,
        lhs: Register,
        rhs: Register,
    },
    UnaryOp {
        op: alu::UnaryOp,
        register: Register,
    },
    Move {
        destination: Register,
        source: Register,
    },
}

pub struct AluStage {
    pub op: AluOp,
    pub flag_condition: Option<FlagCondition>,
}

#[derive(Debug)]
pub enum RegisterControl {
    Set(Register, i32),
    Move(Register, Register),
    RestoreFlags {
        source: Register,
        mask: alu::Flags,
    },
    // Moves source into destination if flag conditions are met.
    // ConditionalMove {
    //     destination: Register,
    //     source: Register,
    //     flags: alu::Flags,
    //     is_set: bool,
    // },
    SignExtend {
        destination: Register,
        source: Register,
    },
}

#[derive(Debug)]
pub enum IncrementerStage {
    Increment(Register),
    Decrement(Register),
}

pub enum DecoderStage {
    Decode,
    ConditionalDone(FlagCondition),
}

#[derive(Debug, Clone, Copy)]
pub struct FlagCondition {
    pub test_flags: alu::Flags,
    pub test_is_set: bool,
}

pub struct MicroCode {
    pub memory_stage: Option<MemoryStage>,
    pub register_control_stage: Option<RegisterControl>,
    pub decoder_stage: Option<DecoderStage>,
    pub incrementer_stage: Option<IncrementerStage>,
    pub alu_stage: Option<AluStage>,
    pub post_alu_control_stage: Option<RegisterControl>,
    // Set to true if is the last microcode of the instruction.
    pub is_done: bool,
}

impl MicroCode {
    pub fn new() -> MicroCode {
        MicroCode {
            memory_stage: None,
            register_control_stage: None,
            decoder_stage: None,
            incrementer_stage: None,
            alu_stage: None,
            post_alu_control_stage: None,
            is_done: false,
        }
    }
}

impl MicroCode {
    pub fn execute(mut self, cpu: &mut Cpu, memory: &Memory) -> Result<Output> {
        // Step 1: Execute the memory operation if any.
        let memory_side_effect = self.memory_stage.and_then(|x| x.execute(cpu, memory));
        // Step 2: Register control.
        if let Some(register_control) = self.register_control_stage {
            register_control.execute(cpu);
        }
        // Step 3: Perform any ALU.
        if let Some(alu) = self.alu_stage {
            alu.execute(cpu);
        }
        // Step 4: Perform special decode stages (almost always an actual decode, sometimes an
        // early instruction end).
        let decoder_side_effect = if let Some(stage) = self.decoder_stage {
            debug_assert!(self.post_alu_control_stage.is_none());
            stage.execute(cpu, memory)
        } else {
            None
        };
        // Possibly increment counters.
        if let Some(incrementer_stage) = self.incrementer_stage {
            incrementer_stage.execute(cpu);
        }
        // Step 5: Do any post-alu register control.
        if let Some(stage) = self.post_alu_control_stage {
            stage.execute(cpu);
        }
        // If there are more micro-codes to execute, take away the is_done mark.
        if decoder_side_effect.is_some() {
            self.is_done = false;
        }
        // Return a decoder's output if it exists, otherwise just return the memory side effect.
        Ok(Output {
            side_effect: decoder_side_effect.or(memory_side_effect),
            is_done: self.is_done,
        })
    }
}

impl AluOp {
    fn execute(self, cpu: &Cpu) -> (Register, (i32, alu::Flags)) {
        let flags = alu::Flags::from_bits(cpu.registers.get(Register::F)).unwrap();
        match self {
            AluOp::BinaryOp { op, lhs, rhs } => (
                lhs,
                op.execute(cpu.registers.get(lhs), cpu.registers.get(rhs), flags),
            ),
            AluOp::UnaryOp { op, register } => {
                (register, op.execute(cpu.registers.get(register), flags))
            }
            AluOp::Move {
                destination,
                source,
            } => (destination, (cpu.registers.get(source), flags)),
        }
    }
}

impl FlagCondition {
    pub fn is_met(self, flags: i32) -> bool {
        alu::Flags::from_bits(flags)
            .unwrap()
            .contains(self.test_flags)
            == self.test_is_set
    }
}

impl AluStage {
    fn execute(self, cpu: &mut Cpu) {
        dbg!(self.flag_condition);
        let condition_met = if let Some(condition) = self.flag_condition {
            condition.is_met(cpu.registers.get(Register::F))
        } else {
            true
        };
        dbg!(condition_met);
        if condition_met {
            let (destination, (value, new_flags)) = self.op.execute(cpu);
            cpu.registers.set(destination, value);
            cpu.registers.set(Register::F, new_flags.bits());
        }
    }
}

impl DecoderStage {
    fn execute(self, cpu: &mut Cpu, memory: &Memory) -> Option<SideEffect> {
        match self {
            DecoderStage::Decode => decoder::execute_decode_stage(cpu, memory).unwrap(),
            DecoderStage::ConditionalDone(condition) => {
                if condition.is_met(cpu.registers.get(Register::F)) {
                    Some(SideEffect::DropRemainingMicroCodes)
                } else {
                    None
                }
            }
        }
    }
}

impl IncrementerStage {
    fn execute(self, cpu: &mut Cpu) {
        match self {
            IncrementerStage::Increment(register) => {
                debug_assert!(register.is_pair());
                cpu.registers.set(
                    register,
                    (cpu.registers.get(register) as u16).wrapping_add(1).into(),
                );
            }
            IncrementerStage::Decrement(register) => {
                debug_assert!(register.is_pair());
                cpu.registers.set(
                    register,
                    (cpu.registers.get(register) as u16).wrapping_sub(1).into(),
                )
            }
        }
    }
}

impl RegisterControl {
    fn execute(self, cpu: &mut Cpu) {
        match self {
            RegisterControl::Set(register, value) => cpu.registers.set(register, value),
            RegisterControl::Move(destination, source) => {
                debug_assert_eq!(destination.is_pair(), source.is_pair());
                println!(
                    "Setting {:?} by {:?} to {:X?}",
                    destination,
                    source,
                    cpu.registers.get(source)
                );
                cpu.registers.set(destination, cpu.registers.get(source))
            }
            RegisterControl::RestoreFlags { source, mask } => {
                RegisterControl::restore_flags(cpu, source, mask)
            }
            // RegisterControl::ConditionalMove {
            //     destination,
            //     source,
            //     flags,
            //     is_set,
            // } => RegisterControl::conditional_move(cpu, destination, source, flags, is_set),
            RegisterControl::SignExtend {
                destination,
                source,
            } => RegisterControl::sign_extend(cpu, destination, source),
        }
    }

    fn restore_flags(cpu: &mut Cpu, source: Register, mask: alu::Flags) {
        let current_flags = cpu.registers.get(Register::F);
        let backup_flags = cpu.registers.get(source);
        let mask_i32 = mask.bits();
        let new_flags = (backup_flags & mask_i32) | (current_flags & !mask_i32);
        debug_assert!(new_flags.leading_zeros() >= 24);
        debug_assert!(new_flags.trailing_zeros() >= 4);
        cpu.registers.set(Register::F, new_flags);
    }

    fn sign_extend(cpu: &mut Cpu, destination: Register, source: Register) {
        let source_value = cpu.registers.get(source);
        // Can also use conversion to not be so literal.
        let result = if (source_value & 0x80) != 0 {
            0xFF
        } else {
            0x00
        };
        cpu.registers.set(destination, result);
    }
}

impl MemoryStage {
    fn execute(self, cpu: &mut Cpu, memory: &Memory) -> Option<SideEffect> {
        match self {
            MemoryStage::Read {
                destination,
                address,
            } => {
                assert!(destination.is_single());
                assert!(address.is_pair());
                // MEMORY.
                // We let reads happen at any time since the writes are synchronized to happen
                // at the end of every machine cycle.
                cpu.registers
                    .set(destination, memory.read(cpu.registers.get(address)));
                None
            }
            MemoryStage::Write { address, value } => {
                assert!(address.is_pair());
                assert!(value.is_single());
                Some(SideEffect::Write {
                    raw_address: cpu.registers.get(address),
                    value: cpu.registers.get(value),
                })
            }
        }
    }
}
