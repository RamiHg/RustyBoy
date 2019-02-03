use super::register::Register;
use super::*;

use Register::*;

#[test]
fn test_ld_a_bc_de_hli_hld() {
    for &(op, reg) in &[(0x0A, BC), (0x1A, DE)] {
        with_default()
            .set_mem_8bit(0xD000, 0xEA)
            .set_reg_pair(reg, 0xD000)
            .execute_instructions(&[op])
            .assert_reg_eq(A, 0xEA);
    }
}

// #[test]
// fn test_ld_reg_hl() {
//     for &(op, reg) in &[
//         (0x46, B),
//         (0x4e, C),
//         (0x56, D),
//         (0x5e, E),
//         (0x66, H),
//         (0x6e, L),
//         (0x7e, A),
//     ] {
//         with_default()
//             .set_mem_8bit(0xD000, 0xEA)
//             .set_reg_pair(HL, 0xD000)
//             .execute_instructions(&[op])
//             .assert_reg_eq(reg, 0xEA);
//     }
// }
