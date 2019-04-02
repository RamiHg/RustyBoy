use crate::error::Result;
use crate::{cart, cpu, memory};

pub struct System {
    cpu: cpu::Cpu,
    memory: Box<memory::Memory>,
}

impl System {
    fn execute_t_cycle(&mut self) -> Result<cpu::Output> {
        let cpu_output = self.cpu.execute_t_cycle(&self.memory)?;
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
        self.cpu.handle_interrupts(&self.memory)?;
        Ok(last_result.unwrap())
    }
}

#[cfg(test)]
impl System {
    pub fn new_test_system() -> System {
        System {
            cpu: cpu::Cpu::new(),
            memory: Box::new(memory::Memory::new(Box::new(cart::test::Cart))),
        }
    }

    pub fn memory_mut(&mut self) -> &mut memory::Memory { &mut self.memory }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu { &mut self.cpu }
}
