use super::{Arg, Command, MaybeArg, Op};
use crate::cpu::alu;
use crate::cpu::register::Register;

pub fn parse_op(op: &str) -> Op {
    let mut parts = op.splitn(2, char::is_whitespace);
    let cmd_str =
        parts.next().unwrap_or_else(|| panic!("Command {} did not contain a valid op.", op));
    let mut args = parts.next().unwrap_or("").split(',').map(str::trim);
    let lhs = args.next().and_then(|x| parse_arg(x));
    let rhs = args.next().and_then(|x| parse_arg(x));
    use Command::*;
    let cmd = match cmd_str {
        "ADDR" => ADDR,
        "RADDR" => RADDR,
        "ADDR_H_FF" => ADDR_H_FF,
        "RD" => RD,
        "WR" => WR,
        "MOV" => MOV,
        "LD" => LD,
        "ALU" => AluPlaceholder,
        "ADD" => AluOp(alu::Op::Add),
        "ADC" => AluOp(alu::Op::Adc),
        "SUB" => AluOp(alu::Op::Sub),
        "AND" => AluOp(alu::Op::And),
        "BIT" => BIT,
        "FMSK" => FMSK,
        "FZ" => FZ,
        "CSE" => CSE,
        "INC" => INC,
        "DEC" => DEC,
        "END" => END,
        "CCEND" => CCEND,
        "EI" => EI,
        "DI" => DI,
        "CB" => CB,
        "HALT" => HALT,
        _ => panic!("Unexpected command: \"{}\"", cmd_str),
    };
    Op { cmd, lhs: MaybeArg::new(lhs), rhs: MaybeArg::new(rhs) }
}

fn parse_arg(arg: &str) -> Option<Arg> {
    if arg.is_empty() {
        return None;
    }
    Some(match arg {
        "A" => Arg::Register(Register::A),
        "ACT" => Arg::Register(Register::ACT),
        "TMP" => Arg::Register(Register::ALU_TMP),
        "H" => Arg::Register(Register::H),
        "L" => Arg::Register(Register::L),
        "W" => Arg::Register(Register::TEMP_HIGH),
        "Z" => Arg::Register(Register::TEMP_LOW),
        "WZ" => Arg::Register(Register::TEMP),
        "HL" => Arg::Register(Register::HL),
        "PC" => Arg::Register(Register::PC),
        "BC" => Arg::Register(Register::BC),
        "PC_H" => Arg::Register(Register::PC_HIGH),
        "PC_L" => Arg::Register(Register::PC_LOW),
        "SP" => Arg::Register(Register::SP),
        "SP_H" => Arg::Register(Register::SP_HIGH),
        "SP_L" => Arg::Register(Register::SP_LOW),
        "RHS" => Arg::Rhs,
        "RHS_H" => Arg::RhsHigh,
        "RHS_L" => Arg::RhsLow,
        "LHS" => Arg::Lhs,
        "LHS_L" => Arg::LhsLow,
        "LHS_H" => Arg::LhsHigh,
        "CC" => Arg::CCPlaceholder,
        "i32" => Arg::IntegerPlaceholder,
        "OP_Y8" => Arg::OpYMul8,
        _ => Arg::ConstantPlaceholder(arg.into()),
    })
}
