use num_traits::FromPrimitive;

use super::*;
use crate::cpu::register::Register;
use Register::*;

use crate::io_registers::Register as IORegister;
use crate::io_registers::{TimerControl, TimerFrequency};

fn make_timer_control(freq: TimerFrequency) -> TimerControl {
    let mut ctrl = TimerControl(0);
    ctrl.set_enabled(true);
    ctrl.set_frequency(freq as u8);
    ctrl
}

#[rustfmt::skip]
const INTERRUPT_HANDLER: [u8; 3] = [
    // 0x40. Add 1 to A.
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
        // Useless instruction.
        LD_A_A,
        JR, 0xFD,
    ];

    let for_cycles = |freq| match freq {
        0 => 1024 * 256 * 3 / 4,
        1 => 16 * 256 * 3 / 4,
        2 => 64 * 256 * 3 / 4,
        3 => 256 * 256 * 3 / 4,
        _ => panic!(),
    };

    for freq_index in 0..4 {
        let freq = TimerFrequency::from_i32(freq_index).unwrap();
        with_default()
            .set_mem_range(0x50, &INTERRUPT_HANDLER)
            .set_mem_8bit(TimerControl::ADDRESS, make_timer_control(freq).0 as i32)
            .set_mem_8bit(io_registers::Addresses::InterruptEnable as i32, 0xFF)
            .set_reg(SP, 0xFFFF)
            .execute_instructions_for_mcycles(&PRELUDE, for_cycles(freq as i32))
            .assert_reg_eq(A, 1);
    }
}
