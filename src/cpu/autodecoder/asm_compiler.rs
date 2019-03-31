use super::asm::{AluCommand, Arg, Command, Op};
use super::micro_code::{AluOp, AluOutSelect, IncOp, MicroCode};
use crate::cpu::register::Register;

impl AluOutSelect {
    fn from_register(register: Register) -> Option<AluOutSelect> {
        use AluOutSelect::*;
        match register {
            Register::ALU_TMP => Some(Tmp),
            Register::A => Some(A),
            Register::ACT => Some(ACT),
            Register::F => Some(F),
            _ => None,
        }
    }
}

/// Main entry point. Uses compile_op and micro_code_combine (defined at the bottom) to compile a
/// list of asm::Ops.
pub fn compile_op_list<'a>(op_list: impl Iterator<Item = &'a Op>) -> MicroCode {
    let code = op_list
        .map(compile_op)
        .fold(MicroCode::default(), micro_code_combine);
    verify_micro_code(&code);
    code
}

fn compile_op(op: &Op) -> MicroCode {
    use Command::*;
    let compile_fn = match op.cmd {
        ADDR => compile_addr,
        RADDR => compile_raddr,
        ADDR_H_FF => compile_addr_h_ff,
        RD => compile_rd,
        WR => compile_wr,
        LD => compile_ld,
        ALU(_) | MOV => compile_alu,
        CSE => compile_cse,
        FMSK => compile_fmsk,
        FZ => compile_f,
        INC => compile_incdec,
        DEC => compile_incdec,
        END => compile_end,
        CCEND => compile_ccend,
        NOP => compile_nop,
        _ => panic!("Implement {:?}", op.cmd),
    };
    compile_fn(op)
}

fn compile_addr(op: &Op) -> MicroCode {
    // Latch an address.
    let addr_select = op.lhs.expect_as_pair();
    op.rhs.expect_none();
    MicroCode {
        reg_to_addr_buffer: true,
        addr_select,
        ..Default::default()
    }
}

fn compile_raddr(op: &Op) -> MicroCode {
    // This is simply ADDR followed by read enable.
    let base = compile_addr(op);
    MicroCode {
        mem_read_enable: true,
        ..base
    }
}

fn compile_addr_h_ff(op: &Op) -> MicroCode {
    op.lhs.expect_none();
    op.rhs.expect_none();
    MicroCode {
        ff_to_addr_hi: true,
        ..Default::default()
    }
}

fn compile_rd(op: &Op) -> MicroCode {
    let dst = op.lhs.expect_as_register();
    assert!(dst.is_single());
    op.rhs.expect_none();
    match dst {
        Register::ALU_TMP => MicroCode {
            alu_out_select: AluOutSelect::Tmp,
            alu_reg_write_enable: true,
            ..Default::default()
        },
        Register::ACT => MicroCode {
            alu_out_select: AluOutSelect::ACT,
            alu_reg_write_enable: true,
            ..Default::default()
        },
        _ => MicroCode {
            reg_select: dst,
            reg_write_enable: true,
            ..Default::default()
        },
    }
}

fn compile_wr(op: &Op) -> MicroCode {
    let src = op.lhs.expect_as_register();
    assert!(src.is_single());
    op.rhs.expect_none();
    MicroCode {
        mem_write_enable: true,
        reg_select: src,
        reg_to_data: true,
        ..Default::default()
    }
}

fn compile_mov(op: &Op) -> MicroCode {
    let dst = op.lhs.expect_as_register();
    op.rhs.expect_none();
    if dst.is_single() {
        let as_alu = Op {
            cmd: Command::ALU(AluCommand::Mov),
            lhs: op.lhs.clone(),
            rhs: op.rhs.clone(),
        };
        compile_alu(&as_alu)
    } else {
        // This is actually an incrementer operation.
        MicroCode {
            inc_op: IncOp::Mov,
            inc_to_addr_bus: true,
            addr_select: dst,
            addr_write_enable: true,
            ..Default::default()
        }
    }
}

fn compile_incdec(op: &Op) -> MicroCode {
    let addr_select = op.lhs.expect_as_pair();
    op.rhs.expect_none();
    let inc_op = match op.cmd {
        Command::INC => IncOp::Inc,
        Command::DEC => IncOp::Dec,
        Command::MOV => IncOp::Mov,
        _ => panic!(),
    };
    MicroCode {
        inc_op,
        inc_to_addr_bus: true,
        addr_select,
        addr_write_enable: true,
        ..Default::default()
    }
}

