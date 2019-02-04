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

pub enum ValueType {
    Register(Register),
    IndirectRegister(Register),
    Imm8(i32),
    Imm16(i32),
    IndirectImm16(i32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MemoryStage {
    None,
    ReadMem(ReadMem),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpecialStage {
    None,
    Decode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MicroCode {
    pub memory_stage: MemoryStage,
    pub special_stage: SpecialStage,
    pub inc_pc: bool,
}

pub struct Builder {
    current_code: MicroCode,
    codes: Vec<MicroCode>,
}

impl MicroCode {
    pub fn new() -> MicroCode {
        MicroCode {
            memory_stage: MemoryStage::None,
            special_stage: SpecialStage::None,
            inc_pc: false,
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

    pub fn then_done(mut self) -> Vec<MicroCode> {
        self.codes.push(self.current_code);
        self.codes
    }

    pub fn then(mut self) -> Builder {
        self.codes.push(self.current_code);
        self.current_code = MicroCode::new();
        self
    }

    pub fn read_mem(mut self, destination: Register, address: Register) -> Builder {
        self.current_code.memory_stage = MemoryStage::ReadMem(ReadMem {
            destination,
            address,
        });
        self
    }

    pub fn decode(self) -> Builder {
        Builder {
            current_code: MicroCode {
                special_stage: SpecialStage::Decode,
                inc_pc: true,
                ..self.read_mem(Register::TEMP_LOW, Register::PC).current_code
            },
            codes: Vec::new(),
        }
    }
}

impl MicroCode {
    pub fn execute(self, cpu: &mut Cpu, memory: &Memory) -> Result<InstrResult> {
        // Step 1: Execute the memory operation.
        let memory_result = match self.memory_stage {
            MemoryStage::ReadMem(read) => read.execute(cpu, memory)?,
            _ => InstrResult::None,
        };
        // Last step: Execute the "special" stage. Right now that's decoding.
        if let SpecialStage::Decode = self.special_stage {
            return decoder::execute(cpu, memory);
        }
        if self.inc_pc {
            let pc = (cpu.registers.get(Register::PC) as u16).wrapping_add(1);
            cpu.registers.set(Register::PC, pc as i32);
        }
        Ok(memory_result)
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
