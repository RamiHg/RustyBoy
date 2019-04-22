use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;
use crate::util;
use crate::util::is_bit_set;

use bitfield::bitfield;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Clone, Copy, Debug, FromPrimitive)]
pub enum TimerFrequency {
    Every1024 = 0, // 4kHz
    Every16 = 1,   // ~262kHz
    Every64 = 2,   // 64kHz
    Every256 = 3,  // 16kHz
}

/// Timer Control register (TAC). 0xFF07
bitfield! {
    pub struct TimerControl(u8);
    impl Debug;
    u8;
    pub into TimerFrequency, frequency, set_frequency: 1, 0;
    pub enabled, set_enabled: 2;
}

declare_register_u8!(TimerControl, io_registers::Addresses::TimerControl);
from_u8!(TimerFrequency);

#[derive(Copy, Clone, Debug)]
pub struct Timer {
    // DIV.
    counter: i32,
    tac: TimerControl,
    /// 9-bit register. Exposes only 8 bits.
    tima: i32,
    tma: i32,
    should_interrupt: bool,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            counter: 0,
            tac: TimerControl(0xFF),
            tima: 0,
            tma: 0,
            should_interrupt: false,
        }
    }

    #[allow(warnings)]
    pub fn execute_mcycle(&self) -> (Timer, Interrupts) {
        if self.tac.enabled() {
            trace!(
                target: "timer",
                "Counter: {}\tTIMA: {}\tFire: {}",
                self.counter % 16,
                self.tima,
                self.should_interrupt
            );
        }
        // If we're not enabled, don't do anything.
        let mut new_state = *self;
        let mut fire_interrupt = Interrupts::empty();
        new_state.counter = (self.counter + 1) & 0xFFFF;
        let old_bit = self.edge_detector_input();
        let new_bit = new_state.edge_detector_input();
        if old_bit && !new_bit && self.tac.enabled() {
            // Negative edge detector fired! Increase TIMA.
            new_state.tima += 1;
        }
        // Check for TIMA overflow.
        let tima_overflows = (self.tima & 0x100) != 0;
        if tima_overflows {
            new_state.tima = new_state.tma;
            new_state.should_interrupt = true;
        }
        if self.should_interrupt {
            debug_assert!(self.tac.enabled());
            fire_interrupt = Interrupts::TIMER;
            new_state.should_interrupt = false;
        }
        (new_state, fire_interrupt)
    }

    /// Tries to emulate the internal behavior of the timer as much possible (mostly to accurately
    /// implement unintended consequences and glitches!).
    fn edge_detector_input(&self) -> bool {
        let tac = self.tac.0 as i32;
        let freq_0 = is_bit_set(tac, 0);
        let freq_1 = is_bit_set(tac, 1);
        let freq_1_a = if freq_0 {
            is_bit_set(self.counter, 7)
        } else {
            is_bit_set(self.counter, 5)
        };
        let freq_1_b = if freq_0 {
            is_bit_set(self.counter, 3)
        } else {
            is_bit_set(self.counter, 9)
        };
        if freq_1 { freq_1_a } else { freq_1_b }
    }
}

impl mmu::MemoryMapped for Timer {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::TimerDiv) => Some(self.counter >> 8),
            Some(io_registers::Addresses::TimerControl) => Some(((self.tac.0 & 0x7) | 0xF8) as i32),
            Some(io_registers::Addresses::TimerCounter) => Some(self.tima & 0xFF),
            Some(io_registers::Addresses::TimerModulo) => Some(self.tma),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        debug_assert!(util::is_8bit(value));
        let mmu::Address(_, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::TimerDiv) => {
                self.counter = 0;
                Some(())
            }
            Some(io_registers::Addresses::TimerControl) => {
                self.tac.0 = value as u8;
                Some(())
            }
            Some(io_registers::Addresses::TimerCounter) => {
                // If we're setting TIMA to TMA in this cycle, ignore any other request coming from
                // the CPU.
                //if !(self.should_interrupt && (self.tima & 0x100) != 0) {
                debug_assert!(!self.should_interrupt);
                self.tima = value;
                // }
                Some(())
            }
            Some(io_registers::Addresses::TimerModulo) => {
                self.tma = value;
                Some(())
            }
            _ => None,
        }
    }
}