fn compile_ld(op: &Op) -> MicroCode {
    // Dirty secret: we have to use the ALU to do any (8-bit) register moves.
    let destination =
        AluOutSelect::from_register(op.lhs.expect_as_register()).unwrap_or_else(|| {
            panic!(
                "LD can only have an ALU register as a destination: {:?}",
                op.lhs
            )
        });
    match &op.rhs.0 {
        Some(Arg::Register(source)) if source == &Register::A => match destination {
            AluOutSelect::ACT => MicroCode {
                alu_a_to_act: true,
                ..Default::default()
            },
            AluOutSelect::Tmp => MicroCode {
                alu_a_to_tmp: true,
                ..Default::default()
            },
            _ => panic!("Cannot write A to {:?}", destination),
        },
        Some(Arg::Register(source)) => MicroCode {
            // Write the source to the data bus.
            reg_select: *source,
            reg_to_data: true,
            // Sample the data bus into the ALU register.
            alu_out_select: destination,
            alu_reg_write_enable: true,
            ..Default::default()
        },
        Some(Arg::ConstantPlaceholder(string)) => {
            let value: i32 = string
                .parse()
                .unwrap_or_else(|_| panic!("Cannot parse {} as int", string));
            if destination == AluOutSelect::Tmp && value == 1 {
                MicroCode {
                    alu_one_to_tmp: true,
                    ..Default::default()
                }
            } else {
                panic!(
                    "Unsupported constant {} to load to {:?}",
                    string, destination
                )
            }
        }
        _ => panic!("Unexpected LD RHS: {:?}", op.rhs.0),
    }
}

fn compile_alu(op: &Op) -> MicroCode {
    let alu_command = match &op.cmd {
        Command::ALU(command) => command,
        Command::MOV => &AluCommand::Mov,
        _ => panic!(),
    };
    // Revisit if this mapping should even exist.
    let alu_op = match alu_command {
        AluCommand::Mov => AluOp::Mov,
        AluCommand::Add => AluOp::Add,
        AluCommand::Addc => AluOp::Addc,
        AluCommand::Sub => AluOp::Sub,
        AluCommand::Subc => AluOp::Subc,
        AluCommand::And => AluOp::And,
        AluCommand::Xor => AluOp::Xor,
        AluCommand::Or => AluOp::Or,
        AluCommand::Cp => AluOp::Cp,
        _ => panic!("Implement {:?}", alu_command),
    };
    if alu_op == AluOp::Mov && op.lhs.expect_as_register().is_pair() {
        // This is actually an incrementer operation!
        return compile_incdec(op);
    }
    let dst = op.lhs.expect_as_register();
    assert!(dst.is_single());
    if let Register::A = dst {
        MicroCode {
            alu_op,
            alu_out_select: AluOutSelect::Result,
            alu_to_data: true,
            alu_reg_write_enable: true,
            ..Default::default()
        }
    } else {
        MicroCode {
            alu_op,
            alu_out_select: AluOutSelect::Result,
            alu_to_data: true,
            reg_select: dst,
            reg_write_enable: true,
            ..Default::default()
        }
    }
}

fn compile_cse(op: &Op) -> MicroCode {
    op.lhs.expect_none();
    op.rhs.expect_none();
    MicroCode {
        alu_cse_to_tmp: true,
        ..Default::default()
    }
}

fn compile_fmsk(op: &Op) -> MicroCode {
    op.rhs.expect_none();
    if let Some(Arg::ConstantPlaceholder(string)) = &op.lhs.0 {
        let mask = i32::from_str_radix(string, 2)
            .unwrap_or_else(|_| panic!("Invalid FMSK constant: {}", string));
        MicroCode {
            alu_write_f_mask: mask as u8,
            ..Default::default()
        }
    } else {
        panic!("Unexpected FMSK arg: {:?}", op.lhs)
    }
}

fn compile_f(op: &Op) -> MicroCode {
    assert_eq!(op.cmd, Command::FZ);
    let string_value = if let Some(Arg::ConstantPlaceholder(string)) = &op.lhs.0 {
        string
    } else {
        panic!("Expected arbitrary constant. Got: {:?}", op.rhs)
    };
    // I'm too lazy to make this any smarter.
    if string_value == "0" {
        MicroCode {
            alu_f_force_nz: true,
            ..Default::default()
        }
    } else {
        panic!("Unsupported FZ command: {:?}", op);
    }
}

fn compile_end(op: &Op) -> MicroCode {
    op.lhs.expect_none();
    op.rhs.expect_none();
    MicroCode {
        is_end: true,
        ..Default::default()
    }
}

fn compile_ccend(op: &Op) -> MicroCode {
    let cond = if let Some(Arg::CC(cond)) = op.lhs.0 {
        cond
    } else {
        panic!("Expected condition. Got: {:?}", op.lhs)
    };
    op.rhs.expect_none();
    MicroCode {
        is_cond_end: true,
        cond,
        ..Default::default()
    }
}

