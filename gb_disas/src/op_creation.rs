use super::{Arg, Command, Op};

impl Op {
    pub fn new(name: &'static str) -> Op {
        Op {
            command: Command { name },
            lhs: None,
            rhs: None,
            byte_size: 1,
        }
    }

    pub fn new_sized(name: &'static str, size: u8) -> Op {
        Op {
            byte_size: size,
            ..Op::new(name)
        }
    }

    pub fn new_alu_op(value: u8, arg: Arg) -> Op {
        let command = Command {
            name: match value {
                0 => "ADD",
                1 => "ADC",
                2 => "SUB",
                3 => "SBC",
                4 => "AND",
                5 => "XOR",
                6 => "OR",
                7 => "CP",
                _ => panic!(),
            },
        };
        let (lhs, rhs) = if let 0...3 = value {
            (Some(Arg::from_reg("A")), Some(arg))
        } else {
            (Some(arg), None)
        };
        Op {
            command,
            lhs,
            rhs,
            byte_size: 1,
        }
    }

    pub fn with_size(self, byte_size: u8) -> Op { Op { byte_size, ..self } }

    pub fn with_lhs(self, arg: Arg) -> Op {
        Op {
            lhs: Some(arg),
            ..self
        }
    }

    pub fn with_rhs(self, arg: Arg) -> Op {
        Op {
            rhs: Some(arg),
            ..self
        }
    }
}

impl Arg {
    pub fn from_i8(value: u8) -> Arg { Arg::Signed8bit(value as i8) }
    pub fn from_u8(value: u8) -> Arg { Arg::Unsigned8bit(value) }
    pub fn from_reg(name: &'static str) -> Arg { Arg::Register(name) }
    pub fn as_indirect(self) -> Arg { Arg::IndirectRef(Box::new(self)) }
    pub fn as_ffplus(self) -> Arg { Arg::FFPlus(Box::new(self)).as_indirect() }
    pub fn as_spplus(self) -> Arg { Arg::SPPlus(Box::new(self)) }

    pub fn from_cond(value: u8) -> Arg {
        Arg::Condition(match value {
            0 => "NZ",
            1 => "Z",
            2 => "NC",
            3 => "C",
            _ => panic!(),
        })
    }

    pub fn from_sp_table(value: u8) -> Arg {
        Arg::Register(match value {
            0 => "BC",
            1 => "DE",
            2 => "HL",
            3 => "SP",
            _ => panic!(),
        })
    }

    pub fn from_af_table(value: u8) -> Arg {
        Arg::Register(match value {
            0 => "BC",
            1 => "DE",
            2 => "HL",
            3 => "AF",
            _ => panic!(),
        })
    }

    pub fn from_reg_table(value: u8) -> Arg {
        let reg = Arg::Register(match value {
            0 => "B",
            1 => "C",
            2 => "D",
            3 => "E",
            4 => "H",
            5 => "L",
            6 => "HL",
            7 => "A",
            _ => panic!(),
        });
        if value == 6 { reg.as_indirect() } else { reg }
    }
}
