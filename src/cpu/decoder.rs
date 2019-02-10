use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use super::alu;
use super::micro_code::*;
use super::register::Register;
use super::{Cpu, Error, Result};
use crate::memory::Memory;

/// Alu operations.
#[derive(FromPrimitive)]
pub enum AluOpTable {
    AddA,
    AdcA,
    SubA,
    SbcA,
    AndA,
    XorA,
    OrA,
    CpA,
}

/// Decodes any 8-bit ALU operation in the form of OP A, r.
/// (e.g. ADD A, C).
fn decode_8bit_binary_alu(op_y: i32, op_z: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    debug_assert!(op_z <= 7 && op_z >= 0);
    let alu_table_entry = AluOpTable::from_i32(op_y).unwrap();
    let rhs = Register::from_single_table(op_z);
    if let Register::HL = rhs {
        Builder::new()
            .nothing_then()
            .read_mem(Register::TEMP_LOW, Register::HL)
            .binary_op(alu_table_entry.into(), Register::A, Register::TEMP_LOW)
            .then_done()
    } else {
        Builder::new()
            .binary_op(alu_table_entry.into(), Register::A, rhs)
            .then_done()
    }
}

/// Decodes any 8-bit ALU operation in the form of OP A, imm8.
fn decode_8bit_binary_imm_alu(op_y: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    let alu_table_entry = AluOpTable::from_i32(op_y).unwrap();
    Builder::new()
        .nothing_then()
        .read_mem(Register::TEMP_LOW, Register::PC)
        .increment(IncrementerStage::PC)
        .binary_op(alu_table_entry.into(), Register::A, Register::TEMP_LOW)
        .then_done()
}

/// Decodes 8bit INC/DECs.
fn decode_incdec(unary_op: alu::UnaryOp, op_y: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    let register = Register::from_single_table(op_y);
    if let Register::HL = register {
        // Load the value from memory, increment it, store it again. Takes 3 mcycles.
        Builder::new()
            .nothing_then()
            .read_mem(Register::TEMP_LOW, Register::HL)
            .unary_op(unary_op, Register::TEMP_LOW)
            .then()
            .write_mem(Register::HL, Register::TEMP_LOW)
            .then_done()
    } else {
        Builder::new().unary_op(unary_op, register).then_done()
    }
}

/// After two registers are prepared for a 16bit add, this helper method actually performs the
/// add.
fn continue_16bit_add(builder: Builder, lhs: Register, rhs: Register) -> Builder {
    let (lhs_high, lhs_low) = lhs.decompose_pair();
    let (rhs_high, rhs_low) = rhs.decompose_pair();
    // We only want to preseve the zero bit of the flags register.
    let flags_restore = alu::FlagRegister::new(false, false, false, true);
    builder
        // Save the flags register to be able to preserve the zero bit.
        .move_reg(Register::TEMP_LOW, Register::F)
        .binary_op(alu::BinaryOp::Add, lhs_low, rhs_low)
        .then()
        .binary_op(alu::BinaryOp::Adc, lhs_high, rhs_high)
        // Restore the zero flags bit.
        .post_alu_restore_flags(Register::TEMP_LOW, flags_restore)
}

