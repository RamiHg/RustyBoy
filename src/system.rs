use crate::error::{self, Result};
use crate::{cart, cpu, mmu, util};

enum MemoryModules {
    Cart,
    Ram,
}

pub struct System {
    cpu: cpu::Cpu,

    memory: mmu::Memory,
    cart: Box<mmu::MemoryMapped>,
}

impl System {
    fn read_request(&self, raw_address: i32) -> Result<i32> {
        let modules = [&self.memory, self.cart.as_ref()];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in &modules {
            if let Some(result) = module.read(address) {
                return Ok(result);
            }
        }
        return Err(error::Type::InvalidOperation(format!(
            "Could not find any memory module accepting {:X?}",
            raw_address
        )));
    }

    fn write_request(&mut self, raw_address: i32, value: i32) -> Result<()> {
        let mut modules = [&mut self.memory, self.cart.as_mut()];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in &mut modules {
            if let Some(result) = module.write(address, value) {
                return result;
            }
        }
        return Err(error::Type::InvalidOperation(format!(
            "Could not find any memory module accepting {:X?}",
            raw_address
        )));
    }

    fn handle_cpu_memory_reads(&mut self) -> Result<()> {
        assert!(!(self.cpu.state.read_latch && self.cpu.state.write_latch));
        let t_state = self.cpu.t_state.get();
        if self.cpu.state.read_latch {
            if t_state == 3 {
                self.cpu.state.data_latch = self.read_request(self.cpu.state.address_latch)?;
            } else {
                // Write garbage in data latch to catch bad reads.
                self.cpu.state.data_latch = -1;
            }
        }
        Ok(())
    }

    fn handle_cpu_memory_writes(&mut self) -> Result<()> {
        // Service write requests at T=4's rising edge.
        if self.cpu.state.write_latch {
            assert!(util::is_16bit(self.cpu.state.address_latch));
            assert!(util::is_8bit(self.cpu.state.data_latch));
            if self.cpu.t_state.get() == 4 {
                self.write_request(self.cpu.state.address_latch, self.cpu.state.data_latch)?;
            }
        }
        Ok(())
    }

    fn execute_t_cycle(&mut self) -> Result<cpu::Output> {
        self.handle_cpu_memory_reads()?;

        let cpu_output = self.cpu.execute_t_cycle(&mut self.memory)?;
        self.handle_cpu_memory_writes()?;

        Ok(cpu_output)
    }

    pub fn execute_machine_cycle(&mut self) -> Result<cpu::Output> {
        let mut last_result = None;
        for i in 0..=3 {
            last_result = Some(self.execute_t_cycle()?);
        }
        Ok(last_result.unwrap())
    }
}

#[cfg(test)]
impl System {
    pub fn new_test_system(cart: Box<dyn mmu::MemoryMapped>) -> System {
        System {
            cpu: cpu::Cpu::new(),
            memory: mmu::Memory::new(),
            cart: cart,
        }
    }

    // pub fn memory_mut(&mut self) -> &mut mmu::Memory { &mut self.memory }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu { &mut self.cpu }

    pub fn memory_write(&mut self, raw_address: i32, value: i32) {
        self.write_request(raw_address, value).unwrap();
    }

    pub fn memory_read(&self, raw_address: i32) -> i32 { self.read_request(raw_address).unwrap() }
    pub fn memory_read_16(&self, raw_address: i32) -> i32 {
        self.memory_read(raw_address) | (self.memory_read(raw_address + 1) << 8)
    }
}
