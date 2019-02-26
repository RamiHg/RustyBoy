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

/// Stores information about what was done at each step of each
/// test. This is then later used to be able to export the tests
/// to aid in hardware verification.
enum Assertion {
    RegEq(Register, i32),
    MemRange { base: i32, values: Vec<u8> },
    Flags(Flags),
    MCycles(i32),
}

#[derive(Default)]
struct TestDescriptor {
    name: String,
    mem_setup: Vec<(i32, i32)>,
    reg_setup: Vec<(Register, i32)>,
    initial_pc: i32,
    num_instructions: i32,
    assertions: Vec<Assertion>,
}

impl TestDescriptor {
    fn add_mem_range(&mut self, base: i32, values: &[u8]) {
        for (i, &value) in values.iter().enumerate() {
            self.mem_setup.push((base + i as i32, value.into()));
        }
    }

    fn serialize(&self) -> String {
        let mut s = String::new();
        // Start off by printing the test name.
        s.push_str(&format!("test_name {}\n", self.name));
        // Serialize the memory values.
        let mut mem_setup = self.mem_setup.clone();
        mem_setup.sort_by(|(addr_lhs, _), (addr_rhs, _)| addr_lhs.cmp(addr_rhs));
        for (addr, value) in mem_setup {
            s.push_str(&format!("set_mem {:X?} {:X?}\n", addr, value));
        }
        // Serialize register values.
        let mut reg_setup = self.reg_setup.clone();
        reg_setup.sort_by(|(lhs, _), (rhs, _)| format!("{:?}", lhs).cmp(&format!("{:?}", rhs)));
        for (reg, value) in reg_setup {
            s.push_str(&format!("set_reg {:?} {:X?}\n", reg, value));
        }
        // Execute!
        s.push_str(&format!("execute {}\n", self.num_instructions));
        s
    }
}

pub struct TestSystem {
    cpu: Cpu,
    memory: Memory,
    cycles: i64,
    desc: TestDescriptor,
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
        // Figure out the test name.
        let bt = backtrace::Backtrace::new();
        let name = format!("{:?}", bt.frames()[2].symbols()[0].name().unwrap());

        let memory = Memory::new(Box::new(ErrorCart));
        let cpu = Cpu::new();
        TestContext(Box::new(TestSystem {
            cpu,
            memory,
            cycles: 0,
            desc: TestDescriptor {
                name,
                ..Default::default()
            },
        }))
    }

    pub fn set_mem_range(mut self, address: usize, values: &[u8]) -> TestContext {
        self.0.memory.mem()[address..address + values.len()].copy_from_slice(values);
        self.0.desc.add_mem_range(address as i32, values);
        self
    }

    pub fn set_mem_8bit(self, addr: i32, value: i32) -> TestContext {
        self.set_mem_range(addr as usize, &[value as u8])
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> TestContext {
        self.0.cpu.registers.set(register, value);
        self.0.desc.reg_setup.push((register, value));
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
        // Capture the flags at the time of execution, rather than each bit set. Can possible
        // do this for registers as well.
        self.0
            .desc
            .reg_setup
            .push((Register::F, self.0.cpu.registers.get(Register::F)));
        self.0.desc.add_mem_range(0xC000, instructions);
        self.0.desc.initial_pc = 0xC000;
        self.0.desc.num_instructions = instructions.len() as i32;
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

    pub fn assert_mcycles(mut self, cycles: i32) -> TestContext {
        self.0.desc.assertions.push(Assertion::MCycles(cycles));
        assert_eq!(self.0.cycles, cycles.into());
        // Serialize the nuggets! (TODO: Kinda hacky. Make test trait that just prints)
        println!("{}", self.0.desc.serialize());
        self
    }

    pub fn assert_reg_eq(mut self, register: Register, value: i32) -> TestContext {
        self.0
            .desc
            .assertions
            .push(Assertion::RegEq(register, value));
        assert_eq!(self.0.cpu.registers.get(register), value);
        self
    }

    /// Only used for nugget creation.
    fn make_assert_mem_nugget(&mut self, base: i32, values: &[u8]) {
        self.0.desc.assertions.push(Assertion::MemRange {
            base,
            values: values.to_vec(),
        });
    }

    pub fn assert_mem_8bit_eq(mut self, address: i32, value: i32) -> TestContext {
        self.make_assert_mem_nugget(address, &[value as u8]);
        assert_eq!(self.0.memory.read(address), value);
        self
    }

    pub fn assert_mem_16bit_eq(mut self, address: i32, value: i32) -> TestContext {
        self.make_assert_mem_nugget(address, &[value as u8, (value >> 8) as u8]);
        assert_eq!(
            i32::from(self.0.memory.read_general_16(address as usize)),
            value
        );
        self
    }

    // Flags register.
    pub fn assert_flags(mut self, expected: Flags) -> TestContext {
        let flags = Flags::from_bits(self.0.cpu.registers.get(Register::F)).unwrap();
        self.0
            .desc
            .assertions
            .push(Assertion::RegEq(Register::F, flags.bits()));
        assert_eq!(flags, expected);
        self
    }
}
