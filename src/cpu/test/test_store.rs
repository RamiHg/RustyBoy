/*
 * TODO:
 * ADD HL, rp[p]
 * ADD SP, d
 * LD HL, SP+d
 */

use super::*;
use crate::cpu::register::Register;

use Register::*;

#[test]
fn test_ld_bc_de_a() {
    for &(op, reg) in &[(0x02, BC), (0x12, DE)] {
        with_default()
            .set_reg(reg, 0xD000)
            .set_reg(A, 0xEA)
            .execute_instructions(&[op])
            .assert_mem_8bit_eq(0xD000, 0xEA)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_hli_hld_a() {
    for &(op, expected) in &[(0x22, 0xD001), (0x32, 0xCFFF)] {
        with_default()
            .set_reg(HL, 0xD000)
            .set_reg(A, 0xEA)
            .execute_instructions(&[op])
            .assert_reg_eq(HL, expected)
            .assert_mem_8bit_eq(0xD000, 0xEA)
            .assert_mcycles(2);
    }
}

#[test]
fn test_ld_assorted() {
    // LD (0xFF00 + n), A
    with_default()
        .set_reg(A, 0xEA)
        .execute_instructions(&[0xE0, 0xAB])
        .assert_mem_8bit_eq(0xFFAB, 0xEA)
        .assert_mcycles(3);
    // LD (0xFF00 + C), A
    with_default()
        .set_reg(A, 0xEA)
        .set_reg(C, 0xAB)
        .execute_instructions(&[0xE2])
        .assert_mem_8bit_eq(0xFFAB, 0xEA)
        .assert_mcycles(2);
    // LD (nn), A
    with_default()
        .set_reg(A, 0xEA)
        .execute_instructions(&[0xEA, 0xAB, 0xFF])
        .assert_mem_8bit_eq(0xFFAB, 0xEA)
        .assert_mcycles(4);
    // LD (HL), n
    with_default()
        .set_reg(HL, 0xFFAB)
        .execute_instructions(&[0x36, 0xEA])
        .assert_mem_8bit_eq(0xFFAB, 0xEA)
        .assert_mcycles(3);
    // LD (nn), SP
    with_default()
        .set_reg(SP, 0xBEEF)
        .execute_instructions(&[0x08, 0xAB, 0xFF])
        .assert_mem_16bit_eq(0xFFAB, 0xBEEF)
        .assert_mcycles(5);
}
