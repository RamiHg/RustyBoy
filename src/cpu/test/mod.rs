use crate::{
    cpu::{alu::Flags, register::Register, *},
    mmu::Memory,
};

use crate::cart;
use crate::mmu;
use crate::system;

mod test_16bit_alu;
mod test_8bit_alu;
mod test_cb_alu;
mod test_flow;
mod test_interrupts;
mod test_load;
mod test_push_pop;
mod test_store;

pub mod instructions {
    use super::Register::{self, *};

    pub const ADD_IMM: u8 = 0xC6;
    pub const RET: u8 = 0xC9;
    pub const RETI: u8 = 0xD9;
    pub const EI: u8 = 0xFB;
    pub const DI: u8 = 0xF3;
    pub const LD_A_IMM: u8 = 0x3E;
    pub const LD_A_A: u8 = 0x7F;
    pub const JP: u8 = 0xC3;
    pub const INC_A: u8 = 0x3C;
    pub const DEC_A: u8 = 0x3D;

    pub const UNARY_SOURCES: [Register; 8] = [B, C, D, E, H, L, HL, A];
}

pub use instructions::*;

/// Stores information about what was done at each step of each
/// test. This is then later used to be able to export the tests
/// to aid in hardware verification.
enum Assertion {
    RegEq(Register, i32),
    MemRange { base: i32, values: Vec<u8> },
    MCycles(i32),
}

impl Assertion {
    fn serialize(&self) -> String {
        use Assertion::*;
        match self {
            RegEq(register, value) => format!("ASSERT_REG {:?} {:X?}\n", register, value),
            MemRange { base, values } => values
                .iter()
                .enumerate()
                .map(|(i, &x)| format!("ASSERT_MEM {:X?} {:X?}\n", base + i as i32, x))
                .collect(),
            MCycles(cycles) => format!("ASSERT_MCYCLES {}\n", cycles),
        }
    }
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
        // Serialize PC.
        s.push_str(&format!("PC {:X?}\n", self.initial_pc));
        // Execute!
        s.push_str(&format!("execute {}\n", self.num_instructions));
        // Serialize assertions.
        self.assertions
            .iter()
            .for_each(|x| s.push_str(&x.serialize()));
        s
    }
}

pub struct TestContext {
    system: Box<system::System>,
    desc: TestDescriptor,
    cycles: i64,
    interrupt_fire_at_tcycle: i32,
    interrupts_fired: i32,
}

pub fn with_default() -> TestContext {
    TestContext::with_default(Box::new(cart::test::DynamicCart::new()))
}
pub fn with_dynamic_cart() -> TestContext {
    TestContext::with_default(Box::new(cart::test::DynamicCart::new()))
}

impl TestContext {
    fn with_default(cart: Box<dyn mmu::MemoryMapped>) -> TestContext {
        // Figure out the test name.
        // let bt = backtrace::Backtrace::new();
        // let first_non_setup = bt.frames()[2..]
        //     .iter()
        //     .flat_map(|x| x.symbols()[0].name().and_then(|y| y.as_str()))
        //     .filter(|y| !y.contains("::setup"))
        //     .nth(0)
        //     .unwrap();
        // let name = first_non_setup.to_string();
        let name = "ignoreme".to_string();

        TestContext {
            system: Box::new(system::System::new_test_system(cart)),
            desc: TestDescriptor {
                name,
                ..Default::default()
            },
            cycles: 0,
            interrupt_fire_at_tcycle: -1,
            interrupts_fired: 0,
        }
    }

    pub fn set_mem_range(mut self, address: usize, values: &[u8]) -> TestContext {
        for (i, value) in (address..address + values.len()).zip(values.iter()) {
            self.system.memory_write(i as i32, *value as i32);
        }
        // self.system.memory_mut().mem()[address..address + values.len()].copy_from_slice(values);
        self.desc.add_mem_range(address as i32, values);
        self
    }

