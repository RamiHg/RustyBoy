use core::fmt;

//use num_traits::FromPrimitive;

use crate::cpu::register;
use crate::memory::{Memory, MemoryError};

use crate::cpu::micro_code::{Builder, InstrResult, MicroCode};

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
    micro_code_stack: Vec<MicroCode>,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            registers: register::File::new([0; 14]),
            micro_code_stack: Vec::new(),
        }
    }

    /// Runs a machine cycle.
    ///
    /// Optionally returns a data bus write request.
    pub fn execute_machine_cycle(&mut self, memory: &Memory) -> Result<InstrResult> {
        if self.micro_code_stack.is_empty() {
            // Run the decoder to get a bunch of microcodes.
            self.micro_code_stack = Builder::decode();
        }
        let top = self.micro_code_stack.remove(0);
        match top.execute(self, memory)? {
            InstrResult::Decode(instructions) => {
                self.micro_code_stack = instructions;
                Ok(InstrResult::None)
            }
            result => Ok(result),
        }
    }
}
