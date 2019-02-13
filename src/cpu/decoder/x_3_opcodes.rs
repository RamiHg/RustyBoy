/*
 * Decoding instructions where x = 3.
 * See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
 */

use num_traits::FromPrimitive;

use super::builder::Builder;
use crate::cpu;

use super::condition_table_lookup;
use super::AluOpTable;
use cpu::alu::{self, Flags};
use cpu::micro_code::*;
use cpu::register::Register;
use cpu::{Error, Result};

use Register::*;

pub fn decode(
    opcode: i32,
    op_z: i32,
    op_y: i32,
    op_x: i32,
    op_q: i32,
    op_p: i32,
) -> Result<Vec<MicroCode>> {
    let err = Err(Error::InvalidOpcode(opcode));
    match op_z {
        // z = 0. Conditional return, mem-mapped register loads, stack operations.
        0 => match op_y {
            // y = 4. LD (0xFF00 + n), A
            4 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .set_reg(TEMP_HIGH, 0xFF)
                .increment(PC)
                .then()
                .write_mem(TEMP, A)
                .then_done()),
            // y = 5. ADD SP, i8.
            5 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .set_reg(TEMP_HIGH, 0)
                .increment(PC)
                .binary_op(alu::BinaryOp::Add, SP_LOW, TEMP_LOW)
                .then()
                .pre_alu_sign_extend(TEMP_LOW, TEMP_LOW)
                .then()
                .binary_op(alu::BinaryOp::Adc, SP_HIGH, TEMP_LOW)
                // Use TEMP_HIGH to clear out Z and N.
                .post_alu_restore_flags(TEMP_HIGH, Flags::ZERO | Flags::SUB)
                .then_done()),
            // y = 6. LD A, (0xFF00 + n)
            6 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .set_reg(TEMP_HIGH, 0xFF)
                .increment(PC)
                .then()
                .read_mem(A, TEMP)
                .then_done()),
            _ => err,
        },
        // z = 1.
        1 => {
            if op_q == 0 {
                err
            } else {
                match op_p {
                    // p = 0. RET
                    0 => Ok(decode_ret()),
                    // p = 2. JP HL
                    2 => Ok(decode_jump_to_hl()),
                    // LD SP, HL
                    3 => Ok(Builder::new()
                        .alu_move(SP_LOW, L)
                        .then()
                        .alu_move(SP_HIGH, H)
                        .then_done()),
                    _ => err,
                }
            }
        }
        // z = 2.
        2 => match op_y {
            // y = 0..3. JP cc, nn
            0..=3 => Ok(decode_conditional_jump(op_y)),
            // y = 4. LD (0xFF00 + C), A
            4 => Ok(Builder::new()
                .set_reg(TEMP_HIGH, 0xFF)
                .alu_move(TEMP_LOW, C)
                .then()
                .write_mem(TEMP, A)
                .then_done()),
            // y = 5. LD (nn), A
            5 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .increment(PC)
                .then()
                .read_mem(TEMP_HIGH, PC)
                .increment(PC)
                .then()
                .write_mem(TEMP, A)
                .then_done()),
            // y = 6. LD A, (0xFF00 + C)
            6 => Ok(Builder::new()
                .set_reg(TEMP_HIGH, 0xFF)
                .alu_move(TEMP_LOW, C)
                .then()
                .read_mem(A, TEMP)
                .then_done()),
            // y = 7. LD A, (nn)
            7 | _ => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .increment(PC)
                .then()
                .read_mem(TEMP_HIGH, PC)
                .increment(PC)
                .then()
                .read_mem(A, TEMP)
                .then_done()),
        },
        // z = 3. Jumps and interrupt control.
        3 => match op_y {
            0 => Ok(decode_jump()),
            _ => err,
        },
        // z = 5. Call and push.
        5 => match op_q {
            1 => match op_p {
                0 => Ok(decode_call()),
                _ => err,
            },
            _ => err,
        },
        // z = 6. Alu on immediate.
        6 => Ok(decode_8bit_binary_imm_alu(op_y)),
        _ => err,
    }
}

/// Decodes any 8-bit ALU operation in the form of OP A, imm8.
fn decode_8bit_binary_imm_alu(op_y: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    let alu_table_entry = AluOpTable::from_i32(op_y).unwrap();
    Builder::new()
        .nothing_then()
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .binary_op(alu_table_entry.into(), A, TEMP_LOW)
        .then_done()
}

/// Decodes JP nn.
fn decode_jump() -> Vec<MicroCode> {
    Builder::new()
        .nothing_then()
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .then()
        .read_mem(TEMP_HIGH, PC)
        // JP nn actually takes 4 cycles. There is an internal delay before jumping.
        .then()
        .alu_move(PC, TEMP)
        .then_done()
}

/// Decodes JP cc, nn.
fn decode_conditional_jump(op_y: i32) -> Vec<MicroCode> {
    let (flags, is_set) = condition_table_lookup(op_y);
    Builder::new()
        .nothing_then()
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .then()
        .read_mem(TEMP_HIGH, PC)
        .increment(PC)
        // Like JP, JP CC takes 4 cycles. So we add a useless delay here.
        .then()
        .alu_move(PC, TEMP)
        .on_condition(flags, is_set)
        .then_done()
}

/// Decodes JP HL.
fn decode_jump_to_hl() -> Vec<MicroCode> {
    // Hacky, but decrement PC to negate the PC increment in the decode instruction.
    Builder::new().alu_move(PC, HL).decrement(PC).then_done()
}

/// Decodes CALL nn.
fn decode_call() -> Vec<MicroCode> {
    Builder::new()
        .nothing_then()
        // M = 2
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .then()
        // M = 3
        .read_mem(TEMP_HIGH, PC)
        .increment(PC)
        .then()
        // M = 4
        .decrement(SP)
        .then()
        // M = 5
        .write_mem(SP, PC_HIGH)
        .decrement(SP)
        .then()
        // M = 6
        .write_mem(SP, PC_LOW)
        .move_reg(PC, TEMP)
        .then_done()
}

/// Decodes CALL cc nn.
fn decode_call_cc(op_y: i32) -> Vec<MicroCode> {
    let (flags, is_set) = condition_table_lookup(op_y);
    Builder::new()
        .nothing_then()
        // M = 2
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .then()
        // M = 3
        .read_mem(TEMP_HIGH, PC)
        .increment(PC)
        .then()
        // M = 4
        .alu_move(PC, TEMP)
        .on_condition(flags, is_set)
        // End the instruction early if the condition is not met.
        .conditional_done(flags, !is_set)
        // Otherwise, continue the call.
        .then_done()
    // M = 5
}

/// Decodes RET.
fn decode_ret() -> Vec<MicroCode> {
    Builder::new()
        .nothing_then()
        // M = 2
        .read_mem(TEMP_LOW, SP)
        .increment(SP)
        .then()
        // M = 3
        .read_mem(TEMP_HIGH, SP)
        .increment(SP)
        .then()
        // M = 4
        .move_reg(PC, TEMP)
        .then_done()
}
