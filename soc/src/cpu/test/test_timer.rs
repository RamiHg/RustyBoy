use num_traits::FromPrimitive;

use super::*;
use crate::cpu::register::Register;
use Register::*;

use crate::timer::TimerFrequency;

#[rustfmt::skip]
const INTERRUPT_HANDLER: [u8; 3] = [
    // 0x50. Add 1 to A.
    ADD_IMM, 1,
    RETI,
];

#[test]
fn test_simple() {
    #[rustfmt::skip]
    const PRELUDE: [u8; 6] = [
        // Enable interrupts.
        EI,
        // Set A to 0. Should happen before any interrupts.
        LD_A_IMM, 0,
        //LD_FF_A, 0x04,
        // Useless instruction.
        LD_A_A,
        // C004
        JR, 0xFD,
    ];

    let increments_for_three_interrupts = (256 + 1) * 4 * 3;

    let freq_to_cycles = |freq| match freq {
        0 => 1024,
        1 => 16,
        2 => 64,
        3 | _ => 256,
    } / 4;

    for freq_index in 1..4 {
        let freq = TimerFrequency::from_i32(freq_index).unwrap();
        let expected_cycles = (freq_to_cycles(freq_index) * 256) * 3 + 2 + 5 + 2;
        with_default()
            .set_mem_range(0x50, &INTERRUPT_HANDLER)
            .setup_timer(freq)
            .set_mem_8bit(io_registers::Addresses::InterruptEnable as i32, 0xFF)
            .set_reg(SP, 0xFFFF)
            .execute_instructions_for_mcycles(&PRELUDE, expected_cycles)
            .assert_reg_eq(A, 3);
    }
}
