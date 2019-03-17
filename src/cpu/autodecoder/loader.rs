use std::collections::HashMap;

use super::AluOp;
use crate::cpu::alu;
use crate::cpu::register::Register;

#[derive(Debug)]
struct MCycleRule {
    rw: Option<String>,
    addr: Option<String>,
    data: Option<String>,
    tmp: Option<String>,
    act: Option<String>,
    tmpf: Option<String>,
    inc: Option<String>,
    addr_2: Option<String>,
    op: Option<String>,
    rd: Option<String>,
    rs: Option<String>,
    end: Option<String>,
}

fn extract_mcycle(mcycle: usize, record: &csv::StringRecord) -> MCycleRule {
    let i = 4 + 13 * mcycle;
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
        act: record.get(i + 5).and_then(to_maybe_string),
        tmpf: record.get(i + 6).and_then(to_maybe_string),
        inc: record.get(i + 7).and_then(to_maybe_string),
        addr_2: record.get(i + 8).and_then(to_maybe_string),
        op: record.get(i + 9).and_then(to_maybe_string),
        rd: record.get(i + 10).and_then(to_maybe_string),
        rs: record.get(i + 11).and_then(to_maybe_string),
        end: record.get(i + 12).and_then(to_maybe_string),
    }
}

#[derive(Clone)]
pub struct HLMicroCodeArray(pub Vec<HLMicroCode>);

