use crate::io_registers;
use crate::mmu;
use crate::system::FireInterrupt;
use crate::util;
use crate::util::is_bit_set;

use num_traits::FromPrimitive;

#[derive(Copy, Clone, Debug)]
pub struct Timer {
    // DIV.
    counter: i32,
    tac: io_registers::TimerControl,
    /// 9-bit register. Exposes only 8 bits.
    tima: i32,
    tma: i32,
    should_interrupt: bool,
    rly: bool,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            counter: 0,
            tac: io_registers::TimerControl(0xFF),
            tima: 0,
            tma: 0,
            should_interrupt: false,
            rly: false,
        }
    }

    #[allow(warnings)]
    pub fn execute_mcycle(&self) -> (Timer, Option<FireInterrupt>) {
        // If we're not enabled, don't do anything.
        let mut new_state = *self;
        let mut fire_interrupt = None;
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
            //fire_interrupt.0 = true;
            new_state.tima = new_state.tma;
            new_state.should_interrupt = true;
        }
        if self.should_interrupt {
            assert!(self.tac.enabled());
            new_state.rly = true;
            //fire_interrupt = Some(FireInterrupt::timer());
            new_state.should_interrupt = false;
        }
        if self.rly {
            fire_interrupt = Some(FireInterrupt::timer());
            new_state.rly = false;
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
        if freq_1 {
            freq_1_a
        } else {
            freq_1_b
        }
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
        assert!(util::is_8bit(value));
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
                assert!(!self.should_interrupt);
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
