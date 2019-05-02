use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;
use crate::util;
use crate::util::is_bit_set;

use bitfield::*;
use io_registers::{Addresses, Register};
use mmu::{MemoryBus, MemoryMapped2};
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
    pub struct TimerControl(i32);
    no default BitRange;
    impl Debug;
    u8;
    pub into TimerFrequency, frequency, set_frequency: 1, 0;
    pub enabled, set_enabled: 2;
}

from_u8!(TimerFrequency);

#[derive(Copy, Clone, Debug)]
pub struct Timer {
    // DIV.
    div: TimerDiv,
    tac: TimerControl,
    /// 9-bit register. Exposes only 8 bits.
    tima: TimerTima,
    tma: TimerTma,
    should_interrupt: bool,
}

define_typed_register!(TimerControl, Addresses::TimerControl);
define_int_register!(TimerDiv, Addresses::TimerDiv);
define_int_register!(TimerTima, Addresses::TimerCounter);
define_int_register!(TimerTma, Addresses::TimerModulo);

impl MemoryMapped2 for Timer {
    fn default_next_state(&self, bus: &MemoryBus) -> Box<Timer> {
        let mut state = Box::new(*self);
        state.tima.set_from_bus(bus);
        state.tma.set_from_bus(bus);
        state.tac.set_from_bus(bus);
        state
    }

    fn execute_tcycle(self: Box<Self>, bus: &MemoryBus) -> (Box<Timer>, Interrupts) {
        // if bus.t_state != 1 {
        //     return (self, Interrupts::empty());
        // }
        let do_print = |header, state: &Timer| {
            trace!(
                target: "timer",
                "({}) - Counter: {}\tTIMA: {}\tFire: {}",
                header,
                *state.div % 16,
                *state.tima,
                state.should_interrupt
            );
        };
        let mut next_state = self.default_next_state(bus);
        // Allow the CPU to overwrite DIV and TIMA.
        // [HW] div = (old + 1) or bus & 0
        if bus.t_state == 1 {
            next_state.div.0 = (*self.div + 4) & 0xFFFF;
        }
        if bus.writes_to(next_state.div.address()).is_some() {
            next_state.div.0 = 0;
        }
        let old_bit = self.edge_detector_input();
        let new_bit = next_state.edge_detector_input();
        if old_bit && !new_bit {
            // Negative edge detector fired! Increase TIMA.
            // [HW] tima = (old + 1) or bus.
            next_state.tima.0 = *self.tima + 1;
        }
        // Check for TIMA overflows.
        // [HW] tima = old or bus.
        let tima = self.tima.0;
        if (tima & 0x100) != 0 {
            // TODO: Handle case where cpu writes TMA
            *next_state.tima = *self.tma;
            next_state.should_interrupt = true;
        }
        let interrupt = if self.should_interrupt {
            next_state.should_interrupt = false;
            Interrupts::TIMER
        } else {
            Interrupts::empty()
        };
        next_state.tima.set_from_bus(bus);
        next_state.tma.set_from_bus(bus);
        next_state.tac.set_from_bus(bus);
        do_print("before", self.as_ref());
        do_print("after", next_state.as_ref());
        (next_state, interrupt)
    }

    fn read_register(&self, address: io_registers::Addresses) -> Option<i32> { None }
    fn write_register(&mut self, address: io_registers::Addresses, value: i32) -> Option<()> {
        None
    }
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            div: TimerDiv(0),
            tac: TimerControl(0xFF),
            tima: TimerTima(0),
            tma: TimerTma(0),
            should_interrupt: false,
        }
    }

    /// Tries to emulate the internal behavior of the timer as much possible (mostly to accurately
    /// implement unintended consequences and glitches!).
    fn edge_detector_input(&self) -> bool {
        let tac = self.tac.0;
        let freq_0 = is_bit_set(tac, 0);
        let freq_1 = is_bit_set(tac, 1);
        let freq_1_a = if freq_0 {
            is_bit_set(*self.div, 7)
        } else {
            is_bit_set(*self.div, 5)
        };
        let freq_1_b = if freq_0 {
            is_bit_set(*self.div, 3)
        } else {
            is_bit_set(*self.div, 9)
        };
        (if freq_1 { freq_1_a } else { freq_1_b }) && self.tac.enabled()
    }
}

impl mmu::MemoryMapped for Timer {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::TimerDiv) => Some(self.div.0 >> 8),
            Some(io_registers::Addresses::TimerControl) => Some(((self.tac.0 & 0x7) | 0xF8) as i32),
            Some(io_registers::Addresses::TimerCounter) => Some(self.tima.0 & 0xFF),
            Some(io_registers::Addresses::TimerModulo) => Some(self.tma.0),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        debug_assert!(util::is_8bit(value));
        let mmu::Address(_, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::TimerDiv) => {
                //self.div.0 = 0;
                Some(())
            }
            Some(io_registers::Addresses::TimerControl) => {
                // self.tac.0 = value;
                Some(())
            }
            Some(io_registers::Addresses::TimerCounter) => {
                // If we're setting TIMA to TMA in this cycle, ignore any other request coming from
                // the CPU.
                //if !(self.should_interrupt && (self.tima & 0x100) != 0) {
                debug_assert!(!self.should_interrupt);
                //self.tima.0 = value;
                // }
                Some(())
            }
            Some(io_registers::Addresses::TimerModulo) => {
                //self.tma.0 = value;
                Some(())
            }
            _ => None,
        }
    }
}
