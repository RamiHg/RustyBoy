mod alu;
mod autodecoder;
mod decoder;
mod micro_code;
mod register;

#[cfg(test)]
mod test;

// TODO: Place this in a better place..
pub use self::micro_code::{Output, SideEffect};

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Error as fmt::Display>::fmt(self, f)
    }
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

#[derive(Debug)]
pub enum DecodeMode {
    Fetch,
    Decode,
    Execute,
}

#[derive(Debug)]
pub struct State {
    decode_mode: DecodeMode,
    address_latch: i32,
    data_latch: i32,
    read_latch: bool,
    write_latch: bool,
}

impl Default for State {
    fn default() -> State {
        State {
            decode_mode: DecodeMode::Fetch,
            address_latch: 0,
            data_latch: 0,
            read_latch: true,
            write_latch: false,
        }
    }
}

pub struct TState(i32);

impl TState {
    pub fn get(&self) -> i32 {
        self.0 + 1
    }

    pub fn inc(&mut self) {
        self.0 = (self.0 + 1) % 4;
    }
}

// This needs to get heavily refactored, with the control unit
// code being migrated here, and state made private.
pub struct Cpu {
    pub state: State,
    pub registers: register::File,
    t_state: TState,
    micro_code_stack: Vec<MicroCode>,
    pub micro_code_v2_stack: Vec<autodecoder::MicroCode>,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            state: State::default(),
            registers: register::File::new([0; register::Register::NumRegisters as usize]),
            t_state: TState(0),
            micro_code_stack: Vec::new(),
            micro_code_v2_stack: Vec::new(),
        }
    }

    fn microcode_prelude(&mut self, memory: &Memory) -> Option<SideEffect> {
        assert!(!(self.state.read_latch && self.state.write_latch));
        // Service read requests at T=3's rising edge.
        println!("Address: {:X?}", self.state.address_latch);

        if self.state.read_latch {
            if self.t_state.get() == 3 {
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
            if self.t_state.get() == 4 {
                return Some(SideEffect::Write {
                    raw_address: self.state.address_latch,
                    value: self.state.data_latch,
                });
            } else {
                panic!(
                    "Writing memory should only be asserted such that it is sampled at the rising \
                     edge of T3."
                );
            }
        }

        None
    }

    pub fn execute_machine_cycle_v2(&mut self, memory: &Memory) -> Result<Output> {
        /*
        let mut last_output = Output {
            side_effect: None,
            is_done: false,
        };
        for i in 0..=3 {
            // Sanity check
            assert_eq!(self.t_state.get(), i + 1);
            // Run the prelude.
            let side_effect = self.microcode_prelude(memory);
            last_output = autodecoder::control_unit::cycle(self, memory);
            last_output.side_effect = side_effect; // hack.
            self.t_state.inc();
        }

        Ok(last_output)
        */
        Err(Error::InvalidOpcode(1337))
    }

    /// Runs a machine cycle.
    ///
    /// Optionally returns a data bus write request.
    pub fn execute_machine_cycle(&mut self, memory: &Memory) -> Result<Output> {
        if self.micro_code_stack.is_empty() {
            // Run the decoder to get a bunch of microcodes.
            self.micro_code_stack = decoder::build_decode();
        }
        let top = self.micro_code_stack.remove(0);
        let micro_code_output = top.execute(self, memory)?;

        // If this is a decode micro-code, push the codes on the stack and return.
        if let Some(SideEffect::Decode(instructions)) = micro_code_output.side_effect {
            assert!(!micro_code_output.is_done);
            self.micro_code_stack = instructions;
            return Ok(Output {
                side_effect: None,
                ..micro_code_output
            });
        }
        // Otherise, return as normal.
        Ok(micro_code_output)
    }
}
