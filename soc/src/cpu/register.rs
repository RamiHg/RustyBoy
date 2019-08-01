use std::convert::From;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

// use super::alu;
use crate::util::{is_16bit, is_8bit};

/// Abstracts the various registers of the Z80.
/// The 16 and 8-bit registers are: AF, BC, DE, HL, SP, PC for 12 8-bit registers in total.
/// They are stored in the order B,C,D,E,H,L,A,F,TEMP,SP,PC.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize), serde(transparent))]
pub struct File([i32; Register::NumRegisters as usize]);

/// The logical list of possible registers and register combination.
#[derive(FromPrimitive, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
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
    INSTR,
    ACT,
    ALU_TMP,
    ALU_TMP_F,
    // TEMP_LOW/HIGH are "temporary" registers that store intermediate microcode results.
    TEMP_LOW,
    TEMP_HIGH,
    SP_LOW,
    SP_HIGH,
    PC_LOW,
    PC_HIGH,
    NumRegisters,
    // "Virtual" registers.
    SP,
    PC,
    BC,
    DE,
    HL,
    AF,
    TEMP,
    INVALID = -1,
}

impl Default for Register {
    fn default() -> Self {
        Register::INVALID
    }
}

impl Default for File {
    fn default() -> File {
        File { 0: [0; Register::NumRegisters as usize] }
    }
}

impl File {
    pub fn get(&self, any: Register) -> i32 {
        let combine_any = |a, b| (self.get(a) << 8) | self.get(b);
        use Register::*;
        match any {
            F => self.0[F as usize] & 0xF0,
            _ if (any as usize) <= (PC_HIGH as usize) => self.0[any as usize],
            SP => combine_any(SP_HIGH, SP_LOW),
            PC => combine_any(PC_HIGH, PC_LOW),
            BC => combine_any(B, C),
            DE => combine_any(D, E),
            HL => combine_any(H, L),
            AF => combine_any(A, F),
            TEMP => combine_any(TEMP_HIGH, TEMP_LOW),
            INVALID => panic!("Attempting to get invalid register."),
            _ => panic!("Non-exhaustive pattern."),
        }
    }

    pub fn set(&mut self, any: Register, value: i32) {
        use Register::*;
        debug_assert!(any.is_pair() && is_16bit(value) || is_8bit(value));
        let value_u8 = value & 0xFF;
        let value_high = (value & 0xFF00) >> 8;
        match any {
            _ if (any as usize) <= (PC_HIGH as usize) => self.0[any as usize] = value_u8 as i32,
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
            AF => {
                self.0[A as usize] = value_high;
                self.0[F as usize] = value_u8;
            }
            SP => {
                self.0[SP_HIGH as usize] = value_high;
                self.0[SP_LOW as usize] = value_u8;
            }
            PC => {
                self.0[PC_HIGH as usize] = value_high;
                self.0[PC_LOW as usize] = value_u8;
            }
            TEMP => {
                self.0[TEMP_HIGH as usize] = value_high;
                self.0[TEMP_LOW as usize] = value_u8;
            }
            INVALID => panic!("Attempting to set invalid register."),
            _ => panic!("Non-exhaustive pattern."),
        }
    }
}

impl core::fmt::Debug for File {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(f)?;
        for i in 0..=(Register::ALU_TMP as i32) {
            writeln!(
                f,
                "{:>10}: {: <2X?}",
                format!("{:?}", Register::from_i32(i).unwrap()),
                self.0[i as usize]
            )?;
        }
        writeln!(f, "{:>10}: {: <2X?}", "PC", self.get(Register::PC))?;
        writeln!(f, "{:>10}: {: <2X?}", "WZ", self.get(Register::TEMP))?;
        Ok(())
    }
}
