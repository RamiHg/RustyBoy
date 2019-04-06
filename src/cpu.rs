use core::fmt;

use crate::error::{self, Result};
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

#[derive(Debug)]
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
    decode_mode: DecodeMode,
    address_latch: i32,
    data_latch: i32,
    read_latch: bool,
    write_latch: bool,

    interrupt_enable_counter: i32,
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

    pub t_state: TState,

    interrupts_enabled: bool,
    is_handling_interrupt: bool,
    interrupt_handle_mcycle: i32,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            state: State::default(),
            registers: register::File::new([0; register::Register::NumRegisters as usize]),
            decoder: decoder::Decoder::new(),
            micro_code_stack: Vec::new(),
            t_state: TState::default(),
            interrupts_enabled: false,
            is_handling_interrupt: false,
            interrupt_handle_mcycle: 0,
        }
    }

    fn microcode_prelude(&mut self, memory: &Memory) -> Option<SideEffect> {
        assert!(!(self.state.read_latch && self.state.write_latch));
        // Service read requests at T=3's rising edge.
        if self.state.read_latch {
            if self.t_state.get() == 3 {
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
            if self.t_state.get() == 4 {
                return Some(SideEffect::Write {
                    raw_address: self.state.address_latch,
                    value: self.state.data_latch,
                });
            }
        }

        None
    }

    pub fn execute_t_cycle(&mut self, memory: &mut Memory) -> Result<Output> {
        // First step is to handle interrupts.
        self.handle_interrupts(memory)?;
        // Then, run through the micro-code prelude.
        let side_effect = self.microcode_prelude(memory);
        // Finally, execute the micro-code.
        let next_state = control_unit::cycle(self, memory);
        let is_done = self.t_state.get() == 4 && next_state.decode_mode == DecodeMode::Fetch;
        self.state = next_state;
        // This will be tricky to translate to hardware.
        if is_done && self.state.interrupt_enable_counter > 0 {
            if self.state.interrupt_enable_counter == 1 {
                self.interrupts_enabled = true;
            }
            self.state.interrupt_enable_counter -= 1;
        }
        self.t_state.inc();
        Ok(Output {
            side_effect,
            is_done,
        })
    }

    pub fn handle_interrupts(&mut self, memory: &mut Memory) -> Result<()> {
        // If interrupts are enabled, check for any fired interrupts. Otherwise, check if we are
        // currently handling an interrupt. If none of that, proceed as usual.
        if self.interrupts_enabled {
            assert!(!self.is_handling_interrupt);
            self.check_for_interrupts(memory)?;
        } else if self.is_handling_interrupt {
            // At the 3rd TCycle of the 4th MCycle (or here, the beginning of the 4th TCycle), read
            // the fired interrupt flag AGAIN, and then decide on which interrupt to handle.
            if self.t_state.get() == 4 {
                if self.interrupt_handle_mcycle == 3 {
                    self.select_fired_interrupt(memory)?;
                }
                self.interrupt_handle_mcycle += 1;
            }
            if self.interrupt_handle_mcycle == 5 {
                // Done with handling the interrupt!
                self.is_handling_interrupt = false;
            }
        }
        Ok(())
    }

    fn check_for_interrupts(&mut self, memory: &Memory) -> Result<()> {
        assert!(self.interrupts_enabled);
        // Only look at interrupts in the beginning of T3, right before PC is incremented.
        if self.state.decode_mode != DecodeMode::Fetch && self.t_state.get() != 3 {
            return Ok(());
        }
        // In this stage, we only check IF there is an interrupt, not WHICH interrupt to fire.
        let interrupt_fired_flag = self.interrupt_fired_flag(memory)?;
        let ie_flag = memory.read(0xFFFF) & 0x1F;
        if (interrupt_fired_flag & ie_flag) != 0 {
            dbg!(interrupt_fired_flag & ie_flag);
            // Go into interrupt handling mode! Pop all in-flight micro-codes, and push the
            // interrupt handling routine micro-codes.
            self.micro_code_stack = self.decoder.interrupt_handler();
            self.interrupts_enabled = false;
            self.is_handling_interrupt = true;
            self.interrupt_handle_mcycle = 0;
        }
        // Otherwise, do nothing.
        Ok(())
    }

    fn select_fired_interrupt(&mut self, memory: &mut Memory) -> Result<()> {
        debug_assert_eq!(self.interrupt_handle_mcycle, 3);
        debug_assert_eq!(self.t_state.get(), 4);
        let interrupt_fired_flag = self.interrupt_fired_flag(memory)?;
        let ie_flag = memory.read(io_registers::Addresses::InterruptEnable as i32) & 0x1F;
        let fired_interrupts = interrupt_fired_flag & ie_flag;
        if fired_interrupts == 0 {
            return Err(error::Type::InvalidOperation(
                "In interrupt handling routine, but found no fired interrupts!".into(),
            ));
        }
        // In hardware this would be a case statement, but let's be clean here.
        let interrupt_index = fired_interrupts.trailing_zeros() as i32;
        dbg!(interrupt_index);
        assert!(interrupt_index <= 4);
        self.registers
            .set(register::Register::TEMP_LOW, interrupt_index * 8);
        // Finally, issue a write to clear the fired bit.
        let new_fired_interrupts = interrupt_fired_flag & !(1 << interrupt_index);
        memory.store(
            io_registers::Addresses::InterruptFired as i32,
            new_fired_interrupts,
        );
        Ok(())
    }

    fn interrupt_fired_flag(&mut self, memory: &Memory) -> Result<i32> {
        let interrupt_fired_flag =
            memory.read(io_registers::Addresses::InterruptFired as i32) & 0x1F;
        if (interrupt_fired_flag & !0x1F) != 0 {
            Err(error::Type::InvalidOperation(format!(
                "Interrupt flag register is corrupt: {:X?}",
                interrupt_fired_flag
            )))
        } else {
            Ok(interrupt_fired_flag)
        }
    }
}
