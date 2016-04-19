pub enum FlagBits {
    CARRY = 1 << 4,
    H_CARRY = 1 << 5,
    SUB = 1 << 6,
    ZERO = 1 << 7
}

pub struct FlagRegister {
    value : u8,
}

impl FlagRegister {
    pub fn new(carry: u32, hcarry: u32, sub: u32, zero: u32) -> FlagRegister {
        let mut ret = FlagRegister{value: 0};

        // So inefficient. TODO 
        ret.set_bit(FlagBits::CARRY, carry);
        ret.set_bit(FlagBits::H_CARRY, hcarry);
        ret.set_bit(FlagBits::SUB, sub);
        ret.set_bit(FlagBits::ZERO, zero);

        ret
    }

    pub fn set_bit(&mut self, bit: FlagBits, val : u32) {
        if val != 0 {
            self.value |= bit as u8;
        }
        else {
            self.value &= !(bit as u8);
        }
    }

    pub fn get_bit(&self, bit: FlagBits) -> u8{
        self.value & (bit as u8)
    }   
}

fn get_add_hc(a: i32, b: i32) -> i32 {
    ((a & 0xf) + (b & 0xf)) & 0xF0
}

fn get_sub_hc(a: i32, b: i32) -> i32 {
    (((a & 0xF) - (b & 0xf)) as u32 & 0xFFFFFFF0) as i32
}

pub fn add_u16_i8(a: u16, b: i8) -> (u16, FlagRegister) {
    // Cast up both to i32s
    let a_i32 = i32::from(a);
    let b_i32 = i32::from(b);

    let hc = get_add_hc(a_i32, b_i32);
    let result = a_i32 + b_i32;
    let c = result & 0xFF0000;

    let z = FlagRegister::new(
        c as u32,
        hc as u32,
        0,
        0
    );

    (result as u16, z)
}

pub fn add_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    // Cast up both to i32s
    let a_i32 = a as i32;
    let b_i32 = b as i32;

    let hc = get_add_hc(a_i32, b_i32);
    let result_i32 = a_i32 + b_i32;
    let c = result_i32 & 0xF00;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(
        c as u32,
        hc as u32,
        0,
        result as u32
    );

    return (result, z);
}

pub fn adc_u8_u8(a: u8, b: u8, prev_c: u8) -> (u8, FlagRegister) {
    let a_i32 = a as i32;
    let carry = if prev_c != 0 { 1 } else { 0 };
    let b_i32 = b as i32 + carry;

    let hc = get_add_hc(a_i32, b_i32);
    let result_i32 = a_i32 + b_i32;
    let c = result_i32 & 0xF00;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(
        c as u32,
        hc as u32,
        0,
        result as u32
    );

    return (result, z);
}

pub fn sub_i8_i8(a: u8, b: u8) -> (u8, FlagRegister) {
    let a_i32 = (a as i8) as i32;
    let b_i32 = (b as i8) as i32;

    let hc = get_sub_hc(a_i32, b_i32);
    let result_i32 = a_i32 - b_i32;
    let c = result_i32 as u32 & 0xFFFFFF00_u32;
    let result = (result_i32 & 0xFF) as u8;

    let z = FlagRegister::new(
        c as u32,
        hc as u32,
        1,
        result as u32
    );

    return (result, z);
}

pub fn sbc_i8_i8(a: u8, b: u8, prev_c: u8) -> (u8, FlagRegister) {
    let a_i32 = (a as i8) as i32;
    let carry = if prev_c != 0 { 1 } else { 0 };
    let b_i32 = (b as i8) as i32 + carry;

    return sub_i8_i8((a_i32 as i8) as u8, (b_i32 as i8) as u8);
}

pub fn and_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a & b;
    
    let z = FlagRegister::new(
        0, 1, 0, result as u32
    );
    
    return (result, z);
}

pub fn or_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a | b;
    let z = FlagRegister::new(
        0, 0, 0, result as u32
    );
    return (result, z);
}

pub fn xor_u8_u8(a: u8, b: u8) -> (u8, FlagRegister) {
    let result = a ^ b;
    let z = FlagRegister::new(
        0, 0, 0, result as u32
    );
    return (result, z);
}