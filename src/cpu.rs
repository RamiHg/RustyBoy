use core::fmt;

use crate::error::Result;
use crate::io_registers;
use crate::memory::{Memory, MemoryError};
use crate::util;
use micro_code::MicroCode;

mod alu;
mod asm;
mod control_unit;
mod decoder;
mod micro_code;
mod register;

#[cfg(test)]
mod test;

pub enum SideEffect {
    Write { raw_address: i32, value: i32 },
}

/// The output of a micro code execution.
/// Legacy. Should probably be removed eventually.
pub struct Output {
    pub side_effect: Option<SideEffect>,
    pub is_done: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecodeMode {
    Fetch,
    Decode,
    Execute,
}

impl Default for DecodeMode {
    fn default() -> Self { DecodeMode::Fetch }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct State {
    t_state: TState,
    decode_mode: DecodeMode,
    address_latch: i32,
    data_latch: i32,
    read_latch: bool,
    write_latch: bool,

    enable_interrupts: bool,
    disable_interrupts: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TState(i32);

impl TState {
    pub fn get(&self) -> i32 { self.0 + 1 }

    pub fn inc(&mut self) { self.0 = (self.0 + 1) % 4; }
}

// This needs to get heavily refactored, with the control unit
// code being migrated here, and state made private.
pub struct Cpu {
    pub state: State,
    pub registers: register::File,
    pub decoder: decoder::Decoder,
    pub micro_code_stack: Vec<MicroCode>,

    interrupts_enabled: bool,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            state: State::default(),
            registers: register::File::new([0; register::Register::NumRegisters as usize]),
            decoder: decoder::Decoder::new(),
            micro_code_stack: Vec::new(),
            interrupts_enabled: false,
        }
    }

    fn microcode_prelude(&mut self, memory: &Memory) -> Option<SideEffect> {
        assert!(!(self.state.read_latch && self.state.write_latch));
        // Service read requests at T=3's rising edge.
        if self.state.read_latch {
            if self.state.t_state.get() == 3 {
                self.state.data_latch = memory.read(self.state.address_latch);
            } else {
                // Write garbage in data latch to catch bad reads.
                self.state.data_latch = -1;
            }
        }
        // Service write requests at T=4's rising edge.
        if self.state.write_latch {
            assert!(util::is_16bit(self.state.address_latch));
            assert!(util::is_8bit(self.state.data_latch));
            if self.state.t_state.get() == 4 {
                return Some(SideEffect::Write {
                    raw_address: self.state.address_latch,
                    value: self.state.data_latch,
                });
            }
        }

        None
    }

    pub fn handle_interrupts(&mut self, memory: &mut Memory) -> Result<()> {
        use io_registers::Register;
        // Completely ignore interrupts if they're not enabled.
        if !self.interrupts_enabled {
            return Ok(());
        }
        // Otherwise, carry on.
        let interrupt_fired_flag = memory.read(io_registers::InterruptFlag::ADDRESS as i32) & 0x1F;
        assert_eq!(interrupt_fired_flag & !0x1F, 0);
        // Re-use io_registers::InterruptFlag to get the IE register.
        let interrupt_enabled_flag = memory.read(0xFFFF) & 0x1F;
        let fired_interrupts = interrupt_fired_flag & interrupt_enabled_flag;
        let interrupt_index = fired_interrupts.trailing_zeros();
    }

    pub fn execute_t_cycle(&mut self, memory: &Memory) -> Result<Output> {
        let side_effect = self.microcode_prelude(memory);
        control_unit::cycle(self, memory);
        self.state.t_state.inc();
        let is_done = self.state.t_state.get() == 1 && self.state.decode_mode == DecodeMode::Fetch;
        Ok(Output {
            side_effect,
            is_done,
        })
    }
}
