use crate::error::Result;
use crate::{cart, cpu, mmu, timer};

pub struct System {
    cpu: cpu::Cpu,
    ram: mmu::Ram,
    cart: cart::Cart,
}

impl System {

    fn read_request(&mut self, raw_address: i32) -> Result<i32> {
        let modules: [&mut dyn mmu::MemoryMapped] = [&mut self.cart, &mut self.ram];
        for module in modules {
            if let Some(value) = module.read()
        }
    }

    fn handle_memory(&mut self) -> Result<()> {
        assert!(!(self.cpu.state.read_latch && self.cpu.state.write_latch));
        let t_state = self.cpu.t_state.get();
        if self.cpu.state.read_latch {
            if t_state == 3 {
                self.cpu.state.data_latch = 
            }
        }
    }

    fn execute_t_cycle(&mut self) -> Result<cpu::Output> {
        let cpu_output = self.cpu.execute_t_cycle(&mut self.memory)?;
        if let Some(cpu::SideEffect::Write { raw_address, value }) = cpu_output.side_effect {
            self.memory.store(raw_address, value);
        };
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
    pub fn new_test_system(cart: Box<dyn cart::Cart>) -> System {
        System {
            cpu: cpu::Cpu::new(),
            memory: Box::new(memory::Memory::new(cart)),
        }
    }

    pub fn memory_mut(&mut self) -> &mut memory::Memory { &mut self.memory }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu { &mut self.cpu }
}
