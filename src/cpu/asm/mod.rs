/// This library handles the parsing and compilation of the high-level assembly down to CPU
/// micro- codes.
mod compiler;
pub mod csv_loader;
pub mod op_map;
mod parser;

use crate::cpu::alu;
use crate::cpu::micro_code;
use crate::cpu::register::Register;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    ADDR,
    RADDR,
    ADDR_H_FF,
    ADDR_H_00,
    RD,
    WR,
    MOV,
    LD,
    AluPlaceholder,
    AluOp(alu::Op),
    FMSK,
    FZ,
    CSE,
    INC,
    DEC,
    END,
    CCEND,
    NOP,
    EI,
    DI,
}

#[derive(Clone, Debug)]
pub enum Arg {
    Register(Register),
    CC(micro_code::Condition),
    Rhs,
    RhsLow,
    RhsHigh,
    Lhs,
    LhsLow,
    LhsHigh,
    ConstantPlaceholder(String),
    CCPlaceholder,
}

#[derive(Debug, Clone)]
pub struct MaybeArg(pub Option<Arg>);

#[derive(Debug, Clone)]
pub struct Op {
    pub cmd: Command,
    pub lhs: MaybeArg,
    pub rhs: MaybeArg,
}

impl Op {
    pub fn nop() -> &'static [Op] {
        static NOP: [Op; 1] = [Op {
            cmd: Command::NOP,
            lhs: MaybeArg(None),
            rhs: MaybeArg(None),
        }];
        &NOP
    }
}

impl MaybeArg {
    pub fn new(arg: Option<Arg>) -> MaybeArg { MaybeArg(arg) }

    pub fn expect_as_register(&self) -> Register {
        match self.0 {
            Some(Arg::Register(register)) => register,
            _ => panic!("Expected register. Actually: {:?}", self),
        }
    }

    pub fn expect_as_pair(&self) -> Register {
        let register = self.expect_as_register();
        if !register.is_pair() {
            panic!("Expected 16-bit register. Got: {:?}", register);
        }
        register
    }

    pub fn expect_none(&self) {
        if self.0.is_some() {
            panic!("Unexpected argument: {:?}", self.0.as_ref().unwrap());
        }
    }
}
