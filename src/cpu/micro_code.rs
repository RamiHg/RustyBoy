use crate::alu::{dec_u16, inc_u16};
use crate::cpu::decoder;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Result};
use crate::memory::Memory;
use crate::mmu;

#[derive(PartialEq)]
pub enum InstrResult {
    None,
    Write(mmu::Write),
    Decode(Vec<MicroCode>),
    Done,
}

#[derive(PartialEq, Debug)]
pub enum MemoryStage {
    None,
    ReadMem(ReadMem),
}
#[derive(PartialEq, Debug)]
pub enum DirectValue {
    Register(Register),
    Imm8(i32),
}

#[derive(PartialEq, Debug)]
pub enum AluStage {
    Move(Register, Register),
}

#[derive(PartialEq, Debug)]
pub enum IncrementerStage {
    PC,
    HLI,
    HLD,
}

#[derive(Debug, PartialEq)]
pub enum SpecialStage {
    None,
    Decode,
}

#[derive(Debug, PartialEq)]
pub struct MicroCode {
    pub memory_stage: MemoryStage,
    pub alu_stage: Option<AluStage>,
    pub special_stage: SpecialStage,
    pub incrementer_stage: Option<IncrementerStage>,
    // Set to true if is the last microcode of the instruction.
    pub done: bool,
}

pub struct Builder {
    current_code: MicroCode,
    codes: Vec<MicroCode>,
}

impl MicroCode {
    pub fn new() -> MicroCode {
        MicroCode {
            memory_stage: MemoryStage::None,
            alu_stage: None,
            special_stage: SpecialStage::None,
            incrementer_stage: None,
            done: false,
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
        self.current_code.done = true;
        self.codes.push(self.current_code);
        self.codes
    }

    pub fn move_reg(mut self, destination: Register, source: Register) -> Builder {
        self.current_code.alu_stage = Some(AluStage::Move(destination, source));
        self
    }

    pub fn read_mem(mut self, destination: Register, address: Register) -> Builder {
        self.current_code.memory_stage = MemoryStage::ReadMem(ReadMem {
            destination,
            address,
        });
        self
    }

    pub fn increment(mut self, increment: IncrementerStage) -> Builder {
        self.current_code.incrementer_stage = Some(increment);
        self
    }

    pub fn decode() -> Vec<MicroCode> {
        Builder::new()
            .read_mem(Register::TEMP_LOW, Register::PC)
            .special_stage(SpecialStage::Decode)
            .increment(IncrementerStage::PC)
            .then_done()
    }

    fn special_stage(mut self, special: SpecialStage) -> Builder {
        self.current_code.special_stage = special;
        self
    }
}

impl MicroCode {
    pub fn execute(self, cpu: &mut Cpu, memory: &Memory) -> Result<InstrResult> {
        // Step 1: Execute the memory operation.
        let memory_result = match self.memory_stage {
            MemoryStage::ReadMem(read) => read.execute(cpu, memory)?,
            _ => InstrResult::None,
        };
        // Step 2: Perform any ALU.
        if let Some(alu) = self.alu_stage {
            alu.execute(cpu);
        }
        // Possibly increment counters.
        if let Some(incrementer) = self.incrementer_stage {
            match incrementer {
                IncrementerStage::PC => cpu
                    .registers
                    .set(Register::PC, inc_u16(cpu.registers.get(Register::PC))),
                IncrementerStage::HLI => cpu
                    .registers
                    .set(Register::HL, inc_u16(cpu.registers.get(Register::HL))),
                IncrementerStage::HLD => cpu
                    .registers
                    .set(Register::HL, dec_u16(cpu.registers.get(Register::HL))),
            }
        }
        // Last step: Execute the "special" stage. Right now that's decoding.
        if let SpecialStage::Decode = self.special_stage {
            // Early exit if is decoder step.
            return decoder::execute(cpu, memory);
        }
        // Return either the memory request or the Done result.
        if self.done {
            assert!(memory_result == InstrResult::None);
            Ok(InstrResult::Done)
        } else {
            Ok(memory_result)
        }
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ReadMem {
    destination: Register,
    address: Register,
}

impl ReadMem {
    fn execute(self, cpu: &mut Cpu, memory: &Memory) -> Result<InstrResult> {
        assert!(self.destination.is_single());
        assert!(self.address.is_pair());
        // MEMORY.
        let memory_value = memory.read(cpu.registers.get(self.address));
        cpu.registers.set(self.destination, memory_value);
        Ok(InstrResult::None)
    }
}
