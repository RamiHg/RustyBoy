use crate::io_registers;
use crate::mmu;
use crate::system;

use num_traits::FromPrimitive;
use system::Interrupts;

#[derive(Clone)]
pub struct Controller {
    control: io_registers::SerialControl,
    data: i32,
    // Controls the shift in/out.
    counter: i32,
    // Buffers the data in.
    buffer: i32,
    // Saves bytes as they get buffered in.
    bytes: Vec<char>,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            control: io_registers::SerialControl(0),
            data: 0,
            counter: 0,
            buffer: 0,
            bytes: Vec::new(),
        }
    }

    pub fn execute_tcycle(&self) -> (Controller, Interrupts) {
        let mut next_state = self.clone();
        let mut fire_interrupt = Interrupts::empty();
        if self.control.is_transferring() {
            next_state.counter = self.counter + 1;

            if self.counter != 123123 && (self.counter % 512) == 0 {
                //println!("dat: {:#010b}", self.data);
                //println!("buf: {:#010b}", self.buffer);
                // Clock in/out one bit.
                let bit_index = self.counter / 512;
                if bit_index < 8 {
                    if bit_index == 0 {
                        print!("{}", self.data as u8 as char);
                    }
                    next_state.buffer = (next_state.buffer << 1) | ((self.data & 0x80) >> 7);
                    next_state.data = ((self.data << 1) | 1) & 0xFF;
                } else {
                    assert_eq!(bit_index, 8);
                    // End the transfer!
                    next_state.bytes.push(self.buffer as u8 as char);
                    //print!("{}", self.buffer as u8 as char);
                    next_state.control.set_transferring(false);
                    fire_interrupt = Interrupts::SERIAL;
                    next_state.counter = 0;
                    next_state.buffer = 0;
                }
            }
        } else {
            debug_assert_eq!(self.counter, 0);
        }
        (next_state, fire_interrupt)
    }
}

impl mmu::MemoryMapped for Controller {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(location, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::SerialControl) => Some(self.control.0 as i32),
            Some(io_registers::Addresses::SerialData) => Some(self.data),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(location, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::SerialControl) => {
                self.control.0 = value;
                Some(())
            }
            Some(io_registers::Addresses::SerialData) => {
                self.data = value;
                Some(())
            }
            _ => None,
        }
    }
}
