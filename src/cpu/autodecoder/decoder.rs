use std::collections::HashMap;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::cpu::alu;

use super::asm;
use super::csv_parser;
use super::micro_code::{Condition, MicroCode};
use super::op_map::MCycleMap;

use crate::{
    cpu::{register::Register, Cpu},
    memory::Memory,
};

pub struct Decoder {
    pla: MCycleMap,
}

#[derive(FromPrimitive)]
enum AluOpTable {
    AddA,
    AdcA,
    SubA,
    SbcA,
    AndA,
    XorA,
    OrA,
    CpA,
}

impl Into<asm::AluCommand> for AluOpTable {
    fn into(self) -> asm::AluCommand {
        use asm::AluCommand::*;
        match self {
            AluOpTable::AddA => Add,
            AluOpTable::AdcA => Addc,
            AluOpTable::SubA => Sub,
            AluOpTable::SbcA => Subc,
            AluOpTable::AndA => And,
            AluOpTable::XorA => Xor,
            AluOpTable::OrA => Or,
            AluOpTable::CpA => Cp,
        }
    }
}

impl Decoder {
    pub fn new() -> Decoder {
        Decoder {
            pla: csv_parser::parse_csv(
                r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv",
            ),
        }
    }

    pub fn decode(&self, op: i32, memory: &Memory) -> Vec<MicroCode> { self.decode_op(op) }

