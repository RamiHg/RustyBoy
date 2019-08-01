use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;

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

impl Register {
    pub fn is_pair(self) -> bool {
        use Register::*;
        match self {
            BC | DE | HL | SP | PC | TEMP | AF => true,
            _ => false,
        }
    }

    pub fn is_single(self) -> bool {
        !self.is_pair()
    }

    pub fn from_single_table(single_value: i32) -> Register {
        Register::from(SingleTable::from_i32(single_value).unwrap())
    }

    pub fn from_sp_pair_table(pair_value: i32) -> Register {
        use Register::*;
        match pair_value {
            0 => BC,
            1 => DE,
            2 => HL,
            3 => SP,
            _ => panic!("Unexpected pair_value: {}.", pair_value),
        }
    }

    /// Decomposes a 16-bit register into its high and low bytes.
    pub fn decompose_pair(self) -> (Register, Register) {
        debug_assert!(self.is_pair());
        use Register::*;
        match self {
            BC => (B, C),
            DE => (D, E),
            HL => (H, L),
            AF => (A, F),
            SP => (SP_HIGH, SP_LOW),
            PC => (PC_HIGH, PC_LOW),
            TEMP => (TEMP_HIGH, TEMP_LOW),
            _ => panic!("Unexpected match result."),
        }
    }

    #[allow(dead_code)]
    pub fn overlaps(self, rhs: Register) -> bool {
        if self.is_pair() ^ rhs.is_pair() {
            self == rhs
        } else if self.is_pair() {
            let (high, low) = self.decompose_pair();
            high == rhs || low == rhs
        } else {
            rhs.overlaps(self)
        }
    }
}

/// 8-bit register table. Note that this maps to the instruction opcodes.
#[derive(FromPrimitive, PartialEq, Clone, Copy)]
pub(crate) enum SingleTable {
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

/// 16-bit SP-based register table. Note that this maps to instruction opcodes.
/// TODO: Move these tables to decoder.
#[derive(FromPrimitive)]
pub(crate) enum SPPairTable {
    BC,
    DE,
    HL,
    SP,
}
