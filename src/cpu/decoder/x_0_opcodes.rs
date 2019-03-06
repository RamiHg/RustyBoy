/*
 * Decoding instructions where x = 0.
 * See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
 */

use super::builder::Builder;
use crate::cpu;

use super::condition_table_lookup;
use cpu::{
    alu::{self, Flags},
    micro_code::*,
    register::Register,
    Error, Result,
};

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
        // z = 0. Relative jumps and other ops.
        0 => match op_y {
            1 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .increment(PC)
                .then()
                .read_mem(TEMP_HIGH, PC)
                .increment(PC)
                .then()
                .write_mem(TEMP, SP_LOW)
                .increment(TEMP)
                .then()
                .write_mem(TEMP, SP_HIGH)
                .then_done()),
            // y = 3. JR d
            3 => Ok(decode_jr()),
            // y = 4..7. JR cc, d
            4..=7 => Ok(decode_jr_cc(op_y - 4)),
            _ => err,
        },
        // z = 1
        1 => match op_q {
            // q = 0
            // LD (BC/DE/HL/SP)
            // Can probably just store straight into register pairs.
            0 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .increment(PC)
                .then()
                .read_mem(TEMP_HIGH, PC)
                .alu_move(Register::from_sp_pair_table(op_p), TEMP)
                .increment(PC)
                .then_done()),
            // q = 1. ADD HL, rr
            1 | _ => Ok(decode_16bit_add(op_p)),
        },
        // z = 2
        2 => {
            let (pair, increment) = match op_p {
                0 => (BC, None),
                1 => (DE, None),
                2 => (HL, Some(IncrementerStage::Increment(HL))),
                3 | _ => (HL, Some(IncrementerStage::Decrement(HL))),
            };
            match op_q {
                // q = 0
                // LD (BC/De/HLI/HLD), A
                0 => Ok(Builder::new()
                    .nothing_then()
                    .write_mem(pair, A)
                    .maybe_increment(increment)
                    .then_done()),
                // q = 1
                // LD A, (BC/DE/HLI/HLD)
                1 | _ => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(A, pair)
                    .maybe_increment(increment)
                    .then_done()),
            }
        }
        // z = 3. INC/DEC rr.
        3 => Ok(decode_16bit_incdec(op_q, op_p)),
        // z = 4. 8-bit inc.
        4 => Ok(decode_incdec(alu::UnaryOp::Inc, op_y)),
        // z = 5. 8-bit dec.
        5 => Ok(decode_incdec(alu::UnaryOp::Dec, op_y)),
        // z = 6. 8-bit immediate loading.
        6 => match op_y {
            // LD (HL), imm8
            6 => Ok(Builder::new()
                .nothing_then()
                .read_mem(TEMP_LOW, PC)
                .increment(PC)
                .then()
                .write_mem(HL, TEMP_LOW)
                .then_done()),
            // LD r[y], imm8
            _ => Ok(Builder::new()
                .nothing_then()
                .read_mem(Register::from_single_table(op_y), PC)
                .increment(PC)
                .then_done()),
        },
        _ => err,
    }
}

/// Decodes ADD HL, rr.
fn decode_16bit_add(op_p: i32) -> Vec<MicroCode> {
    let rhs = Register::from_sp_pair_table(op_p);
    let (rhs_high, rhs_low) = rhs.decompose_pair();
    // We only want to preseve the zero bit of the flags register.
    let flags_restore = Flags::ZERO;
    Builder::new()
        // Save the flags register to be able to preserve the zero bit.
        .move_reg(TEMP_LOW, F)
        .binary_op(alu::BinaryOp::Add, L, rhs_low)
        .then()
        .binary_op(alu::BinaryOp::Adc, H, rhs_high)
        // Restore the zero flags bit.
        .post_alu_restore_flags(TEMP_LOW, flags_restore)
        .then_done()
}

/// Decodes INC/DEC rr.
fn decode_16bit_incdec(op_q: i32, op_p: i32) -> Vec<MicroCode> {
    let register = Register::from_sp_pair_table(op_p);
    Builder::new()
        .nothing_then()
        .incrementer_stage(if op_q == 0 {
            IncrementerStage::Increment(register)
        } else {
            IncrementerStage::Decrement(register)
        })
        .then_done()
}

/// Decodes 8bit INC/DECs.
fn decode_incdec(unary_op: alu::UnaryOp, op_y: i32) -> Vec<MicroCode> {
    debug_assert!(op_y <= 7 && op_y >= 0);
    let register = Register::from_single_table(op_y);
    if let HL = register {
        // Load the value from memory, increment it, store it again. Takes 3 mcycles.
        Builder::new()
            .nothing_then()
            .read_mem(TEMP_LOW, HL)
            .unary_op(unary_op, TEMP_LOW)
            .then()
            .write_mem(HL, TEMP_LOW)
            .then_done()
    } else {
        Builder::new().unary_op(unary_op, register).then_done()
    }
}

// Decodes JR n.
fn decode_jr() -> Vec<MicroCode> {
    Builder::new()
        .nothing_then()
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        // Save flags.
        .move_reg(TEMP_HIGH, F)
        .binary_op(alu::BinaryOp::Add, PC_LOW, TEMP_LOW)
        .then()
        .pre_alu_sign_extend(TEMP_LOW, TEMP_LOW)
        .binary_op(alu::BinaryOp::Adc, PC_HIGH, TEMP_LOW)
        .post_alu_move_reg(F, TEMP_HIGH)
        .then_done()
}

fn decode_jr_cc(op_y: i32) -> Vec<MicroCode> {
    let (flags, is_set) = condition_table_lookup(op_y);
    Builder::new()
        // Save flags.
        .move_reg(TEMP_HIGH, F)
        .then()
        .read_mem(TEMP_LOW, PC)
        .increment(PC)
        .binary_op(alu::BinaryOp::Add, PC_LOW, TEMP_LOW)
        .on_condition(flags, is_set)
        .then()
        .pre_alu_sign_extend(TEMP_LOW, TEMP_LOW)
        .binary_op(alu::BinaryOp::Adc, PC_HIGH, TEMP_LOW)
        .on_condition(flags, is_set)
        // Restore flags.
        .post_alu_move_reg(F, TEMP_HIGH)
        .then_done()
}
