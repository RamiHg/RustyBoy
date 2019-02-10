use core::convert::From;
use core::ops::Fn;

use bitfield::bitfield;

use super::decoder;
use crate::util::is_16bit;

bitfield! {
    pub struct FlagRegister(u32);
    pub carry, set_carry: 4;
    pub half_carry, set_half_carry: 5;
    pub subtract, set_subtract: 6;
    pub zero, set_zero: 7;
}

impl FlagRegister {
    pub fn new(carry: bool, half_carry: bool, subtract: bool, zero: bool) -> FlagRegister {
        let bits = ((0b0001_0000) * (carry as i32))
            | ((0b0010_0000) * (half_carry as i32))
            | ((0b0100_0000) * (subtract as i32))
            | ((0b1000_0000) * (zero as i32));
        FlagRegister(bits as u32)
    }
}

pub enum BinaryOp {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

pub enum UnaryOp {
    Inc,
    Dec,
}

impl From<decoder::AluOpTable> for BinaryOp {
    fn from(op: decoder::AluOpTable) -> BinaryOp {
        use decoder::AluOpTable;
        use BinaryOp::*;
        match op {
            AluOpTable::AddA => Add,
            AluOpTable::AdcA => Adc,
            AluOpTable::SubA => Sub,
            AluOpTable::SbcA => Sbc,
            AluOpTable::AndA => And,
            AluOpTable::XorA => Xor,
            AluOpTable::OrA => Or,
            AluOpTable::CpA => Cp,
        }
    }
}

impl BinaryOp {
    pub fn execute(&self, lhs: i32, rhs: i32, flags: &FlagRegister) -> (i32, FlagRegister) {
        use BinaryOp::*;
        match self {
            Add => generic_8bit_math_op(lhs, rhs, 0, |x, y| x + y),
            Adc => generic_8bit_math_op(lhs, rhs, flags.carry().into(), |x, y| x + y),
            Sub => {
                let (result, mut flags) = generic_8bit_math_op(lhs, rhs, 0, |x, y| x - y);
                flags.set_subtract(true);
                (result, flags)
            }
            Sbc => {
                let (result, mut flags) = generic_8bit_math_op(lhs, rhs, 1, |x, y| x - y);
                flags.set_subtract(true);
                (result, flags)
            }
            Cp => {
                // CP is basically a subtract with the results being ignored.
                let (_, mut flags) = generic_8bit_math_op(lhs, rhs, 0, |x, y| x - y);
                flags.set_subtract(true);
                (lhs, flags)
            }
            And => generic_8bit_logical_op(lhs, rhs, true, |x, y| x & y),
            Xor => generic_8bit_logical_op(lhs, rhs, false, |x, y| x ^ y),
            Or => generic_8bit_logical_op(lhs, rhs, false, |x, y| x | y),
        }
    }
}

impl UnaryOp {
    pub fn execute(&self, value: i32, flags: &FlagRegister) -> (i32, FlagRegister) {
        use UnaryOp::*;
        match self {
            Inc => {
                // Reuse add functionality.
                let (result, mut flags) = BinaryOp::Add.execute(value, 1, flags);
                // Carry is not affected.
                flags.set_carry(false);
                (result, flags)
            }
            Dec => {
                // Reuse add functionality.
                let (result, mut flags) = BinaryOp::Sub.execute(value, 1, flags);
                // Carry is not affected.
                flags.set_carry(false);
                (result, flags)
            }
        }
    }
}

/// A generic 8-bit binary op (e.g. add or subtract) that also computes flags.
fn generic_8bit_math_op<T>(lhs: i32, rhs: i32, prev_carry: i32, op: T) -> (i32, FlagRegister)
where
    T: Fn(i32, i32) -> i32,
{
    // Compute the half-carry (or borrow).
    let half_carry = (op(op(lhs & 0xF, rhs & 0xF), prev_carry) & 0x10) != 0;
    let result_full = op(op(lhs, rhs), prev_carry);
    let carry = (result_full & 0x100) != 0;
    let result = result_full & 0xFF;
    (
        result,
        FlagRegister::new(carry, half_carry, false, result == 0),
    )
}

/// Slightly simpler version of generic_8bit_math_op that doesn't care about carry.
fn generic_8bit_logical_op<T>(
    lhs: i32,
    rhs: i32,
    has_high_carry: bool,
    op: T,
) -> (i32, FlagRegister)
where
    T: Fn(i32, i32) -> i32,
{
    let result = op(lhs, rhs) & 0xFF;
    (
        result,
        FlagRegister::new(false, has_high_carry, false, result == 0),
    )
}

pub fn inc_u16(a: i32) -> i32 {
    assert!(is_16bit(a));
    i32::from((a as u16).wrapping_add(1))
}

pub fn dec_u16(a: i32) -> i32 {
    assert!(is_16bit(a));
    i32::from((a as u16).wrapping_sub(1))
}

/*

// Having a useless first parameter to fit with helper functions in cpu
pub fn inc_u8_u8(_unused: u8, a: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let (result, mut flags) = add_u8_u8(a, 1);
    // The carry flag is not affected
    flags.set_bit(
        FlagBits::Carry,
        current_flags.get_bit(FlagBits::Carry) as u32,
    );
    (result, flags)
}

pub fn dec_u8_u8(_unused: u8, a: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let (result, mut flags) = sub_u8_u8(a, 1);
    // The carry flag is not affected
    flags.set_bit(
        FlagBits::Carry,
        current_flags.get_bit(FlagBits::Carry) as u32,
    );
    (result, flags)
}

pub fn add_u16_u16(a: u16, b: u16, current_flags: &FlagRegister) -> (u16, FlagRegister) {
    // Cast up
    let a32 = a as u32;
    let b32 = b as u32;

    // H is carry from bit 11
    let h = ((a32 & 0xFFF) + (b32 & 0xFFF)) & 0xF000;
    // C is carry from bit 15
    let result = a32 + b32;
    let c = result & 0xF0000;

    let z = FlagRegister::new(c, h, 0, current_flags.has_bit(FlagBits::Zero));

    (result as u16, z)
}

// Used for LDHL sp+n and ADD SP, n
pub fn add_u16_i8(a: u16, b: u8) -> (u16, FlagRegister) {
    // Cast up to i32s
    let a32 = a as i32;
    let b32 = (b as i8) as i32;

    let result = (a32 + b32) as u16;
    let result32 = a32 + b32;
    let hc = (a32 ^ b32 ^ result32) & 0x10;
    let c = (a32 ^ b32 ^ result32) & 0x100;

    let z = FlagRegister::new(c as u32, hc as u32, 0, false);

    (result, z)
}

// Misc funcs
pub fn swap_u8(a: u8) -> (u8, FlagRegister) {
    let result = ((a & 0xF0) >> 4) | ((a & 0xF) << 4);
    let z = FlagRegister::new(0, 0, 0, result == 0);
    (result, z)
}

// I have no idea how this works, so I just referenced an implementation in nesdev.com
// http://forums.nesdev.com/viewtopic.php?t=9088
pub fn daa(a_u8: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let mut a = a_u8 as i32;

    if !current_flags.has_bit(FlagBits::Sub) {
        if current_flags.has_bit(FlagBits::HCarry) || (a & 0xF) > 9 {
            a += 0x06;
        }

        if current_flags.has_bit(FlagBits::Carry) || a > 0x9F {
            a += 0x60;
        }
    } else {
        if current_flags.has_bit(FlagBits::HCarry) {
            a = (a - 6) & 0xFF;
        }

        if current_flags.has_bit(FlagBits::Carry) {
            a -= 0x60;
        }
    }

    let c = a & 0x100;
    a &= 0xFF;

    // Flags are a bit tricky
    let flags = FlagRegister::new(
        current_flags.get_bit(FlagBits::Carry) as u32 | c as u32,
        0,
        current_flags.get_bit(FlagBits::Sub) as u32,
        a == 0,
    );

    (a as u8, flags)
}

pub fn cpl_u8(a: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let result: u8 = !a;

    let flags = FlagRegister::new(
        current_flags.get_bit(FlagBits::Carry) as u32,
        1,
        1,
        current_flags.has_bit(FlagBits::Zero),
    );

    (result, flags)
}

pub fn ccf_u8(current_flags: &FlagRegister) -> (FlagRegister) {
    let c = if current_flags.has_bit(FlagBits::Carry) {
        0
    } else {
        1
    };
    FlagRegister::new(c, 0, 0, current_flags.has_bit(FlagBits::Zero))
}

pub fn rotate_left_high_to_carry_u8(a: u8, _: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x80;
    let result: u8 = (a << 1) | (c >> 7);
    let flags = FlagRegister::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_left_through_carry_u8(a: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x80;
    let old_c = if current_flags.has_bit(FlagBits::Carry) {
        1
    } else {
        0
    }; // todo: refactor get_bit
    let result: u8 = (a << 1) | old_c;
    let flags = FlagRegister::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_right_low_to_carry_u8(a: u8, _: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x1;
    let result: u8 = (a >> 1) | (c << 7);
    let flags = FlagRegister::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_right_through_carry_u8(a: u8, current_flags: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x1;
    let old_c = if current_flags.has_bit(FlagBits::Carry) {
        0x80
    } else {
        0
    }; // todo: refactor get_bit
    let result: u8 = (a >> 1) | old_c;
    let flags = FlagRegister::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn shift_left_u8(a: u8, _: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x80;
    let result: u8 = a << 1;
    (result, FlagRegister::new(c as u32, 0, 0, result == 0))
}

pub fn shift_right_preserve_high_u8(a: u8, _: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x1;
    let result: u8 = (a >> 1) | (a & 0x80);
    (result, FlagRegister::new(c as u32, 0, 0, result == 0))
}

pub fn shift_right_u8(a: u8, _: &FlagRegister) -> (u8, FlagRegister) {
    let c = a & 0x1;
    let result = a >> 1;
    (result, FlagRegister::new(c as u32, 0, 0, result == 0))
}

pub fn bit_test_u8(a: u8, bit: u8, current_flags: &FlagRegister) -> (FlagRegister) {
    let is_zero = (a & (1 << bit)) == 0;

    FlagRegister::new(current_flags.get_bit(FlagBits::Carry) as u32, 1, 0, is_zero)
}
*/
