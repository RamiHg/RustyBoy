pub mod asm;
pub mod asm_compiler;
pub mod asm_parser;
pub mod control_unit;
pub mod csv_parser;
pub mod decoder;
pub mod micro_code;
pub mod op_map;

use super::register::Register;

pub use micro_code::MicroCode;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AluOp {
    Mov,
    Add,
    Addc,
    Sub,
    Subc,
    And,
    Xor,
    Or,
    Cp,
    Cpl,
    Daa,
}

#[derive(Debug, Clone, Copy)]
pub enum IncOp {
    Mov = 0,
    Inc,
    Dec,
}
