use std::convert::From;

use crate::util::{is_16bit, is_8bit};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Abstracts the various registers of the Z80.
/// The 16 and 8-bit registers are: AF, BC, DE, HL, SP, PC for 12 8-bit registers in total.
/// They are stored in the order B,C,D,E,H,L,A,F,SP,PC,TEMP
pub struct File([i32; 14]);

/// The logical list of possible registers and register combination.
#[derive(FromPrimitive, Clone, Copy, PartialEq, Debug)]
#[allow(non_camel_case_types)]
pub enum Register {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    F,
    // TEMP_LOW/HIGH are "temporary" registers that store intermediate microcode results.
    TEMP_LOW,
    TEMP_HIGH,
    SP,
    PC,
    // "Virtual" registers. I.e. a register pair.
    BC,
    DE,
    HL,
    TEMP,
}

impl Register {
    pub fn is_pair(self) -> bool {
        use Register::*;
        match self {
            BC | DE | HL | SP | PC | TEMP => true,
            _ => false,
        }
    }

    pub fn is_single(self) -> bool {
        !self.is_pair()
    }

    fn is_16bit(self) -> bool {
        self.is_pair()
    }
}

/// 8-bit register table. Note that this maps to the instruction opcodes.
#[derive(FromPrimitive, PartialEq, Clone, Copy)]
pub enum SingleTable {
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
    A,
}

impl From<SingleTable> for Register {
    fn from(single: SingleTable) -> Register {
        use SingleTable::*;
        match single {
            B | C | D | E | H | L => Register::from_usize(single as usize).unwrap(),
            A => Register::A,
            HL => Register::HL,
        }
    }
}

impl File {
    pub fn new(values: [i32; 14]) -> File {
        File(values)
    }

    pub fn get(&self, any: Register) -> i32 {
        let combine_any = |a, b| self.combine(a as usize, b as usize);
        use Register::*;
        match any {
            _ if (any as usize) <= (TEMP_HIGH as usize) => self.0[any as usize],
            SP => self.combine(SP as usize + 1, SP as usize),
            PC => self.combine(PC as usize + 1, PC as usize),
            BC => combine_any(Register::B, Register::C),
            DE => combine_any(Register::D, Register::E),
            HL => combine_any(Register::H, Register::L),
            TEMP => combine_any(Register::TEMP_HIGH, Register::TEMP_LOW),
            _ => panic!("Non-exhaustive pattern."),
        }
    }

    pub fn set(&mut self, any: Register, value: i32) {
        use Register::*;
        assert!(any.is_16bit() && is_16bit(value) || is_8bit(value));
        let value_u8 = value & 0xFF;
        let value_high = (value & 0xFF00) >> 8;
        match any {
            _ if (any as usize) <= (TEMP_HIGH as usize) => self.0[any as usize] = value_u8 as i32,
            BC => {
                self.0[B as usize] = value_high;
                self.0[C as usize] = value_u8;
            }
            DE => {
                self.0[D as usize] = value_high;
                self.0[E as usize] = value_u8;
            }
            HL => {
                self.0[H as usize] = value_high;
                self.0[L as usize] = value_u8;
            }
            SP => {
                self.0[SP as usize + 1] = value_high;
                self.0[SP as usize] = value_u8;
            }
            PC => {
                self.0[PC as usize + 1] = value_high;
                self.0[PC as usize] = value_u8;
            }
            TEMP => {
                self.0[TEMP_HIGH as usize] = value_high;
                self.0[TEMP_LOW as usize] = value_u8;
            }
            _ => panic!("Non-exhaustive pattern."),
        }
    }

    fn combine(&self, i: usize, j: usize) -> i32 {
        ((self.0[i] as i32) << 8) | (self.0[j] as i32)
    }
}
