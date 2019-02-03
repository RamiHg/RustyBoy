#![allow(clippy::verbose_bit_mask)]

#[derive(Copy, Clone)]
pub enum FlagBits {
    Carry = 1 << 4,
    HCarry = 1 << 5,
    Sub = 1 << 6,
    Zero = 1 << 7,
}

pub struct FlagRegister {
    pub value: u8,
}

impl FlagRegister {
    pub fn new(carry: u32, hcarry: u32, sub: u32, zero: bool) -> FlagRegister {
        let mut ret = FlagRegister { value: 0 };

        // So inefficient. TODO
        ret.set_bit(FlagBits::Carry, carry);
        ret.set_bit(FlagBits::HCarry, hcarry);
        ret.set_bit(FlagBits::Sub, sub);
        ret.set_bit(FlagBits::Zero, if zero { 1 } else { 0 });

        ret
    }

    pub fn set_bit(&mut self, bit: FlagBits, val: u32) {
        if val != 0 {
            self.value |= bit as u8;
        } else {
            self.value &= !(bit as u8);
        }
    }

    pub fn get_bit(&self, bit: FlagBits) -> u8 {
        self.value & (bit as u8)
    }

    pub fn has_bit(&self, bit: FlagBits) -> bool {
        self.get_bit(bit) != 0
    }
}

fn get_add_hc(a: i32, b: i32) -> i32 {
    ((a & 0xf) + (b & 0xf)) & 0xF0
}

fn get_sub_hc(a: i32, b: i32) -> i32 {
    (((a & 0xF) - (b & 0xf)) as u32 & 0xFFFFFFF0) as i32
}

pub fn add_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    // Cast up both to i32s
    let a_i32 = a as i32;
    let b_i32 = b as i32;

    let hc = get_add_hc(a_i32, b_i32);
    let result_i32 = a_i32 + b_i32;
    let c = result_i32 & 0xF00;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(c as u32, hc as u32, 0, result == 0);

    (result, z)
}

pub fn adc_u8_u8(a: u8, b: u8, prev_c: u8) -> (u8, FlagRegister) {
    let a_i32 = a as i32;
    let carry = if prev_c != 0 { 1 } else { 0 };
    let b_i32 = b as i32;

    let hc = ((a_i32 & 0xF) + (b_i32 & 0xF) + carry) & 0xF0;
    let result_i32 = a_i32 + b_i32 + carry;
    let c = result_i32 & 0xF00;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(c as u32, hc as u32, 0, result == 0);

    (result, z)
}

pub fn sub_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let a_i32 = a as i32;
    let b_i32 = b as i32;

    let hc = get_sub_hc(a_i32, b_i32);
    let result_i32 = a_i32 - b_i32;
    let c = result_i32 & !0xFF;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(c as u32, hc as u32, 1, result == 0);

    (result, z)
}

pub fn sbc_i8_i8(a: u8, b: u8, prev_c: u8) -> (u8, FlagRegister) {
    let a_i32 = a as i32;
    let carry = if prev_c != 0 { 1 } else { 0 };
    let b_i32 = b as i32;

    let hc = ((a_i32 & 0xF) - (b_i32 & 0xF) - carry) & !0xF;
    let result = a_i32 - b_i32 - carry;
    let c = result & !0xFF;

    let z = FlagRegister::new(c as u32, hc as u32, 1, (result & 0xFF) == 0);

    ((result & 0xFF) as u8, z)
}

pub fn and_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a & b;

    let z = FlagRegister::new(0, 1, 0, result == 0);

    (result, z)
}

pub fn or_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a | b;
    let z = FlagRegister::new(0, 0, 0, result == 0);
    (result, z)
}

pub fn xor_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a ^ b;
    let z = FlagRegister::new(0, 0, 0, result == 0);
    (result, z)
}

pub fn cp_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    // This is basically A - n with the result thrown away
    let (_, flags) = sub_u8_u8(a, b);
    (a, flags)
}

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
