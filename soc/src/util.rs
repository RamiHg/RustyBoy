#[inline]
pub fn is_8bit(value: i32) -> bool {
    (value as u32) <= core::u8::MAX.into()
}
#[inline]
pub fn is_16bit(value: i32) -> bool {
    (value as u32) <= core::u16::MAX.into()
}

pub fn is_bit_set(value: i32, bit: i32) -> bool {
    (value & (1 << bit)) != 0
}

pub fn upper_5_bits(value: i32) -> i32 {
    (value & 0xF8) >> 3
}

pub fn reverse_16bits(mut value: i32) -> i32 {
    let mut result = 0;
    for _ in 0..16 {
        result >>= 1;
        result |= value & 0x8000;
        value <<= 1;
    }
    result
}
