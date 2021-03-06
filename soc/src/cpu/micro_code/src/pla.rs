use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::micro_code::{AluOp, Condition, MicroCode};
use crate::register::Register;
use crate::{csv_loader, op_map::MCycleMap};

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

impl Into<AluOp> for AluOpTable {
    fn into(self) -> AluOp {
        use AluOp::*;
        match self {
            AluOpTable::AddA => Add,
            AluOpTable::AdcA => Adc,
            AluOpTable::SubA => Sub,
            AluOpTable::SbcA => Sbc,
            AluOpTable::AndA => And,
            AluOpTable::XorA => Xor,
            AluOpTable::OrA => Or,
            AluOpTable::CpA => Cp,
        }
    }
}

#[derive(FromPrimitive)]
pub enum RotShiftTable {
    RLC,
    RRC,
    RL,
    RR,
    SLA,
    SRA,
    SWAP,
    SRL,
}

impl Into<AluOp> for RotShiftTable {
    fn into(self) -> AluOp {
        use AluOp::*;
        match self {
            RotShiftTable::RLC => Rlc,
            RotShiftTable::RRC => Rrc,
            RotShiftTable::RL => Rl,
            RotShiftTable::RR => Rr,
            RotShiftTable::SLA => Sla,
            RotShiftTable::SRA => Sra,
            RotShiftTable::SWAP => Swap,
            RotShiftTable::SRL => Srl,
        }
    }
}

#[derive(FromPrimitive)]
pub enum AFPairTable {
    BC,
    DE,
    HL,
    AF,
}

impl Into<Register> for AFPairTable {
    fn into(self) -> Register {
        use AFPairTable::*;
        match self {
            BC => Register::BC,
            DE => Register::DE,
            HL => Register::HL,
            AF => Register::AF,
        }
    }
}

impl From<i32> for Condition {
    fn from(i: i32) -> Condition {
        use Condition::*;
        match i {
            0 => NZ,
            1 => Z,
            2 => NC,
            3 => C,
            _ => panic!("Unexpected condition: {}.", i),
        }
    }
}

pub struct DecoderBuilder {
    pla: MCycleMap,
}

impl Default for DecoderBuilder {
    fn default() -> DecoderBuilder {
        DecoderBuilder {
            pla: csv_loader::parse_csv(include_bytes!("../../../../instructions.csv")),
        }
    }
}

impl DecoderBuilder {
    pub fn decode(&self, op: i32, in_cb_mode: bool) -> Vec<MicroCode> {
        if in_cb_mode {
            self.decode_cb_op(op)
        } else {
            self.decode_op(op)
        }
    }

    pub fn interrupt_handler(&self) -> Vec<MicroCode> {
        let handler = self.pla["INTERRUPT"].clone().compile();
        debug_assert_eq!(handler.len(), 2 + 4 * 4);
        handler
    }

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
        let nop = self.pla["NOP"].clone();

