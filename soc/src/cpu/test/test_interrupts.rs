/// TODO: Test the fact that the interrupt handler won't start in the middle of an instruction.
use super::*;

use crate::cpu::register::Register::*;

use crate::system::Interrupts;
use crate::timer;

#[rustfmt::skip]
const INTERRUPT_HANDLERS: [u8; 40] = [
    // 0x40. Add 3 to A.
    ADD_IMM, 3,
    RETI,
    0, 0, 0, 0, 0,
    // 0x48. Add 7 to A.
    ADD_IMM, 7,
    RETI,
    0, 0, 0, 0, 0,
    // 0x50. Add 11 to A.
    ADD_IMM, 11,
    RETI,
    0, 0, 0, 0, 0,
    // 0x58. Add 13 to A.
    ADD_IMM, 13,
    RETI,
    0, 0, 0, 0, 0,
    // 0x60. Add 17 to A.
    ADD_IMM, 17,
    RETI,
    0, 0, 0, 0, 0
];

fn interrupt_handler_result(index: i32) -> i32 {
    match index {
        0 => 3,
        1 => 7,
        2 => 11,
        3 => 13,
        4 => 17,
        _ => panic!(),
    }
}

/// Most basic interrupt test case. Enables and fires one interrupt at a time. This is to test the
/// most basic boot-strapped version of interrupt handling.
#[test]
fn test_single_interrupt() {
    #[rustfmt::skip]
    const PRELUDE: [u8; 4] = [
        // Enable interrupts.
        EI,
        // Set A to zero. Should happen before interrupt.
        LD_A_IMM, 0,
        // Useless instruction.
        LD_A_A,
    ];
    for i in 0..5 {
        let int_bit = 1 << i;
        with_dynamic_cart()
            .set_mem_range(0x40, &INTERRUPT_HANDLERS)
            .set_mem_8bit(0xFFFF, int_bit)
            .set_mem_8bit(0xFF0F, int_bit)
            .execute_instructions(&PRELUDE)
            .assert_reg_eq(A, interrupt_handler_result(i));
    }
}

/// Tests that the high 3 bits of IF return 1.
#[test]
fn test_if_register() {
    #[rustfmt::skip]
    let ops = &[
        LD_FF_A, 0x0F,
        LD_A_FF, 0x0F
    ];
    with_dynamic_cart().execute_instructions(ops).assert_reg_eq(A, 0xE0);
}

#[test]
fn test_reti_timing() {
    #[rustfmt::skip]
    const RETURN_BODY: [u8; 2] = [
        // Should never execute, as it would interrupt immediately.
        LD_A_IMM, 0xBE
    ];
    #[rustfmt::skip]
    const INTERRUPT_HANDLER: [u8; 3] = [
        // Jump to after our instruction body.
        JP, 0x01, 0xC0,
    ];
    const INITIAL_OPS: [u8; 1] = [RETI];
    with_default()
        // Set the interrupt handling routine.
        .set_mem_range(0x40, &INTERRUPT_HANDLER)
        // Set the instructions that RETI will return to.
        .set_mem_range(0xD000, &RETURN_BODY)
        // Save RETURN_BODY's address on the stack.
        .set_mem_range(0xFF80, &[0x00, 0xD0])
        .set_reg(SP, 0xFF82)
        .set_reg(A, 0xDA)
        // Enable the interrupt.
        .set_mem_8bit(0xFFFF, 1)
        .set_mem_8bit(0xFF0F, 1)
        .execute_instructions(&INITIAL_OPS)
        .assert_reg_eq(A, 0xDA);
}

#[test]
fn test_eidi_chain() {
    #[rustfmt::skip]
    const END_WITH_EI: [u8; 10] = [
        EI, DI, EI, DI, EI, DI, EI,
        LD_A_IMM, 0,
        LD_A_A
    ];
    with_dynamic_cart()
        .set_mem_range(0x40, &INTERRUPT_HANDLERS)
        .set_mem_8bit(0xFFFF, 1)
        .set_mem_8bit(0xFF0F, 1)
        .execute_instructions(&END_WITH_EI)
        .assert_reg_eq(A, interrupt_handler_result(0));
}

#[allow(dead_code)]
#[test]
fn test_ei_chain() {
    #[rustfmt::skip]
    const EI_CHAIN: [u8; 7] = [
        EI, EI, EI, EI, EI,
        LD_A_A,
        LD_A_A,
    ];
}

#[test]
fn test_halt_with_ints_enabled() {
    #[rustfmt::skip]
    let ops = [
        HALT,
        ADD_IMM, 0x01,
    ];
    // Stop right before the interrupt gets fired.
    with_default()
        .set_mem_8bit(0xFFFF, Interrupts::TIMER.bits())
        .set_reg(SP, 0xFFFF)
        .setup_timer(timer::TimerFrequency::Every16)
        .set_reg(A, 0)
        .execute_instructions_for_mcycles(&ops, 256 * (16 / 4) + 1 + 1)
        .assert_reg_eq(A, 0);
    // Then go the whole way.
    with_default()
        .set_mem_8bit(0xFFFF, Interrupts::TIMER.bits())
        .set_reg(SP, 0xFFFF)
        .setup_timer(timer::TimerFrequency::Every16)
        .set_reg(A, 0)
        .execute_instructions_for_mcycles(&ops, 256 * (16 / 4) + 1 + 2)
        .assert_reg_eq(A, 1);
}
