use crate::cart::test::ErrorCart;
use crate::cpu::alu::Flags;
use crate::cpu::register::Register;
use crate::cpu::*;
use crate::memory::Memory;
use crate::system::System;

mod test_16bit_alu;
mod test_8bit_alu;
mod test_flow;
mod test_load;
mod test_store;

pub struct TestSystem {
    cpu: Cpu,
    memory: Memory,
    cycles: i64,
}

impl System for TestSystem {
    fn execute_cpu_cycle(&mut self) -> Result<Output> {
        self.cycles += 1;
        self.cpu.execute_machine_cycle(&self.memory)
    }

    fn commit_memory_write(&mut self, raw_address: i32, value: i32) {
        self.memory.store(raw_address, value);
    }
}

pub struct TestContext(Box<TestSystem>);

pub fn with_default() -> TestContext {
    TestContext::with_default()
}

impl TestContext {
    fn with_default() -> TestContext {
        let memory = Memory::new(Box::new(ErrorCart));
        let cpu = Cpu::new();
        TestContext(Box::new(TestSystem {
            cpu,
            memory,
            cycles: 0,
        }))
    }

    pub fn set_mem_8bit(mut self, addr: i32, value: i32) -> TestContext {
        self.0.memory.store(addr, value);
        self
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> TestContext {
        self.0.cpu.registers.set(register, value);
        self
    }

    pub fn set_flag(mut self, flag: Flags, is_set: bool) -> TestContext {
        let mut current_flags = Flags::from_bits(self.0.cpu.registers.get(Register::F)).unwrap();
        current_flags.set(flag, is_set);
        self.0.cpu.registers.set(Register::F, current_flags.bits());
        self
    }

    pub fn set_carry(self, is_set: bool) -> TestContext {
        self.set_flag(Flags::CARRY, is_set)
    }

    pub fn set_zero(self, is_set: bool) -> TestContext {
        self.set_flag(Flags::ZERO, is_set)
    }

    pub fn set_sub(self, is_set: bool) -> TestContext {
        self.set_flag(Flags::SUB, is_set)
    }

    /// Brings up a System instance, sets it up, runs the given instructions, and returns the resulting
    /// system state.
    pub fn execute_instructions(mut self, instructions: &[u8]) -> TestContext {
        // Copy over the instructions into internal RAM.
        self = self.set_mem_range(0xC000, instructions);
        self.0.cpu.registers.set(Register::PC, 0xC000);
        // Don't let any test go longer than 100 cycles.
        let mut num_cycles_left = 100;
        while self.0.cpu.registers.get(Register::PC) != 0xC000 + instructions.len() as i32 {
            while !self.0.execute_machine_cycle().unwrap().is_done {}
            num_cycles_left -= 1;
            if num_cycles_left <= 0 {
                panic!("Test lasting longer than 100 cycles. Most likely infinite loop.");
            }
        }
        self
    }

    pub fn set_mem_range(mut self, address: usize, values: &[u8]) -> TestContext {
        self.0.memory.mem()[address..address + values.len()].copy_from_slice(values);
        self
    }

    pub fn assert_mcycles(self, cycles: i32) -> TestContext {
        assert_eq!(self.0.cycles, cycles.into());
        self
    }

    pub fn assert_reg_eq(self, register: Register, value: i32) -> TestContext {
        assert_eq!(self.0.cpu.registers.get(register), value);
        self
    }

    pub fn assert_mem_8bit_eq(self, address: i32, value: i32) -> TestContext {
        assert_eq!(self.0.memory.read(address), value);
        self
    }

    pub fn assert_mem_16bit_eq(self, address: i32, value: i32) -> TestContext {
        assert_eq!(
            i32::from(self.0.memory.read_general_16(address as usize)),
            value
        );
        self
    }

    // Flags register.
    pub fn assert_flags(self, expected: Flags) -> TestContext {
        let flags = Flags::from_bits(self.0.cpu.registers.get(Register::F)).unwrap();
        assert_eq!(flags, expected);
        self
    }
}
