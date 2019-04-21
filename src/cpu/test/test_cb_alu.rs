use super::*;
use crate::cpu::register::Register::*;

fn cond_carry(cond: bool) -> Flags {
    if cond { Flags::CARRY } else { Flags::empty() }
}

fn setup_op(op: u8, reg: Register, val: i32, flags: Flags, expected: i32, expected_flags: Flags) {
    let is_hl = reg == Register::HL;
    let test = if is_hl {
        with_default().set_mem_8bit(0xD000, val).set_reg(HL, 0xD000)
    } else {
        with_default().set_reg(reg, val)
    }
    .set_reg(F, flags.bits())
    .execute_instructions(&[0xCB, op])
    .assert_flags(expected_flags)
    .assert_mcycles(if is_hl { 4 } else { 2 });
    if is_hl {
        test.assert_mem_8bit_eq(0xD000, expected);
    } else {
        test.assert_reg_eq(reg, expected);
    }
}

fn test_rotate_op(
    op: u8,
    reg: Register,
    val: i32,
    carry: bool,
    expected: i32,
    expected_carry: bool,
) {
    setup_op(
        op,
        reg,
        val,
        cond_carry(carry),
        expected,
        cond_carry(expected_carry),
    );
    // Make sure Z flag is on when result is 0.
    setup_op(op, reg, 0, Flags::empty(), 0, Flags::ZERO);
}

#[test]
fn test_rlc() {
    for (&op, &reg) in [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b0100_1011, true);
        test_rotate_op(op, reg, 0b0010_0101, true, 0b0100_1010, false);
    }
}

#[test]
fn test_rrc() {
    for (&op, &reg) in [0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b1101_0010, true);
        test_rotate_op(op, reg, 0b1010_0100, true, 0b0101_0010, false);
    }
}

#[test]
fn test_rl() {
    for (&op, &reg) in [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b0100_1010, true);
        test_rotate_op(op, reg, 0b0010_0101, true, 0b0100_1011, false);
    }
}

#[test]
fn test_rr() {
    for (&op, &reg) in [0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b0101_0010, true);
        test_rotate_op(op, reg, 0b1010_0100, true, 0b1101_0010, false);
    }
}

#[test]
fn test_sla() {
    for (&op, &reg) in [0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, true, 0b0100_1010, true);
        test_rotate_op(op, reg, 0b0010_0101, false, 0b0100_1010, false);
    }
}

#[test]
fn test_sra() {
    for (&op, &reg) in [0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b1101_0010, true);
        test_rotate_op(op, reg, 0b0010_0100, true, 0b0001_0010, false);
    }
}

#[test]
fn test_swap() {
    for (&op, &reg) in [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, false, 0b0101_1010, false);
    }
}

#[test]
fn test_srl() {
    for (&op, &reg) in [0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F]
        .iter()
        .zip(UNARY_SOURCES.iter())
    {
        test_rotate_op(op, reg, 0b1010_0101, true, 0b0101_0010, true);
        test_rotate_op(op, reg, 0b1010_0100, false, 0b0101_0010, false);
    }
}

#[test]
fn test_bit() {
    for bit in 0..8 {
        let value = 1 << bit;
        for (&reg, reg_index) in UNARY_SOURCES.iter().zip(0..8) {
            let op = 0x40 + 8 * bit + reg_index;
            setup_op(
                op,
                reg,
                value,
                Flags::CARRY | Flags::SUB | Flags::ZERO,
                value,
                Flags::HCARRY | Flags::CARRY,
            );
            setup_op(
                op,
                reg,
                0,
                Flags::CARRY | Flags::SUB | Flags::ZERO,
                0,
                Flags::HCARRY | Flags::CARRY | Flags::ZERO,
            );
        }
    }
}

#[test]
fn test_res() {
    let all_flags = Flags::CARRY | Flags::SUB | Flags::ZERO | Flags::HCARRY;
    for bit in 0..8 {
        for (&reg, reg_index) in UNARY_SOURCES.iter().zip(0..8) {
            let op = 0x80 + 8 * bit + reg_index;
            let expected = (!(1 << bit)) & 0xFF;
            setup_op(op, reg, 0xFF, all_flags, expected, all_flags);
        }
    }
}

#[test]
fn test_set() {
    let all_flags = Flags::CARRY | Flags::SUB | Flags::ZERO | Flags::HCARRY;
    for bit in 0..8 {
        for (&reg, reg_index) in UNARY_SOURCES.iter().zip(0..8) {
            let op = 0xC0 + 8 * bit + reg_index;
            let expected = 1 << bit;
            setup_op(op, reg, 0, all_flags, expected, all_flags);
        }
    }
}
