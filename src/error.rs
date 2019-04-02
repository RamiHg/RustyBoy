use core::fmt;

use crate::memory;

pub enum Type {
    InvalidOperation(String),
    InvalidOpcode(i32),
    Memory(memory::MemoryError),
}

pub type Result<T> = core::result::Result<T, Type>;

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { <Type as fmt::Display>::fmt(self, f) }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::InvalidOpcode(op) => write!(f, "Invalid opcode: 0x{:X?}.", op),
            _ => write!(f, "Buzz off"),
        }
    }
}
