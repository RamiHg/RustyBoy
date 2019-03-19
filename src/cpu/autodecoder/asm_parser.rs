use lazy_static::lazy_static;
use regex::Regex;

use super::asm::{AluCommand, Arg, Command, Op};
use crate::cpu::register::Register;

const OP_PATTERN: &str = r"([A-Z]+)[[:space:]]*([[:alnum:]]*),?[[:space:]]*([[:alnum:]]*)";

pub fn parse_op(op: &str) -> Op {
    lazy_static! {
        static ref OP_REGEX: Regex = Regex::new(OP_PATTERN).unwrap();
    }
    let groups = OP_REGEX
        .captures(op)
        .unwrap_or_else(|| panic!("Command: \"{}\" not a valid op format.", op));
    let cmd_str = groups
        .get(1)
        .unwrap_or_else(|| panic!("Command \"{}\" did not contain a valid op.", op))
        .as_str();
    use Command::*;
    let cmd = match cmd_str {
        "ADDR" => ADDR,
        "RADDR" => RADDR,
        "RD" => RD,
        "WR" => WR,
        "MOV" => MOV,
        "LD" => LD,
        "ALU" => ALUPlaceholder,
        "ADD" => ALU(AluCommand::Add),
        "FMSK" => FMSK,
        "FZ" => FZ,
        "CSE" => CSE,
        "INC" => INC,
        "DEC" => DEC,
        "END" => END,
        "CCEND" => CCEND,
        _ => panic!("Unexpected command: \"{}\"", cmd_str),
    };
    let lhs = groups.get(2).and_then(|x| parse_arg(x.as_str()));
    let rhs = groups.get(3).and_then(|x| parse_arg(x.as_str()));
    Op { cmd, lhs, rhs }
}

fn parse_arg(arg: &str) -> Option<Arg> {
    if arg.is_empty() {
        return None;
    }
    Some(match arg {
        "RHS" => Arg::Rhs,
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
        "PC_H" => Arg::Register(Register::PC_HIGH),
        "PC_L" => Arg::Register(Register::PC_LOW),
        "SP" => Arg::Register(Register::SP),
        "SP_H" => Arg::Register(Register::PC_HIGH),
        "SP_L" => Arg::Register(Register::PC_LOW),
        "RHS_H" => Arg::RhsHigh,
        "RHS_L" => Arg::RhsLow,
        "LHS" => Arg::Lhs,
        "LHS_L" => Arg::LhsLow,
        "LHS_H" => Arg::LhsHigh,
        _ if is_constant(arg) => Arg::ConstantPlaceholder(arg.into()),
        _ => panic!("Unknown arg: \"{}\"", arg),
    })
}

fn is_constant(value: &str) -> bool { value.parse::<i32>().is_ok() }
