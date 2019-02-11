use super::*;
use crate::cpu::register::Register::*;

const SP_PAIRS: [Register; 4] = [BC, DE, HL, SP];

#[test]
fn test_add_hl_rr() {
    for (&op, &src) in [0x09, 0x19, 0x39].iter().zip([BC, DE, SP].iter()) {
        with_default()
            .set_reg(HL, 0xFF)
            .set_reg(src, 0xFF)
            .set_zero(true)
            .execute_instructions(&[op])
            .assert_reg_eq(HL, 0x1FE)
            .assert_flags(Flags::ZERO)
            .assert_mcycles(2);
        with_default()
            .set_reg(HL, 0xFFFE)
            .set_reg(src, 0x2)
            .set_zero(false)
            .execute_instructions(&[op])
            .assert_reg_eq(HL, 0x0)
            .assert_flags(Flags::HCARRY | Flags::CARRY)
            .assert_mcycles(2);
    }
    with_default()
        .set_reg(HL, 0xFF)
        .execute_instructions(&[0x29])
        .assert_reg_eq(HL, 0x1FE)
        .assert_flags(Flags::empty())
        .assert_mcycles(2);
}

#[test]
fn test_add_sp_i8() {
    with_default()
        .set_reg(SP, 0xFF00)
        .set_zero(true)
        .set_sub(true)
        .execute_instructions(&[0xE8, 0xFF])
        .assert_reg_eq(SP, 0xFEFF)
        .assert_flags(Flags::HCARRY | Flags::CARRY)
        .assert_mcycles(4);
    with_default()
        .set_reg(SP, 0xFF00)
        .set_zero(false)
        .set_sub(false)
        .execute_instructions(&[0xE8, 0x1F])
        .assert_reg_eq(SP, 0xFF1F)
        .assert_flags(Flags::empty())
        .assert_mcycles(4);
}

#[test]
fn test_inc_rr() {
    for (&op, &reg) in [0x03, 0x13, 0x23, 0x33].iter().zip(SP_PAIRS.iter()) {
        with_default()
            .set_reg(reg, 0xFFFF)
            .set_flag(Flags::ZERO | Flags::SUB, true)
            .execute_instructions(&[op])
            .assert_reg_eq(reg, 0)
            .assert_flags(Flags::ZERO | Flags::SUB)
            .assert_mcycles(2);
    }
}

#[test]
fn test_dec_rr() {
    for (&op, &reg) in [0x0B, 0x1B, 0x2B, 0x3B].iter().zip(SP_PAIRS.iter()) {
        with_default()
            .set_reg(reg, 0)
            .set_flag(Flags::ZERO, true)
            .execute_instructions(&[op])
            .assert_reg_eq(reg, 0xFFFF)
            .assert_flags(Flags::ZERO)
            .assert_mcycles(2);
    }
}
