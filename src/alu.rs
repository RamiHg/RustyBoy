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

    pub fn set_bit(&mut self, bit : FlagBits, val : u32) {
        if val != 0 {
            self.value |= bit as u8;
        }
        else {
            self.value &= !(bit as u8);
        }
    }
}

pub fn add_u16_i8(a: u16, b: i8) -> (u16, FlagRegister) {
    // Cast up both to i32s
    let a_i32 = i32::from(a);
    let b_i32 = i32::from(b);

    let hc = ((a_i32 & 0xf) + (b_i32 & 0xf)) & 0x10;
    let result = a_i32 + b_i32;
    let c = result & 0x10000;

    let z = FlagRegister::new(
        c as u32,
        hc as u32,
        0,
        0
    );

    (result as u16, z)
}
