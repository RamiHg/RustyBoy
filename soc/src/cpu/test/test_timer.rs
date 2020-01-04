use num_traits::FromPrimitive;

use super::*;
use crate::cpu::register::Register;
use Register::*;

use crate::timer::TimerFrequency;
// TODO: Set default to do nothing but RETI.
#[rustfmt::skip]
const INTERRUPT_HANDLERS: [u8; 40] = [
    // 0x40. Add 0 to A.
    ADD_IMM, 0,
    RETI,
    0, 0, 0, 0, 0,
    // 0x48. Add 0 to A.
    ADD_IMM, 0,
    RETI,
    0, 0, 0, 0, 0,
    // 0x50. Add 0 to A.
    ADD_IMM, 1,
    RETI,
    0, 0, 0, 0, 0,
    // 0x58. Add 13 to A.
    ADD_IMM, 0,
    RETI,
    0, 0, 0, 0, 0,
    // 0x60. Add 0 to A.
    ADD_IMM, 0,
    RETI,
    0, 0, 0, 0, 0
];

#[test]
fn test_simple() {
    #[rustfmt::skip]
    const PRELUDE: [u8; 6] = [
        // Enable interrupts.
        EI,
        // Set A to 0. Should happen before any interrupts.
        LD_A_IMM, 0,
        // Useless instruction.
        LD_A_A,
        // C004
        JR, 0xFD,
    ];

    // The timer runs in Mcycles.
    let freq_to_cycles = |freq| {
        1048576
            / match freq {
                0 => 4096,
                1 => 262144,
                2 => 65536,
                3 | _ => 16384,
            }
    };

    for freq_index in 0..4 {
        let freq = TimerFrequency::from_i32(freq_index).unwrap();
        let expected_cycles = (freq_to_cycles(freq_index) * 256) * 3 + 2 + 5 + 2;
        with_default()
            .set_mem_range(0x40, &INTERRUPT_HANDLERS)
            .setup_timer(freq)
            .set_mem_8bit(io_registers::Addresses::InterruptEnable as i32, 0xFF)
            .set_reg(SP, 0xFFFF)
            .execute_instructions_for_mcycles(&PRELUDE, expected_cycles)
            .assert_reg_eq(A, 3);
    }
}
