use num_derive::FromPrimitive;

use crate::cpu::alu;
use crate::cpu::register::Register;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IncOp {
    Mov = 0b00,
    Inc = 0b01,
    Dec = 0b10,
}

impl Default for IncOp {
    fn default() -> Self { IncOp::Mov }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AluOutSelect {
    Result,
    Tmp,
    A,
    ACT,
    F,
}

#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive)]
pub enum Condition {
    NZ,
    Z,
    NC,
    C,
}

impl Default for Condition {
    fn default() -> Self { Condition::NZ }
}

impl Default for AluOutSelect {
    fn default() -> Self { AluOutSelect::Result }
}

/// This microcode format is nowhere near size-optimized. There are tons of mutually exclusive bits,
/// and it could probably be cut down in half.
#[derive(Copy, Clone, Debug, Default)]
pub struct MicroCode {
    // These two flags control the RD and WR signal registers on the memory bus. Alone, they do not
    //  do much other than signal to the memory controller intent.
    pub mem_read_enable: bool,
    pub mem_write_enable: bool,

    // Register control.
    pub reg_select: Register,
    pub reg_write_enable: bool,
    pub reg_to_data: bool,

    // Address control.
    /// If true, will drive the address bus from the register file, and more importantly, write
    /// into the address buffer register.
    pub reg_to_addr_buffer: bool,
    pub ff_to_addr_hi: bool,
    pub addr_select: Register,
    /// If true, will overwrite the selected address register with the value in the address bus.
    pub addr_write_enable: bool,

    // Incrementer control.
    pub inc_op: IncOp,
    /// Drives the address bus with the result of the incrementer.
    pub inc_to_addr_bus: bool,

    // Alu control.
    pub alu_op: alu::Op,
    pub alu_out_select: AluOutSelect,
    pub alu_to_data: bool,
    /// Overwrites the selected ALU register with the value in the data bus (or a constant).
    pub alu_reg_write_enable: bool,
    pub alu_a_to_act: bool,
    pub alu_opymul8_to_act: bool,
    pub alu_a_to_tmp: bool,
    pub alu_zero_to_tmp: bool,
    pub alu_one_to_tmp: bool,
    pub alu_cse_to_tmp: bool,
    pub alu_64_to_tmp: bool,
    pub alu_f_force_nz: bool,
    pub alu_write_f_mask: u8,
    pub alu_bit_select: u8,

    // Control flow.
    pub is_end: bool,
    pub is_cond_end: bool,
    pub is_halt: bool,
    pub cond: Condition,
    pub enter_cb_mode: bool,

    // Interrupts.
    pub enable_interrupts: bool,
    pub disable_interrupts: bool,
}
