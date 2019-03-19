use crate::cpu::register::Register;

#[derive(Debug)]
pub enum AluCommand {
    Mov,
    Add,
    Addc,
    Sub,
    Subc,
    And,
    Xor,
    Or,
    Cp,
    Cpl,
    Daa,
}

#[derive(Debug)]
pub enum Command {
    ADDR,
    CCEND,
    RADDR,
    RD,
    WR,
    MOV,
    LD,
    ALUPlaceholder,
    ALU(AluCommand),
    FMSK,
    FZ,
    CSE,
    INC,
    DEC,
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
    ConstantPlaceholder(String),
}

#[derive(Debug)]
pub struct Op {
    pub cmd: Command,
    pub lhs: Option<Arg>,
    pub rhs: Option<Arg>,
}

impl Arg {
    pub fn expect_as_register(&self) -> &Register {
        match self {
            Arg::Register(register) => register,
            _ => panic!("Expected register. Actually: {:?}", self),
        }
    }
}