    pub fn set_mem_8bit(self, addr: i32, value: i32) -> TestContext {
        self.set_mem_range(addr as usize, &[value as u8])
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> TestContext {
        self.system.cpu_mut().registers.set(register, value);
        self.desc.reg_setup.push((register, value));
        self
    }

    pub fn set_flag(mut self, flag: Flags, is_set: bool) -> TestContext {
        let mut current_flags =
            Flags::from_bits(self.system.cpu_mut().registers.get(Register::F)).unwrap();
        current_flags.set(flag, is_set);
        self.system
            .cpu_mut()
            .registers
            .set(Register::F, current_flags.bits());
        self
    }

    pub fn set_carry(self, is_set: bool) -> TestContext { self.set_flag(Flags::CARRY, is_set) }

    pub fn set_zero(self, is_set: bool) -> TestContext { self.set_flag(Flags::ZERO, is_set) }

    pub fn set_sub(self, is_set: bool) -> TestContext { self.set_flag(Flags::SUB, is_set) }

    pub fn fire_interrupts_at(mut self, tcycle: i32, interrupts_fired: i32) -> TestContext {
        self.interrupt_fire_at_tcycle = tcycle;
        self.interrupts_fired = interrupts_fired;
        self
    }

    /// Brings up a System instance, sets it up, runs the given instructions, and returns the
    /// resulting system state.
    pub fn execute_instructions_for_mcycles(
        mut self,
        instructions: &[u8],
        mcycles: i32,
    ) -> TestContext {
        // Capture the flags at the time of execution, rather than each bit set. Can possible
        // do this for registers as well.
        self.desc.reg_setup.push((
            Register::F,
            self.system.cpu_mut().registers.get(Register::F),
        ));
        self.desc.add_mem_range(0xC000, instructions);
        self.desc.initial_pc = 0xC000;
        self.desc.num_instructions = instructions.len() as i32;
        // Copy over the instructions into internal RAM.
        self = self.set_mem_range(0xC000, instructions);
        self.system.cpu_mut().registers.set(Register::PC, 0xC000);
        // Don't let any test go longer than 100 cycles.
        let mut num_cycles_left = if mcycles > 0 { mcycles } else { 100 };
        let mut has_completed_instruction = false;
        while (self.system.cpu_mut().registers.get(Register::PC)
            != 0xC000 + instructions.len() as i32)
            || !has_completed_instruction
        {
            let cpu_output = self.system.execute_machine_cycle().unwrap();
            self.cycles += 1;
            num_cycles_left -= 1;
            has_completed_instruction = cpu_output.is_done;
            if num_cycles_left <= 0 {
                if mcycles > 0 {
                    break;
                } else {
                    panic!("Test lasting longer than 100 cycles. Most likely infinite loop.");
                }
            }
        }
        self
    }

    pub fn execute_instructions(self, instructions: &[u8]) -> TestContext {
        self.execute_instructions_for_mcycles(instructions, -1)
    }

    pub fn assert_mcycles(mut self, cycles: i32) -> TestContext {
        self.desc.assertions.push(Assertion::MCycles(cycles));
        assert_eq!(self.cycles, cycles.into());
        // Serialize the nuggets! (TODO: Kinda hacky. Make test trait that just prints)
        let data = self.desc.serialize();
        use std::{fs::OpenOptions, io::prelude::*};
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("test_data.txt")
            .unwrap();
        file.write_all(data.as_bytes()).unwrap();
        self
    }

    pub fn assert_reg_eq(mut self, register: Register, value: i32) -> TestContext {
        self.desc.assertions.push(Assertion::RegEq(register, value));
        let reg_value = self.system.cpu_mut().registers.get(register);
        assert_eq!(reg_value, value, "{:X?} != {:X?}", reg_value, value);
        self
    }

    /// Only used for nugget creation.
    fn make_assert_mem_nugget(&mut self, base: i32, values: &[u8]) {
        self.desc.assertions.push(Assertion::MemRange {
            base,
            values: values.to_vec(),
        });
    }

    pub fn assert_mem_8bit_eq(mut self, address: i32, value: i32) -> TestContext {
        self.make_assert_mem_nugget(address, &[value as u8]);
        assert_eq!(self.system.memory_read(address), value);
        self
    }

    pub fn assert_mem_16bit_eq(mut self, address: i32, value: i32) -> TestContext {
        self.make_assert_mem_nugget(address, &[value as u8, (value >> 8) as u8]);
        let mem_value = i32::from(self.system.memory_read_16(address));
        assert_eq!(mem_value, value, "{:X?} != {:X?}", mem_value, value);
        self
    }

    // Flags register.
    pub fn assert_flags(mut self, expected: Flags) -> TestContext {
        let flags = Flags::from_bits(self.system.cpu_mut().registers.get(Register::F)).unwrap();
        self.desc
            .assertions
            .push(Assertion::RegEq(Register::F, flags.bits()));
        assert_eq!(flags, expected);
        self
    }
}
