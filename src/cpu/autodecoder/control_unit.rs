use super::{AluOp, IncOp, MicroCode};
use crate::{
    cpu::{
        self,
        micro_code::{Output, SideEffect},
    },
    memory::Memory,
};

use cpu::{
    alu::Flags,
    register::{self, Register},
    Cpu, DecodeMode,
};

use super::decoder;

fn fetch_t1() -> MicroCode {
    MicroCode {
        // We set mem_read_enable to true explicitly even though we always reset at T4.
        mem_read_enable: true,
        mem_set_address: true,
        mem_reg_address: Register::PC,
        ..Default::default()
    }
}

fn fetch_t2() -> MicroCode {
    MicroCode {
        ..Default::default()
    }
}

pub fn cycle(cpu: &mut Cpu, memory: &Memory) -> Output {
    let micro_code = match cpu.state.decode_mode {
        DecodeMode::Fetch if cpu.t_state.get() == 1 => fetch_t1(),
        DecodeMode::Fetch if cpu.t_state.get() == 2 => {
            cpu.state.decode_mode = DecodeMode::Decode;
            fetch_t2()
        }
        DecodeMode::Decode if cpu.t_state.get() == 3 => {
            // Skip over the useless T1 and T2. (TODO: Maybe don't create them in the first place).
            // The decoder is a Mealy FSM in that the T3 state depends on the input (memory).
            cpu.micro_code_v2_stack = decoder::decode(cpu.state.data_latch, cpu, memory)
                .iter()
                .skip(2)
                .cloned()
                .collect();
            cpu.state.decode_mode = DecodeMode::Execute;
            let mut first = cpu.micro_code_v2_stack.remove(0);
            // TODO: Figure out a clean way to do this.
            first.reg_write_enable = false;
            first
        }
        DecodeMode::Execute => cpu.micro_code_v2_stack.remove(0),
        _ => panic!(
            "Invalid internal decode mode: {:?} Tstate {:?}.",
            cpu.state.decode_mode,
            cpu.t_state.get()
        ),
    };
    // Execute the micro-code.
    let output = execute(&micro_code, cpu, memory);
    if micro_code.is_end {
        assert_eq!(cpu.t_state.get(), 4);
        cpu.state.decode_mode = DecodeMode::Fetch;
    }
    output
}

/// Incrementer module.
fn incrementer_logic(
    code: &MicroCode,
    cpu: &Cpu,
    current_state: &register::File,
    new_state: &mut register::File,
) {
    // The input is either whatever is in the address latch, or directly
    // read from the address bus if we're skipping the latch.
    let source_value = if code.inc_skip_latch {
        current_state.get(code.mem_reg_address)
    } else {
        cpu.state.address_latch
    };
    let new_value = match code.inc_op {
        IncOp::Mov => source_value,
        IncOp::Inc => (source_value + 1) & 0xFFFF,
        IncOp::Dec => (source_value - 1) & 0xFFFF,
    };
    if code.inc_write {
        new_state.set(code.inc_dest, new_value)
    }
}

/// ALU module.
fn alu_logic(
    code: &MicroCode,
    cpu: &Cpu,
    current_state: &register::File,
    new_state: &mut register::File,
) -> i32 {
    // For now, model the ALU as a Moore FSM.
    let act = current_state.get(Register::ALU_ACT);
    let rhs = current_state.get(Register::ALU_TMP);
    let f = current_state.get(Register::ALU_TMP_F);

    use AluOp::*;
    let (result, flags) = match code.alu_op {
        Mov => (rhs, Flags::empty()),
        _ => panic!("Implement {:?}.", code.alu_op),
    };

    // Now, perform the sequential logic.
    assert!(
        !(code.alu_bus_tmp_read && code.alu_bus_tmp_read_mem),
        "Race condition. Setting TMP from both memory and register busses."
    );

    let alu_bus_value = current_state.get(code.alu_bus_select);
    // Set the new TMP register.
    new_state.set(
        Register::ALU_TMP,
        if code.alu_bus_tmp_read {
            alu_bus_value
        } else if code.alu_bus_tmp_read_mem {
            cpu.state.data_latch
        } else {
            rhs
        },
    );
    // Set the new ACT register.
    new_state.set(
        Register::ALU_ACT,
        if code.alu_act_read {
            current_state.get(code.reg_select)
        } else {
            act
        },
    );
    // Set the new TMP flags register.
    new_state.set(
        Register::ALU_TMP_F,
        if code.alu_bus_f_reset {
            0
        } else if code.alu_bus_f_read {
            alu_bus_value
        } else {
            flags.bits()
        },
    );
    if code.alu_bus_f_write != 0 {
        assert_eq!(code.alu_bus_f_write, 0xF);
        assert_eq!(
            code.alu_bus_select,
            Register::F,
            "Must set the ALU bus select to F if writing flags: {:?}.",
            code.alu_bus_select
        );
        new_state.set(Register::F, flags.bits());
    }
    // Finally, return the result.
    result
}

fn execute(code: &MicroCode, cpu: &mut Cpu, memory: &Memory) -> Output {
    dbg!(code);
    assert!(
        !(code.mem_read_enable && code.mem_write_enable),
        "Cannot read and write at the same time: {:?}.",
        code
    );
    assert!(
        !(cpu.state.read_latch && cpu.state.write_latch),
        "Invalid CPU state enountered: Read and write latches are asserted."
    );

    let current_regs = cpu.registers;
    let mut new_regs = current_regs;

    if code.mem_read_enable {
        cpu.state.read_latch = true;
        cpu.state.write_latch = false;
    }

    if code.mem_write_enable {
        cpu.state.read_latch = false;
        cpu.state.write_latch = true;
        cpu.state.data_latch = current_regs.get(code.reg_select);
    }

    if code.mem_set_address {
        cpu.state.address_latch = current_regs.get(code.mem_reg_address);
    }

    let alu_result = alu_logic(code, cpu, &current_regs, &mut new_regs);

    if code.reg_write_enable {
        if code.alu_write {
            new_regs.set(code.reg_select, alu_result);
        } else if cpu.state.read_latch {
            assert_eq!(
                cpu.t_state.get(),
                3,
                "Can only sample memory at rising edge of T3. Current TState: {:?}.",
                cpu.t_state.get()
            );
            new_regs.set(code.reg_select, cpu.state.data_latch);
        } else {
            panic!(
                "Register write enabled with no source driving internal bus: {:?}.",
                code
            );
        }
        assert!(
            !code.inc_write || !code.inc_dest.overlaps(code.reg_select),
            "Race condition. Incrementer destination ({:?}) overlaps with register write ({:?}).",
            code.inc_dest,
            code.reg_select
        );
    }

    incrementer_logic(code, cpu, &current_regs, &mut new_regs);

    if cpu.t_state.get() == 4 {
        assert!(
            !code.mem_write_enable,
            "Cannot assert write enable at T4: {:?}.",
            code
        );
        // As a matter of fact, we shouldn't be asserting anything right now!
        assert!(!code.mem_read_enable);
        // Reset memory control.
        cpu.state.read_latch = true;
        cpu.state.write_latch = false;
    }

    // Finally, copy over the new state.
    cpu.registers = new_regs;

    Output {
        side_effect: None,
        is_done: code.is_end,
    }
}
