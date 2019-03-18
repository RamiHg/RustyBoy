use core::ops::Fn;

use bitflags::bitflags;

bitflags! {
    pub struct Flags: i32 {
        const CARRY = 0b0001_0000;
        const HCARRY = 0b0010_0000;
        const SUB = 0b0100_0000;
        const ZERO = 0b1000_0000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
    Swap,
}

impl BinaryOp {
    pub fn execute(self, lhs: i32, rhs: i32, flags: Flags) -> (i32, Flags) {
        use BinaryOp::*;
        match self {
            Add => generic_8bit_math_op(lhs, rhs, 0, |x, y| x + y),
            Adc => generic_8bit_math_op(lhs, rhs, flags.intersects(Flags::CARRY).into(), |x, y| {
                x + y
            }),
            Sub => {
                let (result, mut flags) = generic_8bit_math_op(lhs, rhs, 0, |x, y| x - y);
                flags |= Flags::SUB;
                (result, flags)
            }
            Sbc => {
                let (result, mut flags) = generic_8bit_math_op(lhs, rhs, 1, |x, y| x - y);
                flags |= Flags::SUB;
                (result, flags)
            }
            Cp => {
                // CP is basically a subtract with the results being ignored.
                let (_, mut flags) = generic_8bit_math_op(lhs, rhs, 0, |x, y| x - y);
                flags |= Flags::SUB;
                (lhs, flags)
            }
            And => generic_8bit_logical_op(lhs, rhs, true, |x, y| x & y),
            Xor => generic_8bit_logical_op(lhs, rhs, false, |x, y| x ^ y),
            Or => generic_8bit_logical_op(lhs, rhs, false, |x, y| x | y),
        }
    }
}

impl UnaryOp {
    pub fn execute(&self, value: i32, flags: Flags) -> (i32, Flags) {
        use UnaryOp::*;
        match self {
            Inc => {
                // Reuse add functionality.
                let (result, mut flags) = BinaryOp::Add.execute(value, 1, flags);
                // Carry is not affected.
                flags.remove(Flags::CARRY);
                (result, flags)
            }
            Dec => {
                // Reuse add functionality.
                let (result, mut flags) = BinaryOp::Sub.execute(value, 1, flags);
                // Carry is not affected.
                flags.remove(Flags::CARRY);
                (result, flags)
            }
            Swap => {
                let swap = |x, _| (x << 4) | ((x & 0xF) >> 4);
                generic_8bit_logical_op(value, value, false, swap)
            }
        }
    }
}

/// A generic 8-bit binary op (e.g. add or subtract) that also computes flags.
fn generic_8bit_math_op<T>(lhs: i32, rhs: i32, prev_carry: i32, op: T) -> (i32, Flags)
where
    T: Fn(i32, i32) -> i32,
{
    // Compute the half-carry (or borrow).
    let half_carry = (op(op(lhs & 0xF, rhs & 0xF), prev_carry) & 0x10) != 0;
    let result_full = op(op(lhs, rhs), prev_carry);
    let carry = (result_full & 0x100) != 0;
    let result = result_full & 0xFF;
    let mut flags = Flags::empty();
    flags.set(Flags::CARRY, carry);
    flags.set(Flags::HCARRY, half_carry);
    flags.set(Flags::ZERO, result == 0);
    (result, flags)
}

/// Slightly simpler version of generic_8bit_math_op that doesn't care about carry.
fn generic_8bit_logical_op<T>(lhs: i32, rhs: i32, has_high_carry: bool, op: T) -> (i32, Flags)
where
    T: Fn(i32, i32) -> i32,
{
    let result = op(lhs, rhs) & 0xFF;
    let mut flags = Flags::empty();
    flags.set(Flags::HCARRY, has_high_carry);
    flags.set(Flags::ZERO, result == 0);
    (result, flags)
}

/*

// I have no idea how this works, so I just referenced an implementation in nesdev.com
// http://forums.nesdev.com/viewtopic.php?t=9088
pub fn daa(a_u8: u8, current_flags: &Flags) -> (u8, Flags) {
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
    let flags = Flags::new(
        current_flags.get_bit(FlagBits::Carry) as u32 | c as u32,
        0,
        current_flags.get_bit(FlagBits::Sub) as u32,
        a == 0,
    );

    (a as u8, flags)
}

pub fn cpl_u8(a: u8, current_flags: &Flags) -> (u8, Flags) {
    let result: u8 = !a;

    let flags = Flags::new(
        current_flags.get_bit(FlagBits::Carry) as u32,
        1,
        1,
        current_flags.has_bit(FlagBits::Zero),
    );

    (result, flags)
}

pub fn ccf_u8(current_flags: &Flags) -> (Flags) {
    let c = if current_flags.has_bit(FlagBits::Carry) {
        0
    } else {
        1
    };
    Flags::new(c, 0, 0, current_flags.has_bit(FlagBits::Zero))
}

pub fn rotate_left_high_to_carry_u8(a: u8, _: &Flags) -> (u8, Flags) {
    let c = a & 0x80;
    let result: u8 = (a << 1) | (c >> 7);
    let flags = Flags::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_left_through_carry_u8(a: u8, current_flags: &Flags) -> (u8, Flags) {
    let c = a & 0x80;
    let old_c = if current_flags.has_bit(FlagBits::Carry) {
        1
    } else {
        0
    }; // todo: refactor get_bit
    let result: u8 = (a << 1) | old_c;
    let flags = Flags::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_right_low_to_carry_u8(a: u8, _: &Flags) -> (u8, Flags) {
    let c = a & 0x1;
    let result: u8 = (a >> 1) | (c << 7);
    let flags = Flags::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn rotate_right_through_carry_u8(a: u8, current_flags: &Flags) -> (u8, Flags) {
    let c = a & 0x1;
    let old_c = if current_flags.has_bit(FlagBits::Carry) {
        0x80
    } else {
        0
    }; // todo: refactor get_bit
    let result: u8 = (a >> 1) | old_c;
    let flags = Flags::new(c as u32, 0, 0, result == 0);
    (result, flags)
}

pub fn shift_left_u8(a: u8, _: &Flags) -> (u8, Flags) {
    let c = a & 0x80;
    let result: u8 = a << 1;
    (result, Flags::new(c as u32, 0, 0, result == 0))
}

pub fn shift_right_preserve_high_u8(a: u8, _: &Flags) -> (u8, Flags) {
    let c = a & 0x1;
    let result: u8 = (a >> 1) | (a & 0x80);
    (result, Flags::new(c as u32, 0, 0, result == 0))
}

pub fn shift_right_u8(a: u8, _: &Flags) -> (u8, Flags) {
    let c = a & 0x1;
    let result = a >> 1;
    (result, Flags::new(c as u32, 0, 0, result == 0))
}

pub fn bit_test_u8(a: u8, bit: u8, current_flags: &Flags) -> (Flags) {
    let is_zero = (a & (1 << bit)) == 0;

    Flags::new(current_flags.get_bit(FlagBits::Carry) as u32, 1, 0, is_zero)
}
*/
