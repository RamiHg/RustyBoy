use crate::io_registers;

use bitfield::bitfield;
use bitflags::bitflags;

bitfield! {
    pub struct SquareConfig(u32);
    impl Debug;
    u16;
    pub length, _: 5, 0;
    pub duty, set_duty: 7, 6;
    pub envelop_sweep, _: 10, 8;
    pub envelop_up, _: 11, 11;
    pub volume, _: 15, 12;
    pub freq, _: 26, 16;
    pub is_timed, _: 30;
    pub triggered, set_triggered: 31;
}

bitfield! {
    pub struct SoundEnable(i32);
    no default BitRange;
    u8;
    pub sound1_on, _: 0;
    pub sound2, _: 1;
    pub sound3_on, _: 2;
    pub sound4_on, _: 3;
    pub sound_on, _: 7;
}

impl_bitfield_helpful_traits!(SquareConfig);

impl_bitfield_helpful_traits!(SoundEnable);
impl_bitfield_bitrange!(SoundEnable);
