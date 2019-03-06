pub mod control_unit;
pub mod decoder;
pub mod hl_decoder;
pub mod loader;

use super::register::Register;

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub struct MicroCode {
    mem_read_enable: bool,
    mem_write_enable: bool,
    mem_set_address: bool,
    mem_reg_address: Register,

    reg_select: Register,
    reg_write_enable: bool,

    alu_bus_select: Register,
    alu_bus_tmp_read: bool,
    alu_bus_tmp_read_mem: bool,
    alu_bus_f_reset: bool,
    alu_bus_f_read: bool,
    alu_bus_f_write: u8,
    alu_act_read: bool,
    alu_op: AluOp,
    alu_f_force_carry: bool,
    alu_write: bool,

    inc_op: IncOp,
    inc_skip_latch: bool,
    inc_write: bool,
    inc_dest: Register,

    is_end: bool,
    is_cond_end: bool,
    is_halt: bool,
    is_stop: bool,

    enable_interrupts: bool,
    disable_interrupts: bool,
}

impl Default for MicroCode {
    fn default() -> MicroCode {
        MicroCode {
            mem_read_enable: false,
            mem_write_enable: false,
            mem_set_address: false,
            mem_reg_address: Register::PC,

            reg_select: Register::B,
            reg_write_enable: false,

            alu_bus_select: Register::B,
            alu_bus_tmp_read: false,
            alu_bus_tmp_read_mem: false,
            alu_bus_f_reset: false,
            alu_bus_f_read: false,
            alu_bus_f_write: 0,
            alu_act_read: false,
            alu_op: AluOp::Mov,
            alu_f_force_carry: false,
            alu_write: false,

            inc_op: IncOp::Mov,
            inc_skip_latch: false,
            inc_write: false,
            inc_dest: Register::B,

            is_end: false,
            is_cond_end: false,
            is_halt: false,
            is_stop: false,

            enable_interrupts: false,
            disable_interrupts: false,
        }
    }
}
