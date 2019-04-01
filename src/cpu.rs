mod alu;
mod autodecoder;
mod micro_code;
mod register;

#[cfg(test)]
mod test;

// TODO: Place this in a better place..
pub use self::micro_code::{Output, SideEffect};

use autodecoder::decoder::Decoder;

use core::fmt;

use crate::{
    cpu::micro_code::MicroCode,
    memory::{Memory, MemoryError},
    util,
};

// #[derive(Debug)]
pub enum Error {
    InvalidOperation(String),
    InvalidOpcode(i32),
    Memory(MemoryError),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { <Error as fmt::Display>::fmt(self, f) }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::InvalidOpcode(op) => write!(f, "Invalid opcode: 0x{:X?}.", op),
            _ => write!(f, "Buzz off"),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

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
    pub decoder: Decoder,
    micro_code_stack: Vec<MicroCode>,
    pub micro_code_v2_stack: Vec<autodecoder::MicroCode>,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            state: State::default(),
            registers: register::File::new([0; register::Register::NumRegisters as usize]),
            decoder: Decoder::new(),
            micro_code_stack: Vec::new(),
            micro_code_v2_stack: Vec::new(),
        }
    }

    fn microcode_prelude(&mut self, memory: &Memory) -> Option<SideEffect> {
        assert!(!(self.state.read_latch && self.state.write_latch));
        println!("Address: {:X}", self.state.address_latch);

        dbg!(self.state);
        // Service read requests at T=3's rising edge.
        if self.state.read_latch {
            if self.state.t_state.get() == 3 {
                self.state.data_latch = memory.read(self.state.address_latch);
                println!("Setting Data Latch: {:X}.", self.state.data_latch);
            } else {
                // Write garbage in data latch to catch reads.
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

    pub fn execute_machine_cycle_v2(&mut self, memory: &Memory) -> Result<Output> {
        let mut last_output = Output {
            side_effect: None,
            is_done: false,
        };
        for i in 0..=3 {
            // Sanity check=
            // Run the prelude.
            let side_effect = self.microcode_prelude(memory);
            autodecoder::control_unit::cycle(self, memory);
            last_output.side_effect = last_output.side_effect.or(side_effect);
            self.state.t_state.inc();
        }

        if self.state.t_state.get() == 1 && self.state.decode_mode == DecodeMode::Fetch {
            println!("Im done!");
            last_output.is_done = true;
        }
        Ok(last_output)
    }
}
