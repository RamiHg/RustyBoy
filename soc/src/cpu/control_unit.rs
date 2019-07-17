use crate::cpu;
use crate::util;

use cpu::alu;
use cpu::alu::Flags;
use cpu::register::{self, Register};
use cpu::{Cpu, DecodeMode};

use super::micro_code::{AluOutSelect, Condition, IncOp, MicroCode};

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
    MicroCode { ..Default::default() }
}

fn true_nop() -> MicroCode {
    MicroCode { ..Default::default() }
}

fn nop_end() -> MicroCode {
    MicroCode { is_end: true, ..Default::default() }
}

pub fn cycle(cpu: &mut Cpu) -> (cpu::State, bool) {
    let (micro_code, mut next_mode) = match cpu.state.decode_mode {
        DecodeMode::Fetch => match cpu.t_state.get() {
            1 => (fetch_t1(), DecodeMode::Fetch),
            2 => (fetch_t2(), DecodeMode::Decode),
            _ => panic!("Invalid fetch t-state"),
        },
        DecodeMode::Decode => match cpu.t_state.get() {
            3 => {
                let opcode = cpu.state.data_latch;
                // TODO: Clean up
                if cpu.is_halted {
                    debug_assert!(cpu.micro_code_stack.is_empty());
                    cpu.micro_code_stack.clear();
                    cpu.micro_code_stack.push_back(true_nop()).unwrap();
                    cpu.micro_code_stack.push_back(nop_end()).unwrap();
                } else if !cpu.is_handling_interrupt {
                    debug_assert!(cpu.micro_code_stack.is_empty());
                    cpu.registers.set(Register::INSTR, opcode);
                    cpu.micro_code_stack = cpu.decoder.decode(opcode, cpu.state.in_cb_mode);
                }
                (cpu.micro_code_stack.pop_front().unwrap(), DecodeMode::Execute)
            }
            _ => panic!("Invalid decode t-state"),
        },
        DecodeMode::Execute => (cpu.micro_code_stack.pop_front().unwrap(), DecodeMode::Execute),
    };
    // Execute the micro-code.
    let mut next_state = execute(&micro_code, cpu);
    let is_end = if micro_code.is_cond_end {
        let flags = alu::Flags::from_bits(cpu.registers.get(Register::F)).unwrap();
        let end = !condition_check_passes(flags, micro_code.cond);
        if end {
            cpu.micro_code_stack.clear();
        };
        end
    } else {
        micro_code.is_end
    };
    if micro_code.enter_cb_mode {
        next_state.in_cb_mode = true;
    } else if is_end {
        next_state.in_cb_mode = false;
    }
    if is_end || micro_code.enter_cb_mode {
        assert_eq!(cpu.t_state.get(), 4);
        next_mode = DecodeMode::Fetch;
    }
    next_state.decode_mode = next_mode;
    (next_state, is_end)
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

fn condition_check_passes(flags: alu::Flags, cond: Condition) -> bool {
    match cond {
        Condition::NZ => !flags.intersects(alu::Flags::ZERO),
        Condition::Z => flags.intersects(alu::Flags::ZERO),
        Condition::NC => !flags.intersects(alu::Flags::CARRY),
        Condition::C => flags.intersects(alu::Flags::CARRY),
    }
}

fn alu_logic(
    code: &MicroCode,
    data_bus: i32,
    current_regs: &register::File,
    new_regs: &mut register::File,
) -> i32 {
    let act = if code.alu_mem_as_act {
        debug_assert!(util::is_8bit(data_bus));
        data_bus
    } else {
        current_regs.get(Register::ACT)
    };
    let tmp = current_regs.get(Register::ALU_TMP);
    let current_flags = Flags::from_bits(current_regs.get(Register::F)).unwrap();
    let (result, mut flags) = match code.alu_op {
        alu::Op::Bit | alu::Op::Res | alu::Op::Set => {
            code.alu_op.execute(act, i32::from(code.alu_bit_select), current_flags)
        }
        _ => code.alu_op.execute(act, tmp, current_flags),
    };
    if code.alu_cse_to_tmp {
        let is_negative = (tmp & 0x80) != 0;
        let is_carry = flags.intersects(alu::Flags::CARRY);
        // Can be written as simple arithmetic, but let's model how we want it in hardware.
        let tmp_value = if is_carry == is_negative {
            0
        } else if is_carry && !is_negative {
            1
        } else if !is_carry && is_negative {
            0xFF
        } else {
            panic!()
        };
        new_regs.set(Register::ALU_TMP, tmp_value);
    }
    if code.alu_f_force_nz {
        flags.remove(Flags::ZERO);
    }
    let flag_mask = (code.alu_write_f_mask << 4) as i32;
    let new_flags = (current_flags.bits() & !flag_mask) | (flags.bits() & flag_mask);
    new_regs.set(Register::F, new_flags);
    match code.alu_out_select {
        AluOutSelect::Result => result,
        AluOutSelect::Tmp => tmp,
        AluOutSelect::A => current_regs.get(Register::A),
        AluOutSelect::ACT => act,
        AluOutSelect::F => current_flags.bits(),
    }
}

fn alu_reg_write(code: &MicroCode, data_bus: i32, new_regs: &mut register::File) {
    let data = data_bus;
    match code.alu_out_select {
        AluOutSelect::Tmp => new_regs.set(Register::ALU_TMP, data),
        AluOutSelect::A | AluOutSelect::Result => new_regs.set(Register::A, data),
        AluOutSelect::ACT => new_regs.set(Register::ACT, data),
        AluOutSelect::F => new_regs.set(Register::F, data),
    };
}

fn interrupt_logic(code: &MicroCode, cpu: &mut Cpu, next_state: &mut cpu::State) {
    if code.enable_interrupts && cpu.state.interrupt_enable_counter == 0 {
        trace!(target: "int", "Enabling interrupts.");
        // This is a huge hack - but it might actually be not that bad.
        let is_reti = cpu.registers.get(Register::INSTR) == 0xD9;
        if is_reti {
            next_state.interrupt_enable_counter = 1;
        } else {
            next_state.interrupt_enable_counter = 2;
        }
        cpu.interrupts_enabled = false;
    } else if code.disable_interrupts {
        trace!(target: "int", "Disabling interrupts.");
        cpu.interrupts_enabled = false;
        next_state.interrupt_enable_counter = 0;
    }

    if code.is_halt {
        cpu.is_halted = true;
    }
}

fn execute(code: &MicroCode, cpu: &mut Cpu) -> cpu::State {
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
        if code.ff_to_addr_hi {
            next_state.address_latch |= 0xFF00;
        }
    }

    let data_bus_value = if code.alu_to_data {
        alu_logic(code, cpu.state.data_latch, &current_regs, &mut new_regs)
    } else if code.reg_to_data {
        debug_assert!(!code.reg_write_enable);
        current_regs.get(code.reg_select)
    } else {
        cpu.state.data_latch
    };

    let addr_bus_value =
        if code.inc_to_addr_bus { incrementer_logic(code, cpu, &current_regs) } else { -1 };

    if code.addr_write_enable {
        new_regs.set(code.addr_select, addr_bus_value);
    }

    if code.reg_write_enable {
        new_regs.set(code.reg_select, data_bus_value);
    }

    if code.alu_reg_write_enable {
        alu_reg_write(code, data_bus_value, &mut new_regs);
    }
    if code.alu_a_to_act {
        debug_assert!(!(code.alu_reg_write_enable && code.alu_out_select == AluOutSelect::ACT));
        new_regs.set(Register::ACT, current_regs.get(Register::A));
    }
    if code.alu_opymul8_to_act {
        let op_y = (current_regs.get(Register::INSTR) & 0b0011_1000) >> 3;
        new_regs.set(Register::ACT, op_y * 8);
    }
    if code.alu_a_to_tmp {
        new_regs.set(Register::ALU_TMP, current_regs.get(Register::A));
    }
    if code.alu_one_to_tmp {
        new_regs.set(Register::ALU_TMP, 1);
    } else if code.alu_64_to_tmp {
        new_regs.set(Register::ALU_TMP, 64);
    } else if code.alu_zero_to_tmp {
        new_regs.set(Register::ALU_TMP, 0);
    }

    // Handle interrupt flags.
    interrupt_logic(code, cpu, &mut next_state);

    // Copy to the data latch.
    if code.mem_write_enable {
        next_state.data_latch = data_bus_value;
    }
    cpu.registers = new_regs;
    next_state
}
