pub fn is_8bit(value: i32) -> bool {
    (value as u8) <= core::u8::MAX.into()
}

pub fn is_16bit(value: i32) -> bool {
    (value as u16) <= core::u16::MAX.into()
}
