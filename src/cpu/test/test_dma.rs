use super::*;
use crate::io_registers::Addresses;

use crate::cpu::register::Register::*;

#[test]
fn test_simple() {
    let nums: Vec<u8> = (0u8..161u8).collect();
    let mut test_setup = with_default()
        .set_mem_range(0xDA00, &nums)
        .set_mem_8bit(Addresses::Dma as i32, 0xDA)
        .execute_instructions_for_mcycles(&INF_LOOP, 161);
    for i in 0..160 {
        test_setup = test_setup.assert_mem_8bit_eq(0xFE00 + i, i);
    }
    assert_eq!(test_setup.system.memory().raw_read(0xFEA0), 0);
}

// #[test]
// fn test_setup_delay() {
//     #[rustfmt::skip]
//     let ops = &[
//         LD_A_IMM, 0xDA,
//         LD_HL_IMM, 0x01, 0xFE,
//         LD_FF_A, 0x46,
//         // One cycle delay, then the DMA starts.
//         LD_A_A,
//         LD_A_A,
//     ];
//     with_default()
//         .set_mem_range(0xDA00, &[0xBE, 0xEF])
//         .execute_instructions(ops)
//         .assert_mem_8bit_eq(0xFE00, 0xBE)
//         .assert_mem_8bit_eq(0xFE01, 0x00);
// }

#[test]
fn test_restart_dma() {
    let nums: Vec<u8> = (0u8..160u8).collect();
    let more_nums: Vec<u8> = (160_u8..255u8).collect();
    let mut test_setup = with_default()
        .set_mem_range(0xDA00, &nums)
        .set_mem_range(0xDB00, &more_nums)
        .set_mem_8bit(Addresses::Dma as i32, 0xDA)
        .execute_instructions_for_mcycles(&INF_LOOP, 161)
        .set_mem_8bit(Addresses::Dma as i32, 0xDB)
        .execute_instructions_for_mcycles(&INF_LOOP, 96);
    for i in 0..95 {
        test_setup = test_setup.assert_mem_8bit_eq(0xFE00 + i, 160 + i);
    }
    for i in 96..120 {
        test_setup = test_setup.assert_mem_8bit_eq(0xFE00 + i, i);
    }

    assert_eq!(test_setup.system.memory().raw_read(0xFEA0), 0);
}
