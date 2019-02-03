use super::register::Register;
use super::*;

use Register::*;

#[test]
fn test_indirect_load() {
    for &(op, reg) in &[(0x73, HL), (0x0A, BC), (0x1A, DE)] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg_pair(reg, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0xEA);
    }
}
