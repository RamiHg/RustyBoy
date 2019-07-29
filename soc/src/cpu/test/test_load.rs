use num_traits::FromPrimitive;

use super::*;
use crate::cpu::register::Register;

use Register::*;

#[test]
fn test_ld_a_bc_de() {
    for &(op, reg) in &[(0x0A, BC), (0x1A, DE)] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg(reg, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0xEA)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_a_hli_hld() {
    for &(op, expected) in &[(0x2A, 0xD001), (0x3A, 0xCFFF)] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg(HL, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0xEA)
            .assert_reg_eq(HL, expected)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_reg_hl() {
    for &(op, reg) in &[(0x46, B), (0x4e, C), (0x56, D), (0x5e, E), (0x66, H), (0x6e, L), (0x7e, A)]
    {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg(HL, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(reg, 0xEA)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_reg_i8() {
    for (reg, &op) in [0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E, 0x3E].iter().enumerate() {
        with_default()
            .execute_instructions(&[op, 0xEA])
            .assert_reg_eq(Register::from_usize(reg).unwrap(), 0xEA)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_reg_i16() {
    for (reg, &op) in [0x01, 0x11, 0x21, 0x31].iter().enumerate() {
        with_default()
            .execute_instructions(&[op, 0xEF, 0xBE])
            .assert_reg_eq(Register::from_sp_pair_table(reg as i32), 0xBEEF)
            .assert_mcycles(3);
    }
}

#[test]
fn test_ld_reg_reg() {
    // Unfortunately, table is too long to hard-code. So we programatically encode it.
    for dest in 0..=7 {
        for src in 0..=7 {
            if src == 6 || dest == 6 {
                // Don't test storing into (HL), and don't repeat LD r, (HL) tests.
                break;
            }
            let op = 0b0100_0000 | (dest << 3) | src;
            with_default()
                .set_reg(Register::from_single_table(src), 0xEA)
                .execute_instructions(&[op as u8])
                .assert_reg_eq(Register::from_single_table(dest), 0xEA)
                .assert_mcycles(1);
        }
    }
}

#[test]
fn test_ld_a_m16() {
    with_default()
        .set_mem_8bit(0xD0DA, 0xEA)
        .execute_instructions(&[0xFA, 0xDA, 0xD0])
        .assert_reg_eq(Register::A, 0xEA)
        .assert_mcycles(4);
}

#[test]
fn test_ld_a_c() {
    // LD A, (0xFF00 + C)
    // NOTE: The pastraiser tables INCORRECTLY state that this instruction is 2 bytes long. It is,
    // in fact, one byte! Adding a regression test for this.
    with_default()
        .set_mem_8bit(0xFFAB, 0xEA)
        .set_reg(C, 0xAB)
        .execute_instructions(&[0xF2])
        .assert_reg_eq(A, 0xEA)
        .assert_mcycles(2);
    with_default()
        .set_mem_8bit(0xFFAB, 0xEA)
        .set_reg(C, 0xAB)
        .execute_instructions(&[0xF2, LD_A_IMM, 0xBE])
        .assert_reg_eq(A, 0xBE)
        .assert_mcycles(4);
}

#[test]
fn test_ld_assorted() {
    // LD A, (0xFF00 + n)
    with_default()
        .set_mem_8bit(0xFFAB, 0xEA)
        .execute_instructions(&[0xF0, 0xAB])
        .assert_reg_eq(A, 0xEA)
        .assert_mcycles(3);
    // LD SP, HL.
    with_default()
        .set_reg(HL, 0xBEEF)
        .execute_instructions(&[0xF9])
        .assert_reg_eq(SP, 0xBEEF)
        .assert_mcycles(2);
}
