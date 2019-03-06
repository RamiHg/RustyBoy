use std::collections::HashMap;

use super::{
    loader::{self, HLMicroCodeArray, OpSource},
    MicroCode,
};
use crate::{
    cpu::{register::Register, Cpu},
    memory::Memory,
};

fn decode_op(opcode: i32, mmap: &HashMap<String, HLMicroCodeArray>) -> Vec<MicroCode> {
    // Deconstruct the op into its components.
    let op_z = opcode & 0b0000_0111;
    let op_y = (opcode & 0b0011_1000) >> 3;
    let op_x = (opcode & 0b1100_0000) >> 6;
    let op_q = op_y & 0b001;
    let op_p = (op_y & 0b110) >> 1;

    let hl_codes = match op_x {
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
        _ => panic!("Implement op {:X?}.", opcode),
    };

    hl_codes
        .0
        .iter()
        .flat_map(|x| MicroCode::from(x).to_vec())
        .collect()
}

pub fn decode(op: i32, _cpu: &mut Cpu, memory: &Memory) -> Vec<MicroCode> {
    let mmap = loader::load(r"C:\Users\Ramy\Downloads\CPU Design - Copy of Sheet7.csv");
    decode_op(op, &mmap)
}
