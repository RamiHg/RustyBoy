use super::*;
use crate::cpu::register::Register;

use Register::*;

const SINGLES: [Register; 8] = [B, C, D, E, H, L, HL, A];
// Helper constants so that assert_flags isn't a list of bools. (TODO: Make struct instead?)
const ZERO: bool = true;
const NZERO: bool = false;
const NSUB: bool = false;
const HCY: bool = true;
const NHCY: bool = false;
const CY: bool = true;
const NCY: bool = false;

fn setup_source(src: Register, value: i32) -> TestContext {
    if let HL = src {
        with_default()
            .set_mem_8bit(0xD000, value)
            .set_reg(HL, 0xD000)
    } else {
        with_default().set_reg(src, value)
    }
}

fn cycles_for_source(src: Register) -> i32 {
    if let HL = src {
        2
    } else {
        1
    }
}

#[test]
fn test_add_a_r() {
    for (&op, &src) in [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86]
        .iter()
        .zip(SINGLES.iter())
    {
        setup_source(src, 1)
            .set_reg(A, 0x7F)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0x80)
            .assert_flags(NZERO, NSUB, HCY, NCY)
            .assert_mcycles(cycles_for_source(src));
        setup_source(src, 0x80)
            .set_reg(A, 0x80)
            .execute_instructions(&[op])
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
