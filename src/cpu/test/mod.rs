use crate::cart::test::ErrorCart;
use crate::cpu::micro_code::InstrResult;
use crate::cpu::register::Register;
use crate::cpu::*;
use crate::memory::Memory;
use crate::util::*;

mod test_load;

pub struct TestContext(State);

pub struct State {
    cpu: Cpu,
    memory: Memory,
}

pub fn with_default() -> TestContext {
    TestContext::with_default()
}

impl TestContext {
    fn with_default() -> TestContext {
        let memory = Memory::new(Box::new(ErrorCart));
        let cpu = Cpu::new();
        TestContext(State { cpu, memory })
    }

    pub fn set_mem_8bit(self, addr: i32, value: i32) -> TestContext {
        assert!(is_8bit(value));
        assert!(is_16bit(addr));
        let mut state = self.0;
        state.memory.store_general_8(addr as usize, value as u8);
        TestContext(state)
    }

    pub fn set_reg_pair(mut self, register: Register, value: i32) -> TestContext {
        assert!(is_16bit(value));
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
            while self.0.cpu.execute_machine_cycle(&self.0.memory).unwrap() != InstrResult::Done {}
        }
        self
    }

    pub fn assert_reg_eq(self, register: Register, value: i32) -> TestContext {
        assert_eq!(self.0.cpu.registers.get(register), value);
        self
    }
}
