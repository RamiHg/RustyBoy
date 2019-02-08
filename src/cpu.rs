
mod decoder;
mod micro_code;
mod register;
mod alu;

#[cfg(test)]
mod test;

// TODO: Place this in a better place..
pub use self::micro_code::{Output, SideEffect};

use core::fmt;

use crate::cpu::micro_code::{Builder, MicroCode};
use crate::memory::{Memory, MemoryError};

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
    pub fn execute_machine_cycle(&mut self, memory: &Memory) -> Result<Output> {
        if self.micro_code_stack.is_empty() {
            // Run the decoder to get a bunch of microcodes.
            self.micro_code_stack = Builder::decode();
        }
        let top = self.micro_code_stack.remove(0);
        let micro_code_output = top.execute(self, memory)?;

        // If this is a decode micro-code, push the codes on the stack and return.
        if let Some(SideEffect::Decode(instructions)) = micro_code_output.side_effect {
            assert!(!micro_code_output.is_done);
            self.micro_code_stack = instructions;
            return Ok(Output {
                side_effect: None,
                ..micro_code_output
            });
        }
        // Otherise, return as normal.
        Ok(micro_code_output)
    }
}
