use std::convert::From;

use crate::util::{is_16bit, is_8bit};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Abstracts the various registers of the Z80.
/// The 16 and 8-bit registers are: AF, BC, DE, HL, SP, PC for 12 8-bit registers in total.
/// They are stored in the order B,C,D,E,H,L,A,F,SP,PC
pub struct File([u8; 12]);

/// The logical list of possible registers and register combination.
#[derive(FromPrimitive, Clone, Copy)]
pub enum Register {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    F,
    BC,
    DE,
    HL,
    SP,
    PC,
}

impl Register {
    pub fn is_pair(self) -> bool {
        use Register::*;
        match self {
            BC | DE | HL => true,
            _ => false,
        }
    }

    fn is_16bit(self) -> bool {
        use Register::*;
        match self {
            BC | DE | HL | SP | PC => true,
            _ => false,
        }
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
    pub fn new(values: [u8; 12]) -> File {
        File(values)
    }

    pub fn get(&self, any: Register) -> i32 {
        let combine_any = |a, b| self.combine(a as usize, b as usize);
        use Register::*;
        match any {
            B => self.0[0] as i32,
            C => self.0[1] as i32,
            D => self.0[2] as i32,
            E => self.0[3] as i32,
            H => self.0[4] as i32,
            L => self.0[5] as i32,
            A => self.0[6] as i32,
            F => self.0[7] as i32,
            BC => combine_any(Register::B, Register::C),
            DE => combine_any(Register::D, Register::E),
            HL => combine_any(Register::H, Register::L),
            SP => self.combine(9, 8),
            PC => self.combine(11, 10),
        }
    }

    pub fn set(&mut self, any: Register, value: i32) {
        use Register::*;
        assert!(any.is_16bit() && is_16bit(value) || is_8bit(value));
        let value_u8 = value as u8;
        let value_high = ((value as u16) >> 8) as u8;
        match any {
            B => self.0[B as usize] = value_u8,
            C => self.0[C as usize] = value_u8,
            D => self.0[D as usize] = value_u8,
            E => self.0[E as usize] = value_u8,
            H => self.0[H as usize] = value_u8,
            L => self.0[L as usize] = value_u8,
            A => self.0[A as usize] = value_u8,
            F => self.0[F as usize] = value_u8,
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
                self.0[8] = value_u8;
                self.0[9] = value_high;
            }
            PC => {
                self.0[10] = value_u8;
                self.0[11] = value_high;
            }
        }
    }

    fn combine(&self, i: usize, j: usize) -> i32 {
        ((self.0[i] as i32) << 8) | (self.0[j] as i32)
    }
}
