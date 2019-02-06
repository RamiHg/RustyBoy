use crate::cart::test::ErrorCart;
use crate::cpu::register::Register;
use crate::cpu::*;
use crate::memory::Memory;
use crate::system::System;
use crate::util::*;

mod test_load;

pub struct TestSystem {
    cpu: Cpu,
    memory: Memory,
}

impl System for TestSystem {
    fn execute_cpu_cycle(&mut self) -> Result<Output> {
        self.cpu.execute_machine_cycle(&self.memory)
    }

    fn commit_memory_write(&mut self, raw_address: i32, value: i32) {
        self.memory.store(raw_address, value);
    }
}

pub struct TestContext(TestSystem);

pub fn with_default() -> TestContext {
    TestContext::with_default()
}

impl TestContext {
    fn with_default() -> TestContext {
        let memory = Memory::new(Box::new(ErrorCart));
        let cpu = Cpu::new();
        TestContext(TestSystem { cpu, memory })
    }

    pub fn set_mem_8bit(mut self, addr: i32, value: i32) -> TestContext {
        self.0.memory.store(addr, value);
        self
    }

    pub fn set_mem_16bit(mut self, addr: i32, value: i32) -> TestContext {
        assert!(is_16bit(value));
        self.0.memory.store_general_16(addr as usize, value as u16);
        self
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> TestContext {
        self.0.cpu.registers.set(register, value);
        self
    }

    /// Brings up a System instance, sets it up, runs the given instructions, and returns the resulting
    /// system state.
    pub fn execute_instructions(mut self, instructions: &[u8]) -> TestContext {
        // Copy over the instructions into internal RAM.
        self.0.memory.mem()[0xC000..0xC000 + instructions.len()].copy_from_slice(instructions);
        self.0.cpu.registers.set(Register::PC, 0xC000);
        while self.0.cpu.registers.get(Register::PC) < 0xC000 + instructions.len() as i32 {
            while !self.0.execute_machine_cycle().unwrap().is_done {}
        }
        assert_eq!(
            self.0.cpu.registers.get(Register::PC),
            0xC000 + instructions.len() as i32
        );
        self
    }

    pub fn assert_reg_eq(self, register: Register, value: i32) -> TestContext {
        assert_eq!(self.0.cpu.registers.get(register), value);
        self
    }
}
