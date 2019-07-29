use bitflags::bitflags;

use crate::util;

bitflags! {
    pub struct Flags: i32 {
        const CARRY = 0b0001_0000;
        const HCARRY = 0b0010_0000;
        const SUB = 0b0100_0000;
        const ZERO = 0b1000_0000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Invalid,
    // Binary ops.
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
    // Shifts and rotates.
    Rlc,
    Rl,
    Rrc,
    Rr,
    Sla,
    Sra,
    Srl,
    // Unary ops.
    Mov,
    Cpl,
    Scf,
    Ccf,
    Swap,
    Daa,
    // Bit ops.
    Bit,
    Res,
    Set,
}

impl Default for Op {
    fn default() -> Self {
        Op::Invalid
    }
}

impl Op {
    pub fn execute(self, lhs: i32, rhs: i32, flags: Flags) -> (i32, Flags) {
        debug_assert!(util::is_8bit(lhs));

        let has_carry: i32 = flags.intersects(Flags::CARRY).into();

        let adder = |x, y| x + y;
        let subber = |x, y| x - y;
        let rlcer = |x| (x << 1) | (x >> 7);
        let rrcer = |x| (x >> 1) | ((x & 1) << 7);
        let rler = |x| (x << 1) | has_carry;
        let rrer = |x| (x >> 1) | (has_carry << 7);
        let sler = |x| x << 1;
        let sraer = |x| (x >> 1) | (x & 0x80);
        let srler = |x| x >> 1;
        let swapper = |x| (x << 4) | ((x >> 4) & 0xF);

        let last_bitter = |x| x & 0x80;
        let first_bitter = |x| x & 0x01;
        let zeroer = |_| 0;

        use Op::*;
        match self {
            Invalid => panic!("Attempting to execute invalid ALU op."),
            Mov => (lhs, flags),
            Add => generic_8bit_math_op(lhs, rhs, 0, adder),
            Adc => generic_8bit_math_op(lhs, rhs, flags.intersects(Flags::CARRY).into(), adder),
            Sub => {
                let (result, mut flags) = generic_8bit_math_op(lhs, rhs, 0, subber);
                flags |= Flags::SUB;
                (result, flags)
            }
            Sbc => {
                let (result, mut flags) =
                    generic_8bit_math_op(lhs, rhs, flags.intersects(Flags::CARRY).into(), subber);
                flags |= Flags::SUB;
                (result, flags)
            }
            Cp => {
                // CP is basically a subtract with the results being ignored.
                let (_, mut flags) = generic_8bit_math_op(lhs, rhs, 0, subber);
                flags |= Flags::SUB;
                (lhs, flags)
            }
            And => generic_8bit_logical_op(lhs, rhs, true, |x, y| x & y),
            Xor => generic_8bit_logical_op(lhs, rhs, false, |x, y| x ^ y),
            Or => generic_8bit_logical_op(lhs, rhs, false, |x, y| x | y),
            Rlc => generic_unary_op(lhs, rlcer, last_bitter),
            Rl => generic_unary_op(lhs, rler, last_bitter),
            Rrc => generic_unary_op(lhs, rrcer, first_bitter),
            Rr => generic_unary_op(lhs, rrer, first_bitter),
            Sla => generic_unary_op(lhs, sler, last_bitter),
            Sra => generic_unary_op(lhs, sraer, first_bitter),
            Srl => generic_unary_op(lhs, srler, first_bitter),
            Cpl => {
                let (result, _) = generic_unary_op(lhs, |x| !x, zeroer);
                (result, flags | (Flags::SUB | Flags::HCARRY))
            }
            Scf => (lhs, (flags & Flags::ZERO) | Flags::CARRY),
            Ccf => (lhs, (flags & Flags::ZERO) | ((flags ^ Flags::CARRY) & Flags::CARRY)),
            Daa => daa(lhs, flags),
            Swap => {
                let (result, flags) = generic_unary_op(lhs, swapper, zeroer);
                (result, flags & Flags::ZERO)
            }
            Bit => {
                let mut flags = (flags & Flags::CARRY) | Flags::HCARRY;
                flags.set(Flags::ZERO, (lhs & (1 << rhs)) == 0);
                (lhs, flags)
            }
            Res => (lhs & !(1 << rhs), flags),
            Set => (lhs | (1 << rhs), flags),
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

fn generic_unary_op<T, A>(lhs: i32, op: T, carry_op: A) -> (i32, Flags)
where
    T: Fn(i32) -> i32,
    A: Fn(i32) -> i32,
{
    let result = op(lhs) & 0xFF;
    let new_carry = carry_op(lhs);
    let mut flags = Flags::empty();
    flags.set(Flags::CARRY, new_carry != 0);
    flags.set(Flags::ZERO, result == 0);
    (result, flags)
}

// I have no idea how this works, so I just referenced an implementation in nesdev.com
// http://forums.nesdev.com/viewtopic.php?t=9088
fn daa(mut a: i32, flags: Flags) -> (i32, Flags) {
    if !flags.intersects(Flags::SUB) {
        if flags.intersects(Flags::HCARRY) || (a & 0xF) > 9 {
            a += 0x06;
        }
        if flags.intersects(Flags::CARRY) || a > 0x9F {
            a += 0x60;
        }
    } else {
        if flags.intersects(Flags::HCARRY) {
            a = (a - 6) & 0xFF;
        }
        if flags.intersects(Flags::CARRY) {
            a -= 0x60;
        }
    }

    let c = a & 0x100;
    a &= 0xFF;

    // Flags are a bit tricky
    let mut new_flags = flags & Flags::SUB;
    new_flags.set(Flags::CARRY, flags.intersects(Flags::CARRY) || c != 0);
    new_flags.set(Flags::ZERO, a == 0);
    (a, new_flags)
}
