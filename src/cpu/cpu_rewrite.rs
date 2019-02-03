use core::fmt;

use num_traits::FromPrimitive;

use crate::cpu::instructions::*;
use crate::cpu::register;
use crate::memory::{Memory, MemoryError};

use register::{Register, SingleTable};

// #[derive(Debug)]
pub enum Error {
    InvalidOperation(String),
    InvalidOpcode(i32),
    Memory(MemoryError),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Error as fmt::Display>::fmt(self, f)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::InvalidOpcode(op) => write!(f, "Invalid opcode: 0x{:X?}.", op),
            _ => write!(f, "Buzz off"),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub struct Cpu {
    pub registers: register::File,
    instruction: Option<InstructionType>,
    instruction_mcycle: i32,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            registers: register::File::new([0; 12]),
            instruction: None,
            instruction_mcycle: 0,
        }
    }

    /// Runs a machine cycle.
    ///
    /// Optionally returns a data bus write request.
    pub fn execute_machine_cycle(&mut self, memory: &Memory) -> Result<InstrResult> {
        // Decode an instruction if we need to.
        if self.instruction.is_none() {
            self.instruction = Some(self.decode_instruction(memory)?);
            self.instruction_mcycle = 0;
        }
        let mut instruction = self.instruction.take().unwrap();
        let result = instruction.as_mut_instruction().execute_cycle(
            self.instruction_mcycle,
            self,
            memory,
        )?;
        match result {
            InstrResult::Done => (),
            _ => {
                // The instruction is not done yet. Move it back into the cpu.
                self.instruction = instruction.into();
                self.instruction_mcycle += 1;
            }
        }
        Ok(result)
    }

    fn decode_instruction(&self, memory: &Memory) -> Result<InstructionType> {
        // This method uses the amazing guide to decode instructions programatically:
        // https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html

        let opcode = memory.read(self.registers.get(Register::PC));
        // Deconstruct the op into its components.
        let op_z = opcode & 0b00000111;
        let op_y = (opcode & 0b00111000) >> 3;
        let op_x = (opcode & 0b11000000) >> 6;
        let op_q = op_y & 0b001;
        let op_p = (op_y & 0b110) >> 1;

        let err = Error::InvalidOpcode(opcode);

        // Validating for documentation things that are tautologies.
        debug_assert!(op_p <= 3);

        //#[allow(unreachable_patterns)]
        match op_x {
            // x = 0
            0 => match op_z {
                // z = 2
                2 => match op_q {
                    // q = 1
                    1 => match op_p {
                        0 => Ok(IndirectLoad::new(SingleTable::A, Register::BC, HLOp::None).into()),
                        1 => Ok(IndirectLoad::new(SingleTable::A, Register::DE, HLOp::None).into()),
                        2 => Ok(IndirectLoad::new(SingleTable::A, Register::HL, HLOp::Inc).into()),
                        3 | _ => {
                            Ok(IndirectLoad::new(SingleTable::A, Register::HL, HLOp::Dec).into())
                        }
                    },
                    _ => Err(err),
                },
                _ => Err(err),
            },
            // x = 1
            1 => match op_z {
                6 => match op_y {
                    6 => Err(err),
                    dest_reg => Ok(IndirectLoad::new(
                        SingleTable::from_i32(dest_reg).unwrap(),
                        Register::HL,
                        HLOp::None,
                    )
                    .into()),
                },
                _ => Err(err),
            },
            _ => Err(err),
        }
    }
}
