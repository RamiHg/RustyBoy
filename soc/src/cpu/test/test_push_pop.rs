use super::*;
use crate::cpu::register::Register;

use Register::*;

#[test]
fn test_push() {
    for &(op, reg) in &[(0xC5, BC), (0xD5, DE), (0xE5, HL)] {
        with_default()
            .set_reg(SP, 0xFFFF)
            .set_reg(reg, 0xBEEF)
            .execute_instructions(&[op])
            .assert_reg_eq(SP, 0xFFFD)
            .assert_mem_16bit_eq(0xFFFD, 0xBEEF)
            .assert_mcycles(4);
    }
    with_default()
        .set_reg(SP, 0xFFFF)
        .set_reg(AF, 0xBEEF)
        .execute_instructions(&[0xF5])
        .assert_reg_eq(SP, 0xFFFD)
        .assert_mem_16bit_eq(0xFFFD, 0xBEE0)
        .assert_mcycles(4);
}

#[test]
fn test_pop() {
    for &(op, reg) in &[(0xC1, BC), (0xD1, DE), (0xE1, HL)] {
        with_default()
            .set_reg(SP, 0xFF80)
            .set_mem_range(0xFF80, &[0xEF, 0xBE])
            .execute_instructions(&[op])
            .assert_reg_eq(SP, 0xFF82)
            .assert_reg_eq(reg, 0xBEEF)
            .assert_mcycles(3);
    }
    with_default()
        .set_reg(SP, 0xFF80)
        .set_mem_range(0xFF80, &[0xEF, 0xBE])
        .execute_instructions(&[0xF1])
        .assert_reg_eq(SP, 0xFF82)
        .assert_reg_eq(AF, 0xBEE0)
        .assert_mcycles(3);
}
