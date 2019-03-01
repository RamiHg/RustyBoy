mod builder;
mod x_0_opcodes;
mod x_3_opcodes;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use super::alu;
use super::micro_code::*;
use super::register::Register;
use super::{Cpu, Error, Result};
use crate::memory::Memory;

use builder::Builder;
use Register::*;

/// Alu operations.
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

/// Decodes any 8-bit ALU operation in the form of OP A, r.
/// (e.g. ADD A, C).
fn decode_8bit_binary_alu(op_y: i32, op_z: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    debug_assert!(op_z <= 7 && op_z >= 0);
    let alu_table_entry = AluOpTable::from_i32(op_y).unwrap();
    let rhs = Register::from_single_table(op_z);
    if let HL = rhs {
        Builder::new()
            .nothing_then()
            .read_mem(TEMP_LOW, HL)
            .binary_op(alu_table_entry.into(), A, TEMP_LOW)
            .then_done()
    } else {
        Builder::new()
            .binary_op(alu_table_entry.into(), A, rhs)
            .then_done()
    }
}

fn condition_table_lookup(value: i32) -> (alu::Flags, bool) {
    match value {
        0 => (alu::Flags::ZERO, false),
        1 => (alu::Flags::ZERO, true),
        2 => (alu::Flags::CARRY, false),
        3 => (alu::Flags::CARRY, true),
        _ => panic!(),
    }
}

pub fn build_decode() -> Vec<MicroCode> {
    Builder::decode()
}

// TODO: Refactor into impl.
/// Executes a "Decode" special stage.
/// Assumes that the current micro-code has already read (and will increment) PC.
#[allow(clippy::cyclomatic_complexity)]
pub fn execute_decode_stage(cpu: &mut Cpu, memory: &Memory) -> Result<Option<SideEffect>> {
    // This method uses the amazing guide to decode instructions programatically:
    // https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html

    let opcode = cpu.registers.get(TEMP_LOW);
    // Deconstruct the op into its components.
    let op_z = opcode & 0b0000_0111;
    let op_y = (opcode & 0b0011_1000) >> 3;
    let op_x = (opcode & 0b1100_0000) >> 6;
    let op_q = op_y & 0b001;
    let op_p = (op_y & 0b110) >> 1;

    let err = Error::InvalidOpcode(opcode);

    // println!("Opcode: {:X?}", opcode);

    // Validating preconditions for documentation.
    debug_assert!(op_p <= 3 && op_p >= 0);
    debug_assert!(op_y <= 7 && op_y >= 0);
    debug_assert!(op_z <= 7 && op_z >= 0);
    debug_assert!(op_x <= 3);
    debug_assert!(op_q == 0 || op_q == 1);

    // At some point this will be converted into a static table with pre-defined instruction
    // opcodes. For now, it's ease of development.

    let mut micro_codes: Vec<MicroCode> = match op_x {
        // x = 0
        0 => x_0_opcodes::decode(opcode, op_z, op_y, op_x, op_q, op_p),
        // x = 1. LD r[y], r[z] with the exception of HALT.
        1 => match op_z {
            6 if op_y == 6 => Err(err), // HALT
            // z = 6. LD r[y], (HL)
            6 => Ok(Builder::new()
                .nothing_then()
                .read_mem(Register::from_single_table(op_y), HL)
                .then_done()),
            // LD r[y], r[z]
            _ => Ok(Builder::new()
                .alu_move(
                    Register::from_single_table(op_y),
                    Register::from_single_table(op_z),
                )
                .then_done()),
        },
        // x = 2. Binary Alu.
        2 => Ok(decode_8bit_binary_alu(op_y, op_z)),
        // x = 3. Assorted hodgepodge.
        3 | _ => x_3_opcodes::decode(opcode, op_z, op_y, op_x, op_q, op_p),
    }?;

    // The first microcode is special in the sense that it executes at the same time as the decode
    // step. This, however, means that it is limited only to its alu and register control stages.
    let setup_code = micro_codes.remove(0);
    assert!(
        setup_code.memory_stage.is_none(),
        "First micro-code of any instruction cannot contain any memory operations: {:?}.",
        setup_code.memory_stage
    );
    debug_assert!(setup_code.decoder_stage.is_none());
    let setup_result = setup_code.execute(cpu, memory)?;
    assert!(setup_result.side_effect.is_none());
    // Microcode cannot mark as done if there are other micro codes to execute!
    assert!(!setup_result.is_done || micro_codes.is_empty());
    if micro_codes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SideEffect::Decode(micro_codes)))
    }
}
