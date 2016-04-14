#![feature(plugin)]
#![plugin(bitfield)]

bitfield!{FlagRegister,
    zero_pad: 4,
    carry: 1,
    half_carry: 1,
    sub: 1,
    zero: 1
    }

pub fn add_u16_i8(a: u16, b: i8) -> (u16, FlagRegister) {
    // Cast up both to i32s
    let a_i32 = i32::from(a);
    let b_i32 = i32::from(b);

    let hc = ((a_i32 & 0xf) + (b_32 & 0xf)) & 0x10;
    let result = a_i32 + b_i32;
    let c = result & 0x10000;

    let z: FlagRegister::new(
        0,
        c,
        hc,
        0,
        0
    );

    (result, z)
}
