use crate::alu::{dec_u16, inc_u16};
use crate::cpu::decoder;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Result};
use crate::memory::Memory;

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

#[derive(Debug)]
pub enum AluStage {
    Move(Register, Register),
}

#[derive(Debug)]
pub enum RegisterControl {
    Set(Register, i32),
}

#[derive(Debug)]
pub enum IncrementerStage {
    PC,
    HLI,
    HLD,
    TEMP,
}

#[derive(Debug)]
pub enum SpecialStage {
    Decode,
}

#[derive(Debug)]
pub struct MicroCode {
    pub memory_stage: Option<MemoryStage>,
    pub register_control_stage: Option<RegisterControl>,
    pub alu_stage: Option<AluStage>,
    pub special_stage: Option<SpecialStage>,
    pub incrementer_stage: Option<IncrementerStage>,
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
            alu_stage: None,
            special_stage: None,
            incrementer_stage: None,
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

    pub fn move_reg(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_none());
        self.current_code.alu_stage = Some(AluStage::Move(destination, source));
        self
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::Set(register, value));
        self
    }

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
            .special_stage(SpecialStage::Decode)
            .increment(IncrementerStage::PC)
            .then_done()
    }

    fn special_stage(mut self, special: SpecialStage) -> Builder {
        debug_assert!(self.current_code.special_stage.is_none());
        self.current_code.special_stage = Some(special);
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
        // Step 3: Perform any ALU.
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
        // Last step: Execute the "special" stage. Right now that's decoding.
        let decoder_side_effect = if let Some(SpecialStage::Decode) = self.special_stage {
            debug_assert!(memory_side_effect.is_none());
            decoder::execute_decode_stage(cpu, memory)?
        } else {
            None
        };

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
        match self {
            AluStage::Move(destination, source) => {
                cpu.registers.set(destination, cpu.registers.get(source))
            }
        }
    }
}

impl RegisterControl {
    fn execute(self, cpu: &mut Cpu) {
        match self {
            RegisterControl::Set(register, value) => cpu.registers.set(register, value),
        }
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
