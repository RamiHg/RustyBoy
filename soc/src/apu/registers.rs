use crate::io_registers;

use bitfield::bitfield;
use bitflags::bitflags;
use num_derive::FromPrimitive;

#[derive(Debug, FromPrimitive)]
pub enum EnvelopeMode {
    Attenuate,
    Amplify,
}
from_u8!(EnvelopeMode);

#[derive(Debug, FromPrimitive)]
pub enum SweepMode {
    Increase,
    Decrease,
}
from_u8!(SweepMode);

bitfield! {
    pub struct SquareConfig(u64);
    impl Debug;
    u8;
    pub sweep_shift, _: 2, 0;
    pub into SweepMode, sweep_mode, _: 3, 3;
    pub sweep_time, _: 6, 4;
    pub length, _: 13, 8;
    pub duty, _: 15, 14;
    pub envelope_counter, _: 18, 16;
    pub into EnvelopeMode, envelope_mode, _: 19, 19;
    pub volume, _: 23, 20;
    pub u16, freq, _: 34, 24;
    pub is_timed, _: 38;
    pub triggered, set_triggered: 39;
}

impl SquareConfig {
    pub fn from_low_high(low: u8, high: u32) -> SquareConfig {
        SquareConfig(low as u64 | (high as u64) << 8)
    }
}

bitfield! {
    pub struct WaveConfig(u64);
    impl Debug;
    u8;
    pub enabled, _: 7;
    pub length, _: 15, 8;
    pub volume, _: 22, 21;
    pub u16, freq, _: 34, 24;
    pub is_timed, _: 38;
    pub triggered, set_triggered: 39;
}

impl WaveConfig {
    pub fn from_low_high(low: u8, high: u32) -> WaveConfig {
        WaveConfig(low as u64 | (high as u64) << 8)
    }
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
impl_bitfield_helpful_traits!(WaveConfig);

impl_bitfield_helpful_traits!(SoundEnable);
impl_bitfield_bitrange!(SoundEnable);
