use std::collections::HashMap;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::cpu::alu;

use super::asm_compiler;
use super::csv_parser;
use super::micro_code::MicroCode;
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

impl Into<alu::BinaryOp> for AluOpTable {
    fn into(self) -> alu::BinaryOp {
        use alu::BinaryOp::*;
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

impl Decoder {
    pub fn new() -> Decoder {
        Decoder {
            pla: csv_parser::parse_csv(
                r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv",
            ),
        }
    }

    pub fn decode(&self, op: i32, _cpu: &mut Cpu, memory: &Memory) -> Vec<MicroCode> {
        self.decode_op(op)
    }

    fn decode_op(&self, opcode: i32) -> Vec<MicroCode> {
        // Deconstruct the op into its components.
        let op_z = opcode & 0b0000_0111;
        let op_y = (opcode & 0b0011_1000) >> 3;
        let op_x = (opcode & 0b1100_0000) >> 6;
        let op_q = op_y & 0b001;
        let op_p = (op_y & 0b110) >> 1;

        let hl_codes = match op_x {
            // x = 1
            1 if op_y != 6 && op_z != 6 => self.pla["LDr,r"]
                .flatten()
                .remap_lhs_reg(Register::from_single_table(op_y))
                .remap_rhs_reg(Register::from_single_table(op_z)),
            _ => panic!("Implement {:X?}", opcode),
        };

        /*
        let hl_codes = match op_x {
            // x = 0
            0 => match op_z {
                1 if op_q == 0 => mmap["LDrr,i16"]
                    .clone()
                    .replace_lhs(Register::from_sp_pair_table(op_p)),
                2 if op_q == 0 => match op_p {
                    0 => mmap["LD(rr),A"].clone().replace_lhs(Register::BC),
                    1 => mmap["LD(rr),A"].clone().replace_lhs(Register::DE),
                    _ => panic!(),
                },
                6 => mmap["LDr,i8"]
                    .clone()
                    .replace_lhs(Register::from_single_table(op_y)),
                _ => panic!(),
            },
            // x = 1
            1 if op_y != 6 && op_z != 6 => mmap["LDr,r"]
                .clone()
                .replace_source(
                    OpSource::Lhs,
                    OpSource::Register(Register::from_single_table(op_y)),
                )
                .replace_source(
                    OpSource::Rhs,
                    OpSource::Register(Register::from_single_table(op_z)),
                ),
            // x = 3
            3 => match op_z {
                // z = 6. ALU
                6 => mmap["aluA,i8"]
                    .clone()
                    .replace_binary_op(AluOpTable::from_i32(op_y).unwrap().into()),
            },
            _ => panic!("Implement op {:X?}.", opcode),
        };

        hl_codes
            .0
            .iter()
            .flat_map(|x| MicroCode::from(x).to_vec())
            .collect()
            */
        hl_codes.ops()
    }
}
