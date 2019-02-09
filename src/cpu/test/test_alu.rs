use super::*;
use crate::cpu::register::Register;

use Register::*;

const SOURCES: [Register; 8] = [B, C, D, E, H, L, HL, PC];
// Helper constants so that assert_flags isn't a list of bools. (TODO: Make struct instead?)
const ZERO: bool = true;
const NZERO: bool = false;
const SUB: bool = true;
const NSUB: bool = false;
const HCY: bool = true;
const NHCY: bool = false;
const CY: bool = true;
const NCY: bool = false;

fn setup_lhs_rhs_carry(
    op: i32,
    lhs_value: i32,
    rhs: Register,
    rhs_value: i32,
    carry: bool,
) -> TestContext {
    let mut ops = vec![op as u8];
    // If value is expected as immediate, push it on the instruction list.
    if let PC = rhs {
        ops.push(rhs_value as u8);
    }
    if let HL = rhs {
        with_default()
            .set_mem_8bit(0xD000, rhs_value)
            .set_reg(HL, 0xD000)
    } else if let PC = rhs {
        with_default()
    } else {
        with_default().set_reg(rhs, rhs_value)
    }
    .set_reg(A, lhs_value)
    .set_carry(carry)
    .execute_instructions(&ops)
}

fn setup_lhs_rhs(op: i32, lhs_value: i32, rhs: Register, rhs_value: i32) -> TestContext {
    setup_lhs_rhs_carry(op, lhs_value, rhs, rhs_value, false)
}

fn cycles_for_source(src: Register) -> i32 {
    match src {
        PC | HL => 2,
        _ => 1,
    }
}

#[test]
fn test_add_a() {
    for (&op, &src) in [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0xC6]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0x7F, src, 1)
            .assert_reg_eq(A, 0x80)
            .assert_flags(NZERO, NSUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x80, src, 0x80)
            .assert_reg_eq(A, 0)
            .assert_flags(ZERO, NSUB, NHCY, CY)
            .assert_mcycles(cycles_for_source(src));
    }
    // ADD A, A
    with_default()
        .set_reg(A, 0x8)
        .execute_instructions(&[0x87])
        .assert_reg_eq(A, 0x10)
        .assert_flags(NZERO, NSUB, HCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_adc_a() {
    for (&op, &src) in [0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E, 0xCE]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs_carry(op, 0x7E, src, 1, true)
            .assert_reg_eq(A, 0x80)
            .assert_flags(NZERO, NSUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs_carry(op, 0x80, src, 0x80, true)
            .assert_reg_eq(A, 1)
            .assert_flags(NZERO, NSUB, NHCY, CY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs_carry(op, 0x80, src, 0x80, false)
            .assert_reg_eq(A, 0)
            .assert_flags(ZERO, NSUB, NHCY, CY)
            .assert_mcycles(cycles_for_source(src));
    }
    // ADC A, A
    with_default()
        .set_reg(A, 0x8)
        .set_carry(true)
        .execute_instructions(&[0x8F])
        .assert_reg_eq(A, 0x11)
        .assert_flags(NZERO, NSUB, HCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_sub_a_r() {
    for (&op, &src) in [0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0xD6]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0xF0, src, 0x1)
            .assert_reg_eq(A, 0xEF)
            .assert_flags(NZERO, SUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x01, src, 0x02)
            .assert_reg_eq(A, 0xFF)
            .assert_flags(NZERO, SUB, HCY, CY)
            .assert_mcycles(cycles_for_source(src));
    }
    // SUB A, A
    with_default()
        .set_reg(A, 0x8)
        .execute_instructions(&[0x97])
        .assert_reg_eq(A, 0)
        .assert_flags(ZERO, SUB, NHCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_sbc_a_r() {
    for (&op, &src) in [0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D, 0x9E, 0xDE]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs_carry(op, 0xF0, src, 0x0, true)
            .assert_reg_eq(A, 0xEF)
            .assert_flags(NZERO, SUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs_carry(op, 0x01, src, 0x01, true)
            .assert_reg_eq(A, 0xFF)
            .assert_flags(NZERO, SUB, HCY, CY)
            .assert_mcycles(cycles_for_source(src));
    }
    // SBC A, A
    with_default()
        .set_reg(A, 0x8)
        .set_carry(true)
        .execute_instructions(&[0x9F])
        .assert_reg_eq(A, 0xFF)
        .assert_flags(NZERO, SUB, HCY, CY)
        .assert_mcycles(1);
}

#[test]
fn test_and_a_r() {
    for (&op, &src) in [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xE6]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0x0B, src, 0x05)
            .assert_reg_eq(A, 0x01)
            .assert_flags(NZERO, NSUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x0A, src, 0x05)
            .assert_reg_eq(A, 0)
            .assert_flags(ZERO, NSUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
    }
    // AND A, A
    with_default()
        .set_reg(A, 0x08)
        .execute_instructions(&[0xA7])
        .assert_reg_eq(A, 0x08)
        .assert_flags(NZERO, NSUB, HCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_or_a_r() {
    for (&op, &src) in [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xF6]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0xAA, src, 0x55)
            .assert_reg_eq(A, 0xFF)
            .assert_flags(NZERO, NSUB, NHCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x0, src, 0x0)
            .assert_reg_eq(A, 0x0)
            .assert_flags(ZERO, NSUB, NHCY, NCY)
            .assert_mcycles(cycles_for_source(src));
    }
    // OR A, A
    with_default()
        .set_reg(A, 0x08)
        .execute_instructions(&[0xB7])
        .assert_reg_eq(A, 0x08)
        .assert_flags(NZERO, NSUB, NHCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_xor_a_r() {
    for (&op, &src) in [0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE, 0xEE]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0xAA, src, 0x55)
            .assert_reg_eq(A, 0xFF)
            .assert_flags(NZERO, NSUB, NHCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x11, src, 0x55)
            .assert_reg_eq(A, 0x44)
            .assert_flags(NZERO, NSUB, NHCY, NCY)
            .assert_mcycles(cycles_for_source(src));
    }
    // XOR A, A
    with_default()
        .set_reg(A, 0x08)
        .execute_instructions(&[0xAF])
        .assert_reg_eq(A, 0x0)
        .assert_flags(ZERO, NSUB, NHCY, NCY)
        .assert_mcycles(1);
}

#[test]
fn test_cp_a_r() {
    for (&op, &src) in [0xB8, 0xB9, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xFE]
        .iter()
        .zip(SOURCES.iter())
    {
        setup_lhs_rhs(op, 0xF0, src, 0x1)
            .assert_reg_eq(A, 0xF0)
            .assert_flags(NZERO, SUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_lhs_rhs(op, 0x01, src, 0x02)
            .assert_reg_eq(A, 0x01)
            .assert_flags(NZERO, SUB, HCY, CY)
            .assert_mcycles(cycles_for_source(src));
    }
    // SUB A, A
    with_default()
        .set_reg(A, 0x8)
        .execute_instructions(&[0xBF])
        .assert_reg_eq(A, 0x8)
        .assert_flags(ZERO, SUB, NHCY, NCY)
        .assert_mcycles(1);
}