pub fn load(filename: &str) -> HashMap<String, HLMicroCodeArray> {
    let mut rdr = csv::Reader::from_path(filename).unwrap();
    let mut code_map = HashMap::new();
    // Ignore the first two lines.
    for result in rdr.records().skip(2) {
        let record: csv::StringRecord = result.unwrap();
        let name = &record[0];

        let mut mcycles = Vec::new();
        for i in 0..=6 {
            let code = interpret_mcycle(extract_mcycle(i, &record));
            let is_end = if let EndMode::Yes = code.end_mode {
                true
            } else {
                false
            };
            mcycles.push(code);
            if is_end {
                break;
            }
        }
        code_map.insert(name.replace(" ", "").to_string(), HLMicroCodeArray(mcycles));
    }
    code_map
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpSource {
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

impl OpSource {
    pub fn as_address(self) -> Register {
        match self {
            OpSource::Register(register) => {
                assert!(register.is_pair());
                register
            }
            _ => panic!("Not address."),
        }
    }

    pub fn as_single(self) -> Register {
        match self {
            OpSource::Register(register) => {
                assert!(register.is_single());
                register
            }
            _ => panic!("Not single."),
        }
    }
}

impl From<String> for OpSource {
    fn from(val: String) -> OpSource {
        match val.as_str() {
            "RHS" => OpSource::Rhs,
            "A" => OpSource::Register(Register::A),
            "H" => OpSource::Register(Register::H),
            "L" => OpSource::Register(Register::L),
            "W" => OpSource::Register(Register::TEMP_HIGH),
            "Z" => OpSource::Register(Register::TEMP_LOW),
            "C" => OpSource::Register(Register::C),
            "WZ" => OpSource::Register(Register::TEMP),
            "HL" => OpSource::Register(Register::HL),
            "PC" => OpSource::Register(Register::PC),
            "PC_H" => OpSource::Register(Register::PC_HIGH),
            "PC_L" => OpSource::Register(Register::PC_LOW),
            "SP" => OpSource::Register(Register::SP),
            "SP_H" => OpSource::Register(Register::PC_HIGH),
            "SP_L" => OpSource::Register(Register::PC_LOW),
            "RHS_H" => OpSource::RhsHigh,
            "RHS_L" => OpSource::RhsLow,
            "LHS" => OpSource::Lhs,
            "LHS_L" => OpSource::LhsLow,
            "LHS_H" => OpSource::LhsHigh,
            "OP" => OpSource::Register(Register::INSTR),
            "FF" => OpSource::FF,
            "MEM" => OpSource::Memory,
            "SE" => OpSource::SignExtendTmp,
            "CONST" => OpSource::ConstantPlaceholder,
            _ => panic!("Unexpected value: {}.", val),
        }
    }
}

#[derive(Clone, Copy)]
pub enum TempFlagControl {
    ReadWrite,
    Write { mask: u8 },
}

impl From<&str> for TempFlagControl {
    fn from(value: &str) -> TempFlagControl {
        use TempFlagControl::*;
        match value {
            "RW" => ReadWrite,
            _ if !value.is_empty() => {
                let mask = i32::from_str_radix(value, 2).unwrap();
                assert!(mask < (1 << 4));
                Write { mask: mask as u8 }
            }
            _ => panic!("Unexpected F value: {}.", value),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum IncrementOp {
    Inc,
    Dec,
    MovPc,
}

#[derive(Clone, Copy)]
pub struct Incrementer {
    pub op: IncrementOp,
    pub addr: OpSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HLAluOp {
    BinaryOp(alu::BinaryOp),
    Addc1,
    Mov,
    Daa,
    Cpl,
    Placeholder,
}

#[derive(Clone, Copy)]
pub enum InterruptControl {
    Enable,
    Disable,
}

#[derive(Clone, Copy)]
pub enum EndMode {
    No,
    Yes,
    OnNotCC,
}

#[derive(Clone, Copy)]
pub enum MemoryMode {
    Read,
    Write,
}

#[derive(Clone, Copy)]
pub struct HLMicroCode {
    pub memory_mode: Option<MemoryMode>,
    pub memory_address_source: Option<OpSource>,
    pub register_select: Option<OpSource>,

    pub incrementer: Option<Incrementer>,

    pub alu_tmp: Option<OpSource>,
    pub alu_op: Option<HLAluOp>,
    pub alu_rd: Option<OpSource>,
    pub alu_uses_tmp: bool,
    pub alu_tmp_flag: Option<TempFlagControl>,

    pub end_mode: EndMode,
    pub is_halt: bool,
    pub is_stop: bool,
    pub interrupt_control: Option<InterruptControl>,
}

impl HLMicroCode {
    fn replace_source(&mut self, source: OpSource, with: OpSource) {
        [
            &mut self.memory_address_source,
            &mut self.register_select,
            &mut self.alu_tmp,
            &mut self.alu_rd,
        ]
        .iter_mut()
        .for_each(|x| match x {
            Some(src) if *src == source => {
                x.replace(with);
            }
            _ => (),
        });
    }

    fn replace_binary_op(&mut self, with: alu::BinaryOp) {
        assert_eq!(self.alu_op, Some(HLAluOp::Placeholder));
        self.alu_op = Some(HLAluOp::BinaryOp(with));
    }
}

impl HLMicroCodeArray {
    pub fn replace_source(mut self, source: OpSource, with: OpSource) -> HLMicroCodeArray {
        self.0
            .iter_mut()
            .for_each(|x| x.replace_source(source, with));
        self
    }

    pub fn replace_lhs(self, with: Register) -> HLMicroCodeArray {
        if with.is_pair() {
            let (high, low) = with.decompose_pair();
            self.replace_source(OpSource::LhsLow, OpSource::Register(low))
                .replace_source(OpSource::LhsHigh, OpSource::Register(high))
                .replace_source(OpSource::Lhs, OpSource::Register(with))
        } else {
            self.replace_source(OpSource::Lhs, OpSource::Register(with))
        }
    }

    pub fn replace_rhs(self, with: Register) -> HLMicroCodeArray {
        self.replace_source(OpSource::Rhs, OpSource::Register(with))
    }

    pub fn replace_binary_op(self, with: alu::BinaryOp) -> HLMicroCodeArray {
        self.0.iter_mut().for_each(|x| x.replace_binary_op(with));
        self
    }
}

fn interpret_mcycle(rule: MCycleRule) -> HLMicroCode {
    let memory_mode = if let Some(rw) = rule.rw {
        if rw == "R" {
            Some(MemoryMode::Read)
        } else if rw == "W" {
            Some(MemoryMode::Write)
        } else {
            panic!("Unexpected R/W value: {}.", rw)
        }
    } else {
        None
    };
    let memory_address_source = rule.addr.map(OpSource::from);
    // Data
    let register_select = rule.data.map(OpSource::from);
    // TMP.
    let alu_tmp = rule.tmp.map(OpSource::from);
    // F.
    let alu_tmp_flag = match rule.tmpf {
        Some(val) => Some(TempFlagControl::from(val.as_str())),
        None => None,
    };
    // INC.
    let incrementer = match rule.inc {
        Some(val) => Some(match val.as_str() {
            "Y" => Incrementer {
                op: IncrementOp::Inc,
                addr: OpSource::Register(Register::PC),
            },
            "INC" => Incrementer {
                op: IncrementOp::Inc,
                addr: OpSource::from(rule.addr_2.unwrap()),
            },
            "DEC" => Incrementer {
                op: IncrementOp::Dec,
                addr: OpSource::from(rule.addr_2.unwrap()),
            },
            "MOVPC" => Incrementer {
                op: IncrementOp::MovPc,
                addr: OpSource::from(rule.addr_2.unwrap()),
            },
            _ => panic!("Unexpected INC: {}.", val),
        }),
        None => None,
    };
    // OP.
    let alu_op = match rule.op {
        Some(val) => Some(match val.as_str() {
            "ADD" => HLAluOp::BinaryOp(alu::BinaryOp::Add),
            "ADDC" => HLAluOp::BinaryOp(alu::BinaryOp::Adc),
            "ADDC1" => HLAluOp::Addc1,
            "MOV" => HLAluOp::Mov,
            "SUB" => HLAluOp::BinaryOp(alu::BinaryOp::Sub),
            "DAA" => HLAluOp::Daa,
            "CPL" => HLAluOp::Cpl,
            "OP" => HLAluOp::Placeholder,
            _ => panic!("Unexpected ALU OP: {}.", val),
        }),
        None => None,
    };
    // RD.
    let alu_rd = rule.rd.map(OpSource::from);
    if let Some(OpSource::Register(reg)) = alu_rd {
        assert!(alu_op == Some(HLAluOp::Mov) || reg == Register::A || reg == Register::ALU_ACT,
            "Can only write to arbitrary registers when source does not cause data hazard. Op: {:?}. Rd: {:?}.",
            alu_op, reg);
    }
    if let Some(rs) = rule.rs.as_ref() {
        if rs != "TMP" {
            panic!("Useless column can only be tmp.");
        }
    }
    // END.
    let end = rule.end.unwrap_or_default();
    let end_mode = match end.as_str() {
        "YI" | "YD" | "Y" | "HLT" | "STP" => EndMode::Yes,
        "!cc" => EndMode::OnNotCC,
        "" => EndMode::No,
        _ => panic!("Unexpected END: {}.", end),
    };
    let is_halt = end == "HLT";
    let is_stop = end == "STP";
    let interrupt_control = match end.as_str() {
        "YI" => Some(InterruptControl::Enable),
        "YD" => Some(InterruptControl::Disable),
        _ => None,
    };
    HLMicroCode {
        memory_mode,
        memory_address_source,
        register_select,
        incrementer,
        alu_tmp,
        alu_op,
        alu_rd,
        alu_uses_tmp: rule.rs.is_some(),
        alu_tmp_flag,
        end_mode,
        is_halt,
        is_stop,
        interrupt_control,
    }
}
