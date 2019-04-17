use super::{Arg, Command, Op};
use core::fmt::{Display, Formatter, Result};

impl Display for Arg {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Arg::Register(name) => write!(f, "{}", name),
            Arg::Signed8bit(value) => write!(f, "{}", value),
            Arg::Unsigned8bit(value) => write!(f, "0x{:X?}", value),
            Arg::Unsigned16bit(value) => write!(f, "0x{:X?}", value),
            Arg::Condition(value) => write!(f, "{}", value),
            Arg::IndirectRef(arg) => write!(f, "({})", arg),
            Arg::FFPlus(arg) => write!(f, "0xFF00 + {}", arg),
            Arg::SPPlus(arg) => write!(f, "SP + {}", arg),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> Result { write!(f, "{}", self.name) }
}

impl Display for Op {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.command)?;
        if let Some(lhs) = &self.lhs {
            write!(f, " {}", lhs)?;
        }
        if let Some(rhs) = &self.rhs {
            write!(f, ", {}", rhs)?;
        }
        Ok(())
    }
}