    fn decode_op(&self, opcode: i32) -> Vec<MicroCode> {
        // This method uses the amazing guide to decode instructions programatically:
        // https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html

        // Deconstruct the op into its components.
        let op_z = opcode & 0b0000_0111;
        let op_y = (opcode & 0b0011_1000) >> 3;
        let op_x = (opcode & 0b1100_0000) >> 6;
        let op_q = op_y & 0b001;
        let op_p = (op_y & 0b110) >> 1;

        let maybe_alu_op = AluOpTable::from_i32(op_y);

        let mcycle_list = match op_x {
            // x = 0
            0 => match op_z {
                // z = 0
                0 => match op_y {
                    // JR d
                    3 => self.pla["JR[cc],i8"].prune_ccend(),
                    4..=7 => {
                        self.pla["JR[cc],i8"].remap_cond(Condition::from_i32(op_y - 4).unwrap())
                    }
                    _ => panic!("Implement {:X?}", opcode),
                },
                // z = 1
                1 => match op_q {
                    // q = 0. LD rr, nn
                    0 => self.pla["LDrr,i16"].remap_lhs_reg(Register::from_sp_pair_table(op_p)),
                    1 | _ => self.pla["ADDHL,rr"].remap_rhs_reg(Register::from_sp_pair_table(op_p)),
                },
                // z = 2. Assorted indirect loads
                2 => match op_q {
                    // q = 0
                    0 => match op_p {
                        // LD (BC), A
                        0 => self.pla["LD(rr),r"]
                            .remap_lhs_reg(Register::BC)
                            .remap_rhs_reg(Register::A),
                        // LD (DE), A
                        1 => self.pla["LD(rr),r"]
                            .remap_lhs_reg(Register::DE)
                            .remap_rhs_reg(Register::A),
                        // LD (HL+/i), A
                        2 => self.pla["LDI(HL),A"].clone(),
                        3 | _ => self.pla["LDD(HL),A"].clone(),
                    },
                    // q = 1
                    1 | _ => match op_p {
                        // LD A, (BC)
                        0 => self.pla["LDr,(rr)"]
                            .remap_lhs_reg(Register::A)
                            .remap_rhs_reg(Register::BC),
                        // LD A, (DE)
                        1 => self.pla["LDr,(rr)"]
                            .remap_lhs_reg(Register::A)
                            .remap_rhs_reg(Register::DE),
                        2 => self.pla["LDIA,(HL)"].clone(),
                        3 => self.pla["LDDA,(HL)"].clone(),
                        _ => panic!(),
                    },
                },
                // z = 3. INC/DEC rr
                3 if op_q == 0 => self.pla["INC/DECrr"]
                    .remap_alu_placeholder(asm::AluCommand::Add)
                    .remap_lhs_reg(Register::from_sp_pair_table(op_p)),
                3 if op_q == 1 => self.pla["INC/DECrr"]
                    .remap_alu_placeholder(asm::AluCommand::Sub)
                    .remap_lhs_reg(Register::from_sp_pair_table(op_p)),
                // z = 4. INC n
                4 => {
                    if op_y == 6 {
                        self.pla["INC/DEC(HL)"].remap_alu_placeholder(asm::AluCommand::Add)
                    } else {
                        self.pla["INC/DECr"]
                            .remap_alu_placeholder(asm::AluCommand::Add)
                            .remap_lhs_reg(Register::from_single_table(op_y))
                    }
                }
                // z = 5. DEC n
                5 => {
                    if op_y == 6 {
                        self.pla["INC/DEC(HL)"].remap_alu_placeholder(asm::AluCommand::Sub)
                    } else {
                        self.pla["INC/DECr"]
                            .remap_alu_placeholder(asm::AluCommand::Sub)
                            .remap_lhs_reg(Register::from_single_table(op_y))
                    }
                }
                // z = 6. LD r, n
                6 => self.pla["LDr,i8"].remap_lhs_reg(Register::from_single_table(op_y)),
                _ => panic!(),
            },
            // x = 1
            1 => {
                let op = if op_y == 6 && op_z == 6 {
                    panic!("Implement HALT")
                } else if op_y == 6 {
                    "LD(rr),r"
                } else if op_z == 6 {
                    "LDr,(rr)"
                } else {
                    "LDr,r"
                };
                self.pla[op]
                    .remap_lhs_reg(Register::from_single_table(op_y))
                    .remap_rhs_reg(Register::from_single_table(op_z))
            }
            // x = 2. ALU A, r
            2 if op_z == 6 => self.pla["aluA,(HL)"]
                .remap_alu_placeholder(AluOpTable::from_i32(op_y).unwrap().into()),
            2 => self.pla["aluA,r"]
                .remap_rhs_reg(Register::from_single_table(op_z))
                .remap_alu_placeholder(AluOpTable::from_i32(op_y).unwrap().into()),
            // x = 3
            3 => match op_z {
                // z = 0
                0 => match op_y {
                    4 => self.pla["LD(FF00+i8),A"].clone(),
                    5 => self.pla["ADDSP,i8"].clone(),
                    6 => self.pla["LDA,(FF00+i8)"].clone(),
                    7 => self.pla["LDHL,SP+i8"].clone(),
                    _ => self.pla["RETcc"].remap_cond(Condition::from_i32(op_y).unwrap()),
                },
                // z = 1
                1 => match op_q {
                    // q = 1
                    1 => match op_p {
                        0 => self.pla["RET"].clone(),
                        2 => self.pla["JPHL"].clone(),
                        3 => self.pla["LDSP,HL"].clone(),
                        _ => panic!("Implement {:X?}", opcode),
                    },
                    _ => panic!("Implement {:X?}", opcode),
                },
                // z = 2
                2 => match op_y {
                    4 => self.pla["LD(FF00+C),A"].clone(),
                    5 => self.pla["LD(i16),A"].clone(),
                    6 => self.pla["LDA,(FF00+C)"].clone(),
                    7 => self.pla["LDA,(i16)"].clone(),
                    _ => self.pla["JP[cc],i16"].remap_cond(Condition::from_i32(op_y).unwrap()),
                },
                // z = 3
                3 => match op_y {
                    0 => self.pla["JP[cc],i16"].prune_ccend(),
                    _ => panic!("Implement {:X?}", opcode),
                },
                // z = 4. CALL [cc], nn
                4 => match op_y {
                    0..=3 => {
                        self.pla["CALL[cc],i16"].remap_cond(Condition::from_i32(op_y).unwrap())
                    }
                    _ => panic!("Implement NOP"),
                },
                // z = 5.
                5 => match op_q {
                    1 if op_p == 0 => self.pla["CALL[cc],i16"].prune_ccend(),
                    _ => panic!(),
                },
                // z = 6. ALU A, n
                6 => self.pla["aluA,i8"].remap_alu_placeholder(maybe_alu_op.unwrap().into()),
                _ => panic!("Implement {:X?}", opcode),
            },
            _ => panic!("Implement {:X?}", opcode),
        };

        // Compile the MCyle assembly.
        let micro_codes = mcycle_list.compile();
        micro_codes
    }
}
