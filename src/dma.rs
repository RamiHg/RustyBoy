use super::io_registers;
/// since the DMA interface is simple enough.
use super::mmu;

pub struct DmaRequest {
    pub source_address: i32,
    pub destination_address: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Dma {
    // The DMA control register.
    control: i32,
    source_address: i32,
    destination_address: i32,
    // Simulate 1mcycle setup delay.
    in_setup: bool,
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            control: 0,
            source_address: 0,
            destination_address: 0xFFFF,
            in_setup: false,
        }
    }

    /// Executes a DMA machine cycle. The second tuple value specifies a memory write request
    /// (address, value).
    pub fn execute_mcycle(mut self) -> (Dma, Option<DmaRequest>) {
        let dma_request = if !self.in_setup && self.destination_address < 0xFEA0 {
            let request = DmaRequest {
                source_address: self.source_address,
                destination_address: self.destination_address,
            };
            self.source_address += 1;
            self.destination_address += 1;
            Some(request)
        } else {
            None
        };
        self.in_setup = false;
        (self, dma_request)
    }
}

impl mmu::MemoryMapped for Dma {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw) = address;
        if raw == io_registers::Addresses::Dma as i32 {
            Some(self.control)
        } else {
            None
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        if raw == io_registers::Addresses::Dma as i32 {
            self.control = value;
            self.source_address = value << 8;
            self.destination_address = 0xFE00; // Beginning of OAM.
            self.in_setup = true;
            Some(())
        } else {
            None
        }
    }
}
