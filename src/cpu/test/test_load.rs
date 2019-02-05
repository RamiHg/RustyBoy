use num_traits::FromPrimitive;

use super::register::Register;
use super::*;

use Register::*;

#[test]
fn test_ld_a_bc_de() {
    for &(op, reg) in &[(0x0A, BC), (0x1A, DE)] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg(reg, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0xEA);
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
            .assert_reg_eq(HL, expected);
    }
}

#[test]
fn test_ld_reg_hl() {
    for &(op, reg) in &[
        (0x46, B),
        (0x4e, C),
        (0x56, D),
        (0x5e, E),
        (0x66, H),
        (0x6e, L),
        (0x7e, A),
    ] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg(HL, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(reg, 0xEA);
    }
}

#[test]
fn test_ld_reg_i8() {
    for (reg, &op) in [0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E].iter().enumerate() {
        with_default()
            .execute_instructions(&[op, 0xEA])
            .assert_reg_eq(Register::from_usize(reg).unwrap(), 0xEA);
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
                .assert_reg_eq(Register::from_single_table(dest), 0xEA);
        }
    }
}
