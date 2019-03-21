pub mod asm;
pub mod asm_compiler;
pub mod asm_parser;
// pub mod control_unit;
pub mod csv_parser;
pub mod decoder;
// pub mod hl_decoder;
pub mod loader;
pub mod micro_code;
pub mod op_map;

use super::register::Register;

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

#[derive(Debug, Copy, Clone)]
pub struct MicroCode {
    mem_read_enable: bool,
    mem_write_enable: bool,
    mem_set_address: bool,
    mem_reg_address: Register,

    reg_select: Register,
    reg_write_enable: bool,

    alu_mem_to_a: bool,
    alu_mem_to_act: bool,
    alu_mem_to_tmp: bool,
    alu_mem_to_f: bool,
    alu_mem_to_tmp_f: bool,
    alu_tmp_f_to_f: bool,
    alu_0_to_f: bool,
    alu_force_carry: bool,
    alu_a_to_act: bool,
    alu_result_to_a: bool,
    alu_op: AluOp,

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

            alu_mem_to_a: false,
            alu_mem_to_act: false,
            alu_mem_to_tmp: false,
            alu_mem_to_f: false,
            alu_mem_to_tmp_f: false,
            alu_tmp_f_to_f: false,
            alu_0_to_f: false,
            alu_force_carry: false,
            alu_a_to_act: false,
            alu_result_to_a: false,
            alu_op: AluOp::Mov,

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
