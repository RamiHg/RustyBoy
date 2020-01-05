use crate::io_registers;
use crate::mmu;
use crate::system;

use num_traits::FromPrimitive;
use system::Interrupts;

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[allow(unused)]
pub struct Controller {
    control: io_registers::SerialControl,
    data: i32,
    // Controls the shift in/out.
    counter: i32,
    // Buffers the data in.
    buffer: i32,
    bit_index: i32,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            control: io_registers::SerialControl(0),
            data: 0,
            counter: -50,
            buffer: 0,
            bit_index: 0,
        }
    }

    pub fn execute_tcycle(&self) -> (Controller, Interrupts) {
        #[allow(unused_mut)]
        let mut next_state = *self;
        #[allow(unused_mut)]
        let mut fire_interrupt = Interrupts::empty();
        // next_state.counter = self.counter.wrapping_add(1);
        // if self.control.is_transferring() {
        //     if self.counter >= 0 && (self.counter % 512) == 0 {
        //         // println!("dat: {:#010b}", self.data);
        //         // println!("buf: {:#010b}", self.buffer);
        //         // println!("Counter: {}", self.counter);
        //         // Clock in/out one bit.
        //         if self.bit_index < 7 {
        //             next_state.buffer = (next_state.buffer << 1) | ((self.data & 0x80) >> 7);
        //             next_state.data = ((self.data << 1) | 1) & 0xFF;
        //             next_state.bit_index += 1;
        //         } else {
        //             //assert_eq!(self.bit_index, 8);
        //             // End the transfer!
        //             next_state.control.set_transferring(false);
        //             fire_interrupt = Interrupts::SERIAL;
        //             next_state.buffer = 0;
        //             next_state.bit_index = 0;
        //         }
        //     }
        // } else {
        //     next_state.bit_index = 0;
        // }
        (next_state, fire_interrupt)
    }
}

impl mmu::MemoryMapped for Controller {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        match io_registers::Addresses::from_i32(raw) {
            Some(io_registers::Addresses::SerialControl) => Some(self.control.0 as i32),
            Some(io_registers::Addresses::SerialData) => Some(self.data),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
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
