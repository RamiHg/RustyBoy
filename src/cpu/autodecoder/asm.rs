use crate::cpu::register::Register;

#[derive(Debug)]
pub enum Command {
    ADDR,
    RADDR,
    RD,
    WR,
    MOV,
    LD,
    ALU,
    FMSK,
    FZ,
    CSE,
    INC,
    END,
}

#[derive(Debug)]
pub enum Arg {
    Register(Register),
    Rhs,
    RhsLow,
    RhsHigh,
    Lhs,
    LhsLow,
    LhsHigh,
    ConstantPlaceholder,
}

#[derive(Debug)]
pub struct Op {
    pub cmd: Command,
    pub lhs: Option<Arg>,
    pub rhs: Option<Arg>,
}
