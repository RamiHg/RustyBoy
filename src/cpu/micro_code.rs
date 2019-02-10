use crate::cpu::decoder;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Result};
use crate::memory::Memory;

use super::alu;
use alu::{dec_u16, inc_u16};

pub enum SideEffect {
    Write { raw_address: i32, value: i32 },
    Decode(Vec<MicroCode>),
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
///     a) IncrementerStage: Simple 16-bit incrementer. Probably exists in real hardware.
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

pub enum AluStage {
    BinaryOp {
        op: alu::BinaryOp,
        lhs: Register,
        rhs: Register,
    },
    UnaryOp {
        op: alu::UnaryOp,
        register: Register,
    },
    SignExtend {
        destination: Register,
        source: Register,
    },
}

pub enum RegisterControl {
    Set(Register, i32),
    Move(Register, Register),
    RestoreFlags {
        source: Register,
        mask: alu::FlagRegister,
    },
}

#[derive(Debug)]
pub enum IncrementerStage {
    PC,
    HLI,
    HLD,
    TEMP,
}

pub struct MicroCode {
    pub memory_stage: Option<MemoryStage>,
    pub register_control_stage: Option<RegisterControl>,
    pub has_decode_stage: bool,
    pub incrementer_stage: Option<IncrementerStage>,
    pub alu_stage: Option<AluStage>,
    pub post_alu_control_stage: Option<RegisterControl>,
    // Set to true if is the last microcode of the instruction.
    pub is_done: bool,
}

pub struct Builder {
    current_code: MicroCode,
    codes: Vec<MicroCode>,
}

impl MicroCode {
    pub fn new() -> MicroCode {
        MicroCode {
            memory_stage: None,
            register_control_stage: None,
            has_decode_stage: false,
            incrementer_stage: None,
            alu_stage: None,
            post_alu_control_stage: None,
            is_done: false,
        }
    }
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            current_code: MicroCode::new(),
            codes: Vec::new(),
        }
    }

    pub fn nothing_then(mut self) -> Builder {
        self.current_code = MicroCode::new();
        self.then()
    }

    pub fn then(mut self) -> Builder {
        self.codes.push(self.current_code);
        self.current_code = MicroCode::new();
        self
    }

    pub fn then_done(mut self) -> Vec<MicroCode> {
        self.current_code.is_done = true;
        self.codes.push(self.current_code);
        self.codes
    }

    // ALU.
    pub fn binary_op(mut self, op: alu::BinaryOp, lhs: Register, rhs: Register) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_none());
        self.current_code.alu_stage = Some(AluStage::BinaryOp { op, lhs, rhs });
        self
    }

    pub fn unary_op(mut self, op: alu::UnaryOp, register: Register) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_none());
        self.current_code.alu_stage = Some(AluStage::UnaryOp { op, register });
        self
    }

    pub fn sign_extend(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_none());
        self.current_code.alu_stage = Some(AluStage::SignExtend {
            destination,
            source,
        });
        self
    }

    // (Pre-ALU) Register control.
    pub fn move_reg(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::Move(destination, source));
        self
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::Set(register, value));
        self
    }

    // (Post-ALU) Register control.
    pub fn post_alu_move_reg(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.post_alu_control_stage.is_none());
        self.current_code.post_alu_control_stage = Some(RegisterControl::Move(destination, source));
        self
    }

    // Memory.

    pub fn read_mem(mut self, destination: Register, address: Register) -> Builder {
        debug_assert!(self.current_code.memory_stage.is_none());
        self.current_code.memory_stage = Some(MemoryStage::Read {
            destination,
            address,
        });
        self
    }

    pub fn write_mem(mut self, address: Register, value: Register) -> Builder {
        debug_assert!(self.current_code.memory_stage.is_none());
        self.current_code.memory_stage = Some(MemoryStage::Write { address, value });
        self
    }

    pub fn maybe_increment(mut self, increment: Option<IncrementerStage>) -> Builder {
        debug_assert!(self.current_code.incrementer_stage.is_none());
        self.current_code.incrementer_stage = increment;
        self
    }

    pub fn increment(self, increment: IncrementerStage) -> Builder {
        debug_assert!(self.current_code.incrementer_stage.is_none());
        self.maybe_increment(Some(increment))
    }

    pub fn decode() -> Vec<MicroCode> {
        Builder::new()
            .read_mem(Register::TEMP_LOW, Register::PC)
            .has_decode_stage()
            .increment(IncrementerStage::PC)
            .then_done()
    }

    // Misc stages.
    pub fn post_alu_restore_flags(mut self, source: Register, mask: alu::FlagRegister) -> Builder {
        debug_assert!(self.current_code.post_alu_control_stage.is_none());
        self.current_code.post_alu_control_stage =
            Some(RegisterControl::RestoreFlags { source, mask });
        self
    }

    fn has_decode_stage(mut self) -> Builder {
        self.current_code.has_decode_stage = true;
        self
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
        // Step 3: Perform a decode (if requested).
        let decoder_side_effect = if self.has_decode_stage {
            debug_assert!(self.alu_stage.is_none());
            debug_assert!(self.post_alu_control_stage.is_none());
            decoder::execute_decode_stage(cpu, memory)?
        } else {
            None
        };
        // Step 4: Perform any ALU.
        if let Some(alu) = self.alu_stage {
            alu.execute(cpu);
        }
        // Possibly increment counters.
        if let Some(incrementer_stage) = self.incrementer_stage {
            match incrementer_stage {
                IncrementerStage::PC => cpu
                    .registers
                    .set(Register::PC, inc_u16(cpu.registers.get(Register::PC))),
                IncrementerStage::HLI => cpu
                    .registers
                    .set(Register::HL, inc_u16(cpu.registers.get(Register::HL))),
                IncrementerStage::HLD => cpu
                    .registers
                    .set(Register::HL, dec_u16(cpu.registers.get(Register::HL))),
                IncrementerStage::TEMP => cpu
                    .registers
                    .set(Register::TEMP, inc_u16(cpu.registers.get(Register::TEMP))),
            }
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

impl AluStage {
    fn execute(self, cpu: &mut Cpu) {
        let flags = alu::FlagRegister(cpu.registers.get(Register::F) as u32);
        match self {
            AluStage::BinaryOp { op, lhs, rhs } => {
                let (result, new_flags) =
                    op.execute(cpu.registers.get(lhs), cpu.registers.get(rhs), &flags);
                cpu.registers.set(lhs, result);
                cpu.registers.set(Register::F, new_flags.0 as i32);
            }
            AluStage::UnaryOp { op, register } => {
                let (result, new_flags) = op.execute(cpu.registers.get(register), &flags);
                cpu.registers.set(register, result);
                cpu.registers.set(Register::F, new_flags.0 as i32);
            }
            AluStage::SignExtend {
                destination,
                source,
            } => {
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
    }
}

impl RegisterControl {
    fn execute(self, cpu: &mut Cpu) {
        match self {
            RegisterControl::Set(register, value) => cpu.registers.set(register, value),
            RegisterControl::Move(destination, source) => {
                cpu.registers.set(destination, cpu.registers.get(source))
            }
            RegisterControl::RestoreFlags { source, mask } => {
                RegisterControl::restore_flags(cpu, source, mask)
            }
        }
    }

    fn restore_flags(cpu: &mut Cpu, source: Register, mask: alu::FlagRegister) {
        let current_flags = cpu.registers.get(Register::F);
        let backup_flags = cpu.registers.get(source);
        let mask_i32 = mask.0 as i32;
        let new_flags = (backup_flags & mask_i32) | (current_flags & !mask_i32);
        debug_assert!(new_flags.leading_zeros() >= 24);
        debug_assert!(new_flags.trailing_zeros() >= 4);
        cpu.registers.set(Register::F, new_flags);
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
