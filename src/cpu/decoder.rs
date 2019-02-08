use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::cpu::micro_code::*;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Error, Result};
use crate::memory::Memory;

/// Alu operations.
#[derive(FromPrimitive)]
pub enum AluOpTable {
    AddA,
    AdcA,
    Sub,
    SbcA,
    And,
    Xor,
    Or,
    Cp,
}

fn decode_8bit_binary_alu(op_y: i32, op_z: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    debug_assert!(op_z <= 7 && op_z >= 0);
    let alu_table_entry = AluOpTable::from_i32(op_y).unwrap();
    if op_z == 6 {
        Builder::new()
            .nothing_then()
            .read_mem(Register::TEMP_LOW, Register::HL)
            .binary_op(alu_table_entry.into(), Register::A, Register::TEMP_LOW)
            .then_done()
    } else {
        Builder::new()
            .binary_op(
                alu_table_entry.into(),
                Register::A,
                Register::from_single_table(op_z),
            )
            .then_done()
    }
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
    let op_z = opcode & 0b00000111;
    let op_y = (opcode & 0b00111000) >> 3;
    let op_x = (opcode & 0b11000000) >> 6;
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
                1 | _ => Err(err),
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
                    .move_reg(Register::TEMP_LOW, Register::C)
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
                    .move_reg(Register::TEMP_LOW, Register::C)
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
    assert!(setup_code.special_stage.is_none());
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
