/// Glossary:
/// cycle: One system cycle (clocked at 4MHz).
/// mcycle: One "machine" cycle (clocked at 1MHz).
//use crate::alu;
use core::convert::Into;

use crate::cpu::cpu;
use crate::cpu::register::{Register, SingleTable};
use crate::memory::Memory;
use crate::mmu;

use cpu::Cpu;

pub enum InstructionType {
    IndirectLoad(IndirectLoad),
}

impl InstructionType {
    pub fn as_mut_instruction(&mut self) -> &mut dyn Instruction {
        use InstructionType::*;
        match self {
            IndirectLoad(i) => i,
        }
    }
}

#[derive(PartialEq)]
pub enum InstrResult {
    None,
    Write(mmu::Write),
    Done,
}

type Result = cpu::Result<InstrResult>;

pub trait Instruction /*: core::fmt::Display */ {
    fn execute_cycle(&mut self, cycle: i32, cpu: &mut Cpu, memory: &Memory) -> Result;

    /// Virtually all instructions need to get decoded first. Rather than emulating the decoder,
    /// we fake it by "reading" and incrementing PC after the fact.
    fn simulate_decode(&self, cpu: &mut Cpu) {
        // TODO: Wrapping?
        cpu.registers
            .set(Register::PC, cpu.registers.get(Register::PC) + 1);
    }
}

/// Can be added to an instruction to signify (HL+) or (HL-).
#[derive(Clone, Copy)]
pub enum HLOp {
    None,
    Inc,
    Dec,
}

/// Covers indirect loading from an (XX) location to a register.
/// E.g.
/// LD (BC), A
/// LD A, (DE)
#[derive(Clone, Copy)]
pub struct IndirectLoad {
    source: Register,
    destination: SingleTable,
    hl_op: HLOp,
}

impl IndirectLoad {
    pub fn new(destination: SingleTable, source: Register, hl_op: HLOp) -> IndirectLoad {
        assert!(destination != SingleTable::HL);
        assert!(source.is_pair());
        IndirectLoad {
            source,
            destination,
            hl_op,
        }
    }
}

impl Instruction for IndirectLoad {
    fn execute_cycle(&mut self, mcycle: i32, cpu: &mut Cpu, memory: &Memory) -> Result {
        if mcycle == 0 {
            self.simulate_decode(cpu);
            return Ok(InstrResult::None);
        }
        // MEMORY.
        let source_value = memory.read(cpu.registers.get(self.source));
        cpu.registers.set(self.destination.into(), source_value);
        // Increment or decrement HL if needed.
        match self.hl_op {
            HLOp::Inc => cpu.registers.set(
                Register::HL,
                (cpu.registers.get(Register::HL) as u16).wrapping_add(1) as i32,
            ),
            HLOp::Dec => cpu.registers.set(
                Register::HL,
                (cpu.registers.get(Register::HL) as u16).wrapping_sub(1) as i32,
            ),
            HLOp::None => (),
        }
        Ok(InstrResult::Done)
    }
}

impl Into<InstructionType> for IndirectLoad {
    fn into(self) -> InstructionType {
        InstructionType::IndirectLoad(self)
    }
}