        let mcycle_list = match op_x {
            // x = 0
            0 => match op_z {
                // z = 0
                0 => match op_y {
                    // JR d
                    0 => nop,
                    1 => self.pla["LD(i16),SP"].clone(),
                    2 => nop, // TODO: IMPLEMENT STOP
                    3 => self.pla["JR[cc],i8"].prune_ccend(),
                    4..=7 | _ => self.pla["JR[cc],i8"].remap_cond(Condition::from(op_y - 4)),
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
                        3 | _ => self.pla["LDDA,(HL)"].clone(),
                    },
                },
                // z = 3. INC/DEC rr
                3 if op_q == 0 => self.pla["INC/DECrr"]
                    .remap_alu_placeholder(AluOp::Add)
                    .remap_lhs_reg(Register::from_sp_pair_table(op_p)),
                3 if op_q == 1 => self.pla["INC/DECrr"]
                    .remap_alu_placeholder(AluOp::Sub)
                    .remap_lhs_reg(Register::from_sp_pair_table(op_p)),
                // z = 4. INC n
                4 => {
                    if op_y == 6 {
                        self.pla["INC/DEC(HL)"].remap_alu_placeholder(AluOp::Add)
                    } else {
                        self.pla["INC/DECr"]
                            .remap_alu_placeholder(AluOp::Add)
                            .remap_lhs_reg(Register::from_single_table(op_y))
                    }
                }
                // z = 5. DEC n
                5 => {
                    if op_y == 6 {
                        self.pla["INC/DEC(HL)"].remap_alu_placeholder(AluOp::Sub)
                    } else {
                        self.pla["INC/DECr"]
                            .remap_alu_placeholder(AluOp::Sub)
                            .remap_lhs_reg(Register::from_single_table(op_y))
                    }
                }
                // z = 6. LD r, n
                6 => {
                    if op_y == 6 {
                        self.pla["LD(HL),i8"].clone()
                    } else {
                        self.pla["LDr,i8"].remap_lhs_reg(Register::from_single_table(op_y))
                    }
                }
                // z = 7. Assorted ALU
                7 | _ => match op_y {
                    0 => self.pla["rotA"].remap_alu_placeholder(AluOp::Rlc),
                    1 => self.pla["rotA"].remap_alu_placeholder(AluOp::Rrc),
                    2 => self.pla["rotA"].remap_alu_placeholder(AluOp::Rl),
                    3 => self.pla["rotA"].remap_alu_placeholder(AluOp::Rr),
                    4 => self.pla["aluA,r"]
                        .remap_alu_placeholder(AluOp::Daa)
                        .remap_rhs_reg(Register::TEMP_LOW),
                    5 => self.pla["aluA,r"]
                        .remap_alu_placeholder(AluOp::Cpl)
                        .remap_rhs_reg(Register::TEMP_LOW),
                    6 => self.pla["aluA,r"]
                        .remap_alu_placeholder(AluOp::Scf)
                        .remap_rhs_reg(Register::TEMP_LOW),
                    7 | _ => self.pla["aluA,r"]
                        .remap_alu_placeholder(AluOp::Ccf)
                        .remap_rhs_reg(Register::TEMP_LOW),
                },
            },
            // x = 1
            1 => {
                let op = if op_y == 6 && op_z == 6 {
                    "HALT"
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
            3 | _ => match op_z {
                // z = 0
                0 => match op_y {
                    4 => self.pla["LD(FF00+i8),A"].clone(),
                    5 => self.pla["ADDSP,i8"].clone(),
                    6 => self.pla["LDA,(FF00+i8)"].clone(),
                    7 => self.pla["LDHL,SP+i8"].clone(),
                    _ => self.pla["RETcc"].remap_cond(Condition::from(op_y)),
                },
                // z = 1
                1 => match op_q {
                    // q = 0
                    0 => {
                        self.pla["POPrr"].remap_lhs_reg(AFPairTable::from_i32(op_p).unwrap().into())
                    }
                    // q = 1
                    1 | _ => match op_p {
                        0 => self.pla["RET[I]"].prune_ei(),
                        1 => self.pla["RET[I]"].clone(),
                        2 => self.pla["JPHL"].clone(),
                        3 | _ => self.pla["LDSP,HL"].clone(),
                    },
                },
                // z = 2
                2 => match op_y {
                    4 => self.pla["LD(FF00+C),A"].clone(),
                    5 => self.pla["LD(i16),A"].clone(),
                    6 => self.pla["LDA,(FF00+C)"].clone(),
                    7 => self.pla["LDA,(i16)"].clone(),
                    _ => self.pla["JP[cc],i16"].remap_cond(Condition::from(op_y)),
                },
                // z = 3
                3 => match op_y {
                    0 => self.pla["JP[cc],i16"].prune_ccend(),
                    1 => self.pla["CB"].clone(),
                    6 => self.pla["DI"].clone(),
                    7 => self.pla["EI"].clone(),
                    _ => nop,
                },
                // z = 4. CALL [cc], nn
                4 => match op_y {
                    0..=3 => self.pla["CALL[cc],i16"].remap_cond(Condition::from(op_y)),
                    _ => nop,
                },
                // z = 5.
                5 => match op_q {
                    0 => self.pla["PUSHrr"]
                        .remap_lhs_reg(AFPairTable::from_i32(op_p).unwrap().into()),
                    1 if op_p == 0 => self.pla["CALL[cc],i16"].prune_ccend(),
                    _ => nop,
                },
                // z = 6. ALU A, n
                6 => self.pla["aluA,i8"].remap_alu_placeholder(maybe_alu_op.unwrap().into()),
                7 | _ => self.pla["RST"].clone(),
            },
        };

        // Compile the MCyle assembly.
        mcycle_list.compile()
    }

    fn decode_cb_op(&self, opcode: i32) -> Vec<MicroCode> {
        // Deconstruct the op into its components.
        let op_z = opcode & 0b0000_0111;
        let op_y = (opcode & 0b0011_1000) >> 3;
        let op_x = (opcode & 0b1100_0000) >> 6;

        let is_hl = op_z == 6;

        let alu_op = match op_x {
            0 => RotShiftTable::from_i32(op_y).unwrap().into(),
            1 => AluOp::Bit,
            2 => AluOp::Res,
            3 | _ => AluOp::Set,
        };

        let alu_key = if is_hl {
            if let AluOp::Bit = alu_op {
                "alu(HL)(BITCB)"
            } else {
                "alu(HL)(CB)"
            }
        } else {
            "alur(CB)"
        };

        match op_x {
            0 => self.pla[alu_key]
                .remap_alu_placeholder(alu_op)
                .remap_lhs_reg(Register::from_single_table(op_z))
                .prune_bit(),
            _ => self.pla[alu_key]
                .remap_alu_placeholder(alu_op)
                .remap_lhs_reg(Register::from_single_table(op_z))
                .remap_i32_placeholder(op_y),
        }
        .compile()
    }
}
