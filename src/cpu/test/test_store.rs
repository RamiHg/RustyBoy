/*
 * TODO:
 * LD (HL), i8
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
            .assert_mem_8bit_eq(0xD000, 0xEA);
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
            .assert_mem_8bit_eq(0xD000, 0xEA);
    }
}
