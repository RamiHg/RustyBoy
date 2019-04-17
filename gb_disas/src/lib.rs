pub mod decode;
pub mod display;
pub(crate) mod op_creation;

// Import all the core::fmt::Display trait implementations into the public scope.
pub use display::*;

pub enum Arg {
    Register(&'static str),
    Signed8bit(i8),
    Unsigned8bit(u8),
    Unsigned16bit(u16),
    Condition(&'static str),
    IndirectRef(Box<Arg>),
    FFPlus(Box<Arg>),
    SPPlus(Box<Arg>),
}

pub struct Command {
    pub name: &'static str,
}

pub struct Op {
    pub command: Command,
    pub lhs: Option<Arg>,
    pub rhs: Option<Arg>,
    pub byte_size: u8,
}
