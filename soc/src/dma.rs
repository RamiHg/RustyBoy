use super::io_registers::{self, Addresses, Register};
use super::mmu;

pub struct DmaRequest {
    pub source_address: i32,
    pub destination_address: i32,
}

define_int_register!(Control, Addresses::Dma);

#[derive(Clone, Copy, Debug)]
pub struct Dma {
    // The DMA control register.
    control: Control,
    byte_index: i32,
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            control: Control(0),
            byte_index: 0,
        }
    }

    pub fn is_active(&self) -> bool { self.byte_index > 0 && self.byte_index <= 160 }

    pub fn execute_tcycle(self: Box<Self>, bus: &mmu::MemoryBus) -> (Box<Dma>, Option<DmaRequest>) {
        let mut next_state = Box::new(*self);
        let mut dma_request = None;
        if bus.t_state == 4 && self.byte_index > 0 {
            if self.byte_index <= 160 {
                dma_request = Some(DmaRequest {
                    source_address: (self.control.0 << 8) + 160 - self.byte_index,
                    destination_address: 0xFE00 + 160 - self.byte_index,
                });
            }
            next_state.byte_index -= 1;
        }
        if bus.writes_to_reg(self.control) {
            next_state.control.set_from_bus(bus);
            next_state.byte_index = 161;
        }
        (next_state, dma_request)
    }

    #[cfg(test)]
    pub fn set_control(&mut self, value: i32) {
        self.control.0 = value;
        self.byte_index = 160;
    }
}

impl mmu::MemoryMapped for Dma {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        if raw == io_registers::Addresses::Dma as i32 {
            Some(self.control.0)
        } else {
            None
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        if raw == io_registers::Addresses::Dma as i32 {
            Some(())
        } else {
            None
        }
    }
}