fn compile_nop(op: &Op) -> MicroCode { MicroCode::default() }

// The second part of compilation is combining all the TCycle's microcodes into one. This also
// checks for potential hazards and invalid operations.

/// Used when folding a collection of micro-code. Logically combines two micro-codes. Only checks
/// that the fields being set are not already set.
#[allow(clippy::cognitive_complexity)] // seriously?
fn micro_code_combine(mut acc: MicroCode, code: MicroCode) -> MicroCode {
    // TODO: This could be done using bincode (or transmute, really). But we do it the boring and
    // explicit way as a documentation, and to prevent unexpected mistakes.
    // But to make my life easier, I use a macro.
    macro_rules! move_if_unset {
        ($field:ident) => {
            let default_value = <_ as Default>::default();
            if code.$field != default_value {
                assert_eq!(
                    acc.$field,
                    default_value,
                    "{} is set in microcode, but is already set in previous ops. Value previously \
                     set: {:?}. New value: {:?}",
                    stringify!($field),
                    acc.$field,
                    code.$field
                );
                acc.$field = code.$field;
            }
        };
    }

    move_if_unset!(mem_read_enable);
    move_if_unset!(mem_write_enable);
    move_if_unset!(reg_select);
    move_if_unset!(reg_write_enable);
    move_if_unset!(reg_to_data);
    move_if_unset!(reg_to_addr_buffer);
    move_if_unset!(ff_to_addr_hi);
    move_if_unset!(addr_select);
    move_if_unset!(addr_write_enable);
    move_if_unset!(inc_op);
    move_if_unset!(inc_to_addr_bus);
    move_if_unset!(alu_op);
    move_if_unset!(alu_out_select);
    move_if_unset!(alu_to_data);
    move_if_unset!(alu_reg_write_enable);
    move_if_unset!(alu_a_to_act);
    move_if_unset!(alu_a_to_tmp);
    move_if_unset!(alu_one_to_tmp);
    move_if_unset!(alu_cse_to_tmp);
    move_if_unset!(alu_f_force_nz);
    move_if_unset!(alu_write_f_mask);
    move_if_unset!(is_end);
    move_if_unset!(is_cond_end);
    move_if_unset!(cond);

    acc
}

// Finally, validation. Makes sure (to the best of my abilities) that we don't encounter invalid
// states.
fn verify_micro_code(code: &MicroCode) {
    assert!(
        !(code.mem_read_enable && code.mem_write_enable),
        "Cannot read and write at the same time."
    );
    assert!(
        !(code.reg_write_enable && code.reg_to_data),
        "Cannot read and write data bus from register file."
    );
    assert!(
        !(code.reg_to_addr_buffer && code.addr_write_enable),
        "Cannot read and write address bus from register file."
    );
    assert!(
        !(code.ff_to_addr_hi && !code.reg_to_addr_buffer),
        "Cannot use FF to address high if not writing from register file."
    );
    assert!(
        !(code.inc_to_addr_bus && code.reg_to_addr_buffer),
        "Cannot drive address bus from both register file and address buffer."
    );
    assert!(
        !(code.alu_reg_write_enable
            && code.alu_to_data
            && code.alu_out_select != AluOutSelect::Result),
        "Cannot read and write ALU registers."
    );
    assert!(
        !(code.alu_reg_write_enable
            && code.alu_out_select == AluOutSelect::ACT
            && code.alu_a_to_act),
        "Data hazard writing to ACT."
    );
    assert!(
        !(code.alu_reg_write_enable
            && code.alu_out_select == AluOutSelect::Tmp
            && (code.alu_a_to_tmp || code.alu_one_to_tmp || code.alu_cse_to_tmp)),
        "Data hazard writing to ACT."
    );
    // TODO: Fix this assert logic.
    assert!(
        !(code.alu_one_to_tmp && code.alu_a_to_tmp && code.alu_cse_to_tmp),
        "Data hazard writing to TMP."
    );
    assert!(
        !(code.alu_cse_to_tmp && !code.alu_to_data),
        "Using CSE with potentially no ALU operation."
    );
    assert!(
        !(code.alu_to_data && code.reg_to_data),
        "Cannot drive data bus from both ALU and register file."
    );
    assert!(
        !(code.alu_f_force_nz && ((code.alu_write_f_mask & 0b1000) == 0)),
        "Forcing Z flag without writing it back,"
    );
    assert!(
        !(code.is_end && code.is_cond_end),
        "End and CCEnd both set."
    );
}
