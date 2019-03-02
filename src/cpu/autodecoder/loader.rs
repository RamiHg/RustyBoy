use super::*;
use crate::cpu::register::Register;

use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
struct RowRule {
  name: String,
  cycles: i32,
  nonbranch_cycles: Option<i32>,
  opcode_count: i32,
}

#[derive(Debug)]
struct MCycleRule {
  rw: Option<String>,
  addr: Option<String>,
  data: Option<String>,
  tmp: Option<String>,
  tmpf: Option<String>,
  inc: Option<String>,
  addr_2: Option<String>,
  op: Option<String>,
  rd: Option<String>,
  rs: Option<String>,
  end: Option<String>,
}

enum AluSource {
  Register(Register),
  Memory,
  Rhs,
  RhsLow,
  RhsHigh,
  Lhs,
  LhsLow,
  LhsHigh,
  FF,
  SignExtendTmp,
  ConstantPlaceholder,
}

enum Incrementer {
  Inc(AluSource),
  Dec(AluSource),
  MovPc(AluSource),
  Mov(AluSource),
}

enum AluOp {
  ADD,
  ADDC,
  ADDC1,
  MOV,
  SUB,
  DAA,
  CPL,
  Placeholder,
}

struct HLMicroCode {
  memory_control: Option<MemoryMode>,
  register_select: Option<Register>,

  alu_tmp: Option<AluSource>,
}

fn extract_mcycle(mcycle: usize, record: &csv::StringRecord) -> MCycleRule {
  let i = 4 + 12 * mcycle;
  let to_maybe_string = |x: &str| {
    if x.is_empty() {
      None
    } else {
      Some(String::from(x))
    }
  };

  MCycleRule {
    rw: record.get(i).and_then(to_maybe_string),
    addr: record.get(i + 1).and_then(to_maybe_string),
    data: record.get(i + 3).and_then(to_maybe_string),
    tmp: record.get(i + 4).and_then(to_maybe_string),
    tmpf: record.get(i + 5).and_then(to_maybe_string),
    inc: record.get(i + 6).and_then(to_maybe_string),
    addr_2: record.get(i + 7).and_then(to_maybe_string),
    op: record.get(i + 8).and_then(to_maybe_string),
    rd: record.get(i + 9).and_then(to_maybe_string),
    rs: record.get(i + 10).and_then(to_maybe_string),
    end: record.get(i + 11).and_then(to_maybe_string),
  }
}

pub fn load(filename: &str) {
  let f = std::fs::File::open(filename).unwrap();
  let mut rdr = csv::Reader::from_reader(f);
  // Ignore the first two lines.
  for result in rdr.records().skip(2) {
    let record: csv::StringRecord = result.unwrap();
    println!("{:?}", record);

    for i in 0..=6 {
      interpret_mcycle(extract_mcycle(i, &record));
    }
  }
}

fn get_alu_src(val: String) -> AluSource {
  match val.as_str() {
    "RHS" => AluSource::Rhs,
    "A" => AluSource::Register(Register::A),
    "H" => AluSource::Register(Register::H),
    "L" => AluSource::Register(Register::L),
    "W" => AluSource::Register(Register::TEMP_HIGH),
    "Z" => AluSource::Register(Register::TEMP_LOW),
    "C" => AluSource::Register(Register::C),
    "WZ" => AluSource::Register(Register::TEMP),
    "HL" => AluSource::Register(Register::HL),
    "PC_H" => AluSource::Register(Register::PC_HIGH),
    "PC_L" => AluSource::Register(Register::PC_LOW),
    "SP" => AluSource::Register(Register::SP),
    "SP_H" => AluSource::Register(Register::PC_HIGH),
    "SP_L" => AluSource::Register(Register::PC_LOW),
    "RHS_H" => AluSource::RhsHigh,
    "RHS_L" => AluSource::RhsLow,
    "LHS" => AluSource::Lhs,
    "LHS_L" => AluSource::LhsLow,
    "LHS_H" => AluSource::LhsHigh,
    "OP" => AluSource::Register(Register::INSTR),
    "FF" => AluSource::FF,
    "MEM" => AluSource::Memory,
    "SE" => AluSource::SignExtendTmp,
    "CONST" => AluSource::ConstantPlaceholder,
    _ => panic!("Unexpected value: {}.", val),
  }
}

fn interpret_mcycle(rule: MCycleRule) {
  let memory_control = MemoryControl {
    mode: if let Some(rw) = rule.rw {
      if rw == "R" {
        Some(MemoryMode::Read)
      } else if rw == "W" {
        Some(MemoryMode::Write)
      } else {
        panic!("Unexpected R/W value: {}.", rw)
      }
    } else {
      None
    },
    address_source: if let Some(addr) = rule.addr {
      match addr.as_str() {
        "LHS" => None, // Todo...
        "RHS" => None, // Todo...
        "PC" => Some(Register::PC),
        "SP" => Some(Register::SP),
        "HL" => Some(Register::HL),
        "WZ" => Some(Register::TEMP),
        _ => panic!("Unexpected ADDR: {}.", addr),
      }
    } else {
      None
    },
  };
  // Data
  let register_select = rule.data.map(get_alu_src);
  let alu_tmp = rule.tmp.map(get_alu_src);
  // INC.
  let incrementer = match rule.inc {
    Some(val) => Some(match val.as_str() {
      "Y" => Incrementer::Inc(AluSource::Register(Register::PC)),
      "INC" => Incrementer::Inc(get_alu_src(rule.addr_2.unwrap())),
      "DEC" => Incrementer::Dec(get_alu_src(rule.addr_2.unwrap())),
      "MOVPC" => Incrementer::MovPc(get_alu_src(rule.addr_2.unwrap())),
      "MOV" => Incrementer::Mov(get_alu_src(rule.addr_2.unwrap())),
      _ => panic!("Unexpected INC: {}.", val),
    }),
    None => None,
  };
  // OP.
  let op = match rule.op {
    Some(val) => Some(match val.as_str() {
      "ADD" => AluOp::ADD,
      "ADDC" => AluOp::ADDC,
      "ADDC1" => AluOp::ADDC1,
      "MOV" => AluOp::MOV,
      "SUB" => AluOp::SUB,
      "DAA" => AluOp::DAA,
      "CPL" => AluOp::CPL,
      "OP" => AluOp::Placeholder,
      _ => panic!("Unexpected ALU OP: {}.", val),
    }),
    None => None,
  };
}
