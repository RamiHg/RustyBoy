use crate::cpu;
use crate::memory::Memory;

use cpu::alu::Flags;
use cpu::register::{self, Register};
use cpu::{Cpu, DecodeMode};

use super::decoder;
use super::micro_code::{AluOp, AluOutSelect, IncOp, MicroCode};

fn fetch_t1() -> MicroCode {
    MicroCode {
        // We set mem_read_enable to true explicitly even though we always reset at T4.
        mem_read_enable: true,
        reg_to_addr_buffer: true,
        addr_select: Register::PC,
        ..Default::default()
    }
}

fn fetch_t2() -> MicroCode {
    MicroCode {
        ..Default::default()
    }
}

pub fn cycle(cpu: &mut Cpu, memory: &Memory) {
    dbg!(cpu.state);
    let (micro_code, mut next_state) = match cpu.state.decode_mode {
        DecodeMode::Fetch => match cpu.state.t_state.get() {
            1 => (fetch_t1(), DecodeMode::Fetch),
            2 => (fetch_t2(), DecodeMode::Decode),
            _ => panic!("Invalid fetch t-state"),
        },
        DecodeMode::Decode => match cpu.state.t_state.get() {
            3 => {
                let opcode = cpu.state.data_latch;
                cpu.micro_code_v2_stack = cpu.decoder.decode(opcode, memory);
                (cpu.micro_code_v2_stack.remove(0), DecodeMode::Execute)
            }
            _ => panic!("Invalid decode t-state"),
        },
        DecodeMode::Execute => (cpu.micro_code_v2_stack.remove(0), DecodeMode::Execute),
    };
    // Execute the micro-code.
    execute(&micro_code, cpu, memory);
    if micro_code.is_end {
        assert_eq!(cpu.state.t_state.get(), 4);
        next_state = DecodeMode::Fetch;
    }
    cpu.state.decode_mode = next_state;
}

/// Incrementer module.

fn incrementer_logic(code: &MicroCode, cpu: &Cpu, current_regs: &register::File) -> i32 {
    let source_value = cpu.state.address_latch;
    match code.inc_op {
        IncOp::Mov => source_value,
        IncOp::Inc => (source_value + 1) & 0xFFFF,
        IncOp::Dec => (source_value - 1) & 0xFFFF,
    }
}

fn alu_logic(
    code: &MicroCode,
    current_regs: &register::File,
    new_regs: &mut register::File,
) -> i32 {
    let act = current_regs.get(Register::ACT);
    let tmp = current_regs.get(Register::ALU_TMP);
    use AluOp::*;
    let (result, flags) = match code.alu_op {
        Mov => (act, Flags::empty()),
        _ => panic!("Implement {:?}", code.alu_op),
    };
    let current_flags = current_regs.get(Register::F);
    let flag_mask = (code.alu_write_f_mask << 4) as i32;
    let new_flags = (current_flags & !flag_mask) | (flags.bits() & flag_mask);
    new_regs.set(Register::F, new_flags);
    match code.alu_out_select {
        AluOutSelect::Result => result,
        AluOutSelect::Tmp => tmp,
        AluOutSelect::A => current_regs.get(Register::A),
        AluOutSelect::ACT => act,
        AluOutSelect::F => current_flags,
    }
}

fn alu_reg_write(code: &MicroCode, data: i32, new_regs: &mut register::File) {
    match code.alu_out_select {
        AluOutSelect::Tmp => new_regs.set(Register::ALU_TMP, data),
        AluOutSelect::A => new_regs.set(Register::A, data),
        AluOutSelect::ACT => new_regs.set(Register::ACT, data),
        AluOutSelect::F => new_regs.set(Register::F, data),
        _ => panic!("Invalid AluOutSelect {:?}", code.alu_out_select),
    };
}

fn execute(code: &MicroCode, cpu: &mut Cpu, memory: &Memory) {
    dbg!(code);

    let current_regs = cpu.registers;
    let mut new_regs = current_regs;

    let mut next_state = cpu.state;

    if code.mem_read_enable {
        next_state.read_latch = true;
        next_state.write_latch = false;
    }

    if code.mem_write_enable {
        next_state.read_latch = false;
        next_state.write_latch = true;
    }

    if code.reg_to_addr_buffer {
        debug_assert!(!code.inc_to_addr_bus);
        debug_assert!(!code.addr_write_enable);
        next_state.address_latch = current_regs.get(code.addr_select);
    }

    let addr_bus_value = if code.inc_to_addr_bus {
        incrementer_logic(code, cpu, &current_regs)
    } else {
        -1
    };

    if code.addr_write_enable {
        new_regs.set(code.addr_select, addr_bus_value);
    }

    let data_bus_value = if code.alu_to_data {
        alu_logic(code, &current_regs, &mut new_regs)
    } else if code.reg_to_data {
        debug_assert!(!code.reg_write_enable);
        current_regs.get(code.reg_select)
    } else {
        cpu.state.data_latch
    };

    if code.reg_write_enable {
        new_regs.set(code.reg_select, data_bus_value);
    }

    if code.alu_reg_write_enable {
        alu_reg_write(code, data_bus_value, &mut new_regs);
    }

    //let alu_result = alu_logic(code, cpu, &current_regs, &mut new_regs);

    // Finally, copy over the new state.
    cpu.registers = new_regs;
    cpu.state = next_state;
}
