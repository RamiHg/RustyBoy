use super::{Arg, Op};

struct OpComponents {
    x: u8,
    y: u8,
    z: u8,
    p: u8,
    q: bool,
}

impl OpComponents {
    fn from_byte(byte: u8) -> OpComponents {
        OpComponents {
            x: (byte & 0b11000000) >> 6,
            y: (byte & 0b00111000) >> 3,
            z: (byte & 0b00000111),
            p: (byte & 0b00110000) >> 4,
            q: (byte & 0b00001000) != 0,
        }
    }
}

pub fn decode(byte0: u8, byte1: u8, byte2: u8) -> Result<Op, String> {
    let components = OpComponents::from_byte(byte0);
    let invalid_opcode_error = Err(format!("0x{:X?} is not a valid opcode.", byte0));
    let imm_16 = Arg::Unsigned16bit(byte1 as u16 | ((byte2 as u16) << 8));

    match components.x {
        // x = 0
        0 => Ok(match components.z {
            // z = 0
            0 => match components.y {
                0 => Op::new("NOP"),
                1 => Op::new_sized("LD", 3)
                    .with_lhs(imm_16.as_indirect())
                    .with_rhs(Arg::from_reg("SP")),
                2 => Op::new("STOP"),
                3 => Op::new_sized("JR", 2).with_lhs(Arg::from_i8(byte1)),
                _ => Op::new_sized("JR", 2)
                    .with_lhs(Arg::from_cond(components.y - 4))
                    .with_rhs(Arg::from_i8(byte1)),
            },
            // z = 1
            1 if !components.q => Op::new_sized("LD", 3)
                .with_lhs(Arg::from_sp_table(components.p))
                .with_rhs(imm_16),
            1 => Op::new("ADD")
                .with_lhs(Arg::from_reg("HL"))
                .with_rhs(Arg::from_sp_table(components.p)),
            // z = 2 and q = 0
            2 if !components.q => match components.p {
                0 => Op::new("LD")
                    .with_lhs(Arg::from_reg("BC").as_indirect())
                    .with_rhs(Arg::from_reg("A")),
                1 => Op::new("LD")
                    .with_lhs(Arg::from_reg("DE").as_indirect())
                    .with_rhs(Arg::from_reg("A")),
                2 => Op::new("LD")
                    .with_lhs(Arg::from_reg("HL+").as_indirect())
                    .with_rhs(Arg::from_reg("A")),
                3 | _ => Op::new("LD")
                    .with_lhs(Arg::from_reg("HL-").as_indirect())
                    .with_rhs(Arg::from_reg("A")),
            },
            // z = 2 and q = 1
            2 => match components.p {
                0 => Op::new("LD")
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_reg("BC").as_indirect()),
                1 => Op::new("LD")
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_reg("DE").as_indirect()),
                2 => Op::new("LD")
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_reg("HL+").as_indirect()),
                3 | _ => Op::new("LD")
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_reg("HL-").as_indirect()),
            },
            // z = 3
            3 if !components.q => Op::new("INC").with_lhs(Arg::from_sp_table(components.p)),
            3 => Op::new("DEC").with_lhs(Arg::from_sp_table(components.p)),
            // z = 4
            4 => Op::new("INC").with_lhs(Arg::from_reg_table(components.y)),
            // z = 5
            5 => Op::new("DEC").with_lhs(Arg::from_reg_table(components.y)),
            // z = 6
            6 => Op::new_sized("LD", 2)
                .with_lhs(Arg::from_reg_table(components.y))
                .with_rhs(Arg::from_u8(byte1)),
            // z = 7
            7 | _ => match components.y {
                0 => Op::new("RLCA"),
                1 => Op::new("RRCA"),
                2 => Op::new("RLA"),
                3 => Op::new("RRA"),
                4 => Op::new("DAA"),
                5 => Op::new("CPL"),
                6 => Op::new("SCF"),
                7 | _ => Op::new("CCF"),
            },
        }),
        // x = 1
        1 if components.z == 6 && components.y == 6 => Ok(Op::new("HALT")),
        1 => Ok(Op::new("LD")
            .with_lhs(Arg::from_reg_table(components.y))
            .with_rhs(Arg::from_reg_table(components.z))),
        // x = 2
        2 => Ok(Op::new_alu_op(
            components.y,
            Arg::from_reg_table(components.z),
        )),
        // x = 3
        3 | _ => match components.z {
            // z = 0
            0 => Ok(match components.y {
                4 => Op::new_sized("LD", 2)
                    .with_lhs(Arg::from_u8(byte1).as_ffplus())
                    .with_rhs(Arg::from_reg("A")),
                5 => Op::new_sized("ADD", 2)
                    .with_lhs(Arg::from_reg("SP"))
                    .with_rhs(Arg::from_i8(byte1)),
                6 => Op::new_sized("LD", 2)
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_u8(byte1).as_ffplus()),
                7 => Op::new_sized("LD", 2)
                    .with_lhs(Arg::from_reg("HL"))
                    .with_rhs(Arg::from_i8(byte1).as_spplus()),
                _ => Op::new("RET").with_lhs(Arg::from_cond(components.y)),
            }),
            // z = 1
            1 if !components.q => Ok(Op::new("POP").with_lhs(Arg::from_af_table(components.p))),
            1 => Ok(match components.p {
                0 => Op::new("RET"),
                1 => Op::new("RETI"),
                2 => Op::new("JP").with_lhs(Arg::from_reg("HL")),
                3 | _ => Op::new("LD")
                    .with_lhs(Arg::from_reg("SP"))
                    .with_rhs(Arg::from_reg("HL")),
            }),
            // z = 2
            2 => Ok(match components.y {
                4 => Op::new("LD")
                    .with_lhs(Arg::from_reg("C").as_ffplus())
                    .with_rhs(Arg::from_reg("A")),
                5 => Op::new_sized("LD", 3)
                    .with_lhs(imm_16.as_indirect())
                    .with_rhs(Arg::from_reg("A")),
                6 => Op::new("LD")
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(Arg::from_reg("C").as_ffplus()),
                7 => Op::new_sized("LD", 3)
                    .with_lhs(Arg::from_reg("A"))
                    .with_rhs(imm_16),
                _ => Op::new_sized("JP", 3)
                    .with_lhs(Arg::from_cond(components.y))
                    .with_rhs(imm_16),
            }),
            // z = 3
            3 => match components.y {
                0 => Ok(Op::new_sized("JP", 3).with_lhs(imm_16)),
                6 => Ok(Op::new("DI")),
                7 => Ok(Op::new("EI")),
                // TODO: Implement CB.
                _ => invalid_opcode_error,
            },
            // z = 4
            4 => match components.y {
                0..=3 => Ok(Op::new_sized("CALL", 3)
                    .with_lhs(Arg::from_cond(components.y))
                    .with_rhs(imm_16)),
                _ => invalid_opcode_error,
            },
            // z = 5
            5 if !components.q => Ok(Op::new("PUSH").with_lhs(Arg::from_af_table(components.p))),
            5 => match components.p {
                0 => Ok(Op::new_sized("CALL", 3).with_lhs(imm_16)),
                _ => invalid_opcode_error,
            },
            // z = 6
            6 => Ok(Op::new_alu_op(components.y, Arg::from_u8(byte1)).with_size(2)),
            // z = 7
            7 => Ok(Op::new("RST").with_lhs(Arg::from_u8(components.y * 8))),
            _ => panic!(),
        },
    }
}
