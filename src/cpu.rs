use crate::error::{self, Result};
use crate::io_registers;
use crate::mmu::Memory;
use micro_code::MicroCode;

mod alu;
mod asm;
mod control_unit;
mod decoder;
mod micro_code;
pub mod register;

#[cfg(test)]
mod test;

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
    pub decode_mode: DecodeMode,
    in_cb_mode: bool,
    pub address_latch: i32,
    pub data_latch: i32,
    pub read_latch: bool,
    pub write_latch: bool,

    interrupt_enable_counter: i32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TState(i32);

impl TState {
    pub fn get(&self) -> i32 { self.0 + 1 }

    pub fn inc(&mut self) { self.0 = (self.0 + 1) & 0x3; }
}

// This needs to get heavily refactored, with the control unit
// code being migrated here, and state made private.
pub struct Cpu {
    pub state: State,
    pub registers: register::File,
    pub decoder: decoder::Decoder,
    pub micro_code_stack: Vec<MicroCode>,

    pub t_state: TState,

    pub is_handling_interrupt: bool,
    interrupts_enabled: bool,
    interrupt_handle_mcycle: i32,

    is_halted: bool,
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut cpu = Cpu {
            state: State::default(),
            registers: register::File::new([0; register::Register::NumRegisters as usize]),
            decoder: decoder::Decoder::new(),
            micro_code_stack: Vec::new(),
            t_state: TState::default(),
            interrupts_enabled: false,
            is_handling_interrupt: false,
            interrupt_handle_mcycle: 0,
            is_halted: false,
        };
        cpu.registers.set(register::Register::PC, 0x100);
        cpu
    }

    pub fn execute_t_cycle(&mut self, memory: &mut Memory) -> Result<()> {
        // First step is to handle interrupts. Save off the is_halted flag since the next function
        // can unset it.
        self.handle_interrupts_or_unhalt(memory)?;
        if true || !self.is_halted {
            let (next_state, is_done) = control_unit::cycle(self);
            self.state = next_state;
            // This will be tricky to translate to hardware.
            if is_done && self.state.interrupt_enable_counter > 0 {
                if self.state.interrupt_enable_counter == 1 {
                    self.interrupts_enabled = true;
                }
                self.state.interrupt_enable_counter -= 1;
            }

        }
        self.t_state.inc();
        Ok(())
    }

    // TODO: Clean up the logic and make this function stateless.. It's nasty.
    pub fn handle_interrupts_or_unhalt(&mut self, memory: &mut Memory) -> Result<()> {
        // If interrupts are enabled, check for any fired interrupts. Otherwise, check if we are
        // currently handling an interrupt. If none of that, proceed as usual.
        if self.interrupts_enabled || self.is_halted {
            debug_assert!(!self.is_handling_interrupt);
            if self.has_pending_interrupts(memory)? {
                if self.is_halted {
                    // If we're halted, get out of it, whether interrupts are enabled or not.
                    if self.t_state.get() == 4 {
                        self.is_halted = false;
                    }
                } else if self.interrupts_enabled {
                    // If interrupts are enabled, handle the interrupt!
                    self.enter_interrupt_handler();
                }
            }
        } else if self.is_handling_interrupt {
            // At the 3rd TCycle of the 4th MCycle (or here, the beginning of the 4th TCycle), read
            // the fired interrupt flag AGAIN, and then decide on which interrupt to handle.
            if self.t_state.get() == 3 {
                if self.interrupt_handle_mcycle == 3 {
                    self.select_fired_interrupt(memory)?;
                }
            } else if self.t_state.get() == 4 {
                self.interrupt_handle_mcycle += 1;
            }
            if self.interrupt_handle_mcycle == 5 {
                // Done with handling the interrupt!
                self.is_handling_interrupt = false;
            }
        }
        Ok(())
    }

    fn has_pending_interrupts(&mut self, memory: &Memory) -> Result<bool> {
        debug_assert!(self.interrupts_enabled || self.is_halted);
        // Only look at interrupts in the beginning of T3, right before PC is incremented.
        if !self.is_halted
            && (self.state.decode_mode != DecodeMode::Decode || self.t_state.get() != 3)
        {
            return Ok(false);
        }
        // TODO: All interrupts can be disabled if CPU writes to IF in the same cycle. Somehow
        // support this.
        // In this stage, we only check IF there is an interrupt, not WHICH interrupt to fire.
        let mut interrupt_fired_flag = self.interrupt_fired_flag(memory)?;
        if self.state.write_latch && self.state.address_latch == 0xFF0F {
            warn!("It aint good");
            interrupt_fired_flag = self.state.data_latch;
        }
        let ie_flag = memory.read(0xFFFF) & 0x1F;
        Ok((interrupt_fired_flag & ie_flag) != 0)
    }

    fn enter_interrupt_handler(&mut self) {
        debug_assert!(self.interrupts_enabled);
        // Pop all in-flight micro-codes, and push the interrupt handing routine micro-codes.
        self.micro_code_stack = self.decoder.interrupt_handler();
        self.interrupts_enabled = false;
        self.is_handling_interrupt = true;
        self.interrupt_handle_mcycle = 0;
    }

    fn select_fired_interrupt(&mut self, memory: &mut Memory) -> Result<()> {
        debug_assert_eq!(self.interrupt_handle_mcycle, 3);
        debug_assert_eq!(self.t_state.get(), 3);
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
        trace!(target: "int", "Firing int {}", interrupt_index);
        debug_assert!(interrupt_index <= 4);
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