// TODO: Refactor into impl.
/// Executes a "Decode" special stage.
/// Assumes that the current micro-code has already read (and will increment) PC.
#[allow(clippy::cyclomatic_complexity)]
pub fn execute_decode_stage(cpu: &mut Cpu, memory: &Memory) -> Result<Option<SideEffect>> {
    // This method uses the amazing guide to decode instructions programatically:
    // https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html

    let opcode = cpu.registers.get(Register::TEMP_LOW);
    // Deconstruct the op into its components.
    let op_z = opcode & 0b0000_0111;
    let op_y = (opcode & 0b0011_1000) >> 3;
    let op_x = (opcode & 0b1100_0000) >> 6;
    let op_q = op_y & 0b001;
    let op_p = (op_y & 0b110) >> 1;

    let err = Error::InvalidOpcode(opcode);

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
        0 => match op_z {
            // z = 0. Relative jumps and other ops.
            0 => match op_y {
                1 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .write_mem(Register::TEMP, Register::SP_LOW)
                    .increment(IncrementerStage::TEMP)
                    .then()
                    .write_mem(Register::TEMP, Register::SP_HIGH)
                    .then_done()),
                _ => Err(err),
            },
            // z = 1
            1 => match op_q {
                // q = 0
                // LD (BC/DE/HL/SP)
                // Can probably just store straight into register pairs.
                0 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .move_reg(Register::from_sp_pair_table(op_p), Register::TEMP)
                    .increment(IncrementerStage::PC)
                    .then_done()),
                // q = 1. ADD HL, rr
                1 | _ => Ok(continue_16bit_add(
                    Builder::new(),
                    Register::HL,
                    Register::from_sp_pair_table(op_p),
                )
                .then_done()),
            },
            // z = 2
            2 => {
                let (pair, increment) = match op_p {
                    0 => (Register::BC, None),
                    1 => (Register::DE, None),
                    2 => (Register::HL, Some(IncrementerStage::HLI)),
                    3 | _ => (Register::HL, Some(IncrementerStage::HLD)),
                };
                match op_q {
                    // q = 0
                    // LD (BC/De/HLI/HLD), A
                    0 => Ok(Builder::new()
                        .nothing_then()
                        .write_mem(pair, Register::A)
                        .maybe_increment(increment)
                        .then_done()),
                    // q = 1
                    // LD A, (BC/DE/HLI/HLD)
                    1 | _ => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, pair)
                        .maybe_increment(increment)
                        .then_done()),
                }
            }
            // z = 4. 8-bit inc.
            4 => Ok(decode_incdec(alu::UnaryOp::Inc, op_y)),
            // z = 5. 8-bit dec.
            5 => Ok(decode_incdec(alu::UnaryOp::Dec, op_y)),
            // z = 6. 8-bit immediate loading.
            6 => match op_y {
                // LD (HL), imm8
                6 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .write_mem(Register::HL, Register::TEMP_LOW)
                    .then_done()),
                // LD r[y], imm8
                _ => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::from_single_table(op_y), Register::PC)
                    .increment(IncrementerStage::PC)
                    .then_done()),
            },
            _ => Err(err),
        },
        // x = 1. LD r[y], r[z] with the exception of HALT.
        1 => match op_z {
            6 if op_y == 6 => Err(err), // HALT
            // z = 6. LD r[y], (HL)
            6 => Ok(Builder::new()
                .nothing_then()
                .read_mem(Register::from_single_table(op_y), Register::HL)
                .then_done()),
            // LD r[y], r[z]
            _ => Ok(Builder::new()
                .move_reg(
                    Register::from_single_table(op_y),
                    Register::from_single_table(op_z),
                )
                .then_done()),
        },
        // x = 2. Binary Alu.
        2 => Ok(decode_8bit_binary_alu(op_y, op_z)),
        // x = 3. Assorted.
        3 | _ => match op_z {
            // z = 0. Conditional return, mem-mapped register loads, stack operations.
            0 => match op_y {
                // y = 4. LD (0xFF00 + n), A
                4 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .set_reg(Register::TEMP_HIGH, 0xFF)
                    .increment(IncrementerStage::PC)
                    .then()
                    .write_mem(Register::TEMP, Register::A)
                    .then_done()),
                // y = 5. ADD SP, i8.
                5 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .set_reg(Register::TEMP_HIGH, 0)
                    .increment(IncrementerStage::PC)
                    .binary_op(alu::BinaryOp::Add, Register::SP_LOW, Register::TEMP_LOW)
                    .then()
                    .sign_extend(Register::TEMP_LOW, Register::TEMP_LOW)
                    .then()
                    .binary_op(alu::BinaryOp::Adc, Register::SP_HIGH, Register::TEMP_LOW)
                    // Use TEMP_HIGH to clear out Z and N.
                    .post_alu_restore_flags(
                        Register::TEMP_HIGH,
                        alu::FlagRegister::new(false, false, true, true),
                    )
                    .then_done()),
                // y = 6. LD A, (0xFF00 + n)
                6 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .set_reg(Register::TEMP_HIGH, 0xFF)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::A, Register::TEMP)
                    .then_done()),
                _ => Err(err),
            },
            // z = 1.
            1 => {
                if op_q == 0 {
                    Err(err)
                } else {
                    match op_p {
                        // LD SP, HL
                        3 => Ok(Builder::new()
                            .move_reg(Register::SP_LOW, Register::L)
                            .then()
                            .move_reg(Register::SP_HIGH, Register::H)
                            .then_done()),
                        _ => Err(err),
                    }
                }
            }
            // z = 2.
            2 => match op_y {
                // y = 4. LD (0xFF00 + C), A
                4 => Ok(Builder::new()
                    .set_reg(Register::TEMP_HIGH, 0xFF)
                    .post_alu_move_reg(Register::TEMP_LOW, Register::C)
                    .then()
                    .write_mem(Register::TEMP, Register::A)
                    .then_done()),
                // y = 5. LD (nn), A
                5 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .write_mem(Register::TEMP, Register::A)
                    .then_done()),
                // y = 6. LD A, (0xFF00 + C)
                6 => Ok(Builder::new()
                    .set_reg(Register::TEMP_HIGH, 0xFF)
                    .post_alu_move_reg(Register::TEMP_LOW, Register::C)
                    .then()
                    .read_mem(Register::A, Register::TEMP)
                    .then_done()),
                // y = 7. LD A, (nn)
                7 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::A, Register::TEMP)
                    .then_done()),
                _ => Err(err),
            },
            // z = 6. Alu on immediate.
            6 => Ok(decode_8bit_binary_imm_alu(op_y)),
            _ => Err(err),
        },
    }?;

    // The first microcode is special in the sense that it executes at the same time as the decode
    // step. This, however, means that it is limited only to its alu and register control stages.
    let setup_code = micro_codes.remove(0);
    assert!(
        setup_code.memory_stage.is_none(),
        "First micro-code of any instruction cannot contain any memory operations: {:?}.",
        setup_code.memory_stage
    );
    assert!(
        setup_code.incrementer_stage.is_none(),
        "Cannot use incrementers during the first machine cycle: {:?}.",
        setup_code.incrementer_stage
    );
    debug_assert!(!setup_code.has_decode_stage);
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
