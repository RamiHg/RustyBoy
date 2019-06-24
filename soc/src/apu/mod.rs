use num_traits::PrimInt;
use spin::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::mmu;
use registers::*;

mod channels;
mod device;
mod registers;
mod sound;

pub const TCYCLE_FREQ: i32 = 4_194_304;
pub const MCYCLE_FREQ: i32 = 1_048_576;

pub const SAMPLE_RATE: f32 = 44_100.0;

const SOUND_DOWNSAMPLE: i32 = 1;
pub const BASE_FREQ: i32 = TCYCLE_FREQ / SOUND_DOWNSAMPLE;

pub const LENGTH_COUNTER_PERIOD: i32 = TCYCLE_FREQ / 256 / SOUND_DOWNSAMPLE;
pub const ENVELOPE_PERIOD: i32 = TCYCLE_FREQ / 64 / SOUND_DOWNSAMPLE;
pub const SWEEP_PERIOD: i32 = TCYCLE_FREQ / 128 / SOUND_DOWNSAMPLE;

pub type SharedWaveTable = Arc<RwLock<u128>>;

pub struct Apu {
    #[allow(dead_code)]
    device: Option<device::Device>,
    sound_enable: SoundEnable,

    channel_state: channels::ChannelState,
}

impl Default for Apu {
    fn default() -> Self {
        let channel_state = channels::ChannelState::default();
        let maybe_device = device::Device::try_new(channel_state.clone());
        if let Err(err) = maybe_device {
            println!(
                "Audio device is not available. Audio will be disabled. Error: {}",
                err
            );
        }
        Apu {
            device: maybe_device.ok(),
            sound_enable: SoundEnable(0xF3),
            channel_state,
        }
    }
}

impl Apu {
    pub fn execute_mcycle(&mut self) {}
}

pub type Timer = std::iter::Rev<std::ops::Range<i32>>;
pub fn timer(count: i32) -> Timer {
    (0..count).rev()
}

#[derive(Clone, Debug)]
pub struct CountdownTimer {
    counter: i32,
    timer: std::iter::Cycle<Timer>,
}

impl Iterator for CountdownTimer {
    type Item = Option<i32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.counter > 0 {
            Some(if self.timer.next().unwrap() == 0 {
                self.counter -= 1;
                Some(self.counter)
            } else {
                None
            })
        } else {
            None
        }
    }
}

impl CountdownTimer {
    pub fn new(counter: i32, period: i32) -> CountdownTimer {
        CountdownTimer {
            counter,
            timer: timer(period).cycle(),
        }
    }
}
// TODO: Move to util..
pub fn low_bits(x: u64) -> u8 {
    (x & 0xFF) as u8
}

pub fn high_bits(x: u64) -> u32 {
    (x >> 8) as u32
}

fn set_byte<T: PrimInt>(reg: &mut T, i: i32, value: i32) {
    debug_assert_lt!(i as usize, std::mem::size_of::<T>());
    let i = i as usize;
    let mask = T::from(0xFF).unwrap() << (8 * i);
    *reg = (*reg & !mask) | (T::from(value).unwrap() << (8 * i));
}

fn atomic_set_byte(reg: &AtomicU64, i: i32, value: i32) {
    use std::sync::atomic::Ordering;
    let mut current = reg.load(Ordering::Acquire);
    loop {
        let mut new_reg = current;
        set_byte(&mut new_reg, i, value);
        match reg.compare_exchange_weak(current, new_reg, Ordering::Release, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => {
                current = x;
                eprintln!("There is actual contention with audio thread.")
            }
        }
    }
}

fn get_byte<T: PrimInt>(reg: T, i: i32) -> i32 {
    debug_assert_lt!(i as usize, std::mem::size_of::<T>());
    use num_traits::cast::NumCast;
    let i = i as usize;
    <i32 as NumCast>::from((reg >> (8 * i)) & T::from(0xFF).unwrap()).unwrap()
}

impl mmu::MemoryMapped for Apu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        use std::sync::atomic::Ordering::Acquire;
        let mmu::Address(_, raw) = address;
        match raw {
            0xFF10..=0xFF14 => Some(get_byte(
                self.channel_state.square_1_config.load(Acquire),
                raw - 0xFF10,
            )),
            0xFF16..=0xFF19 => Some(get_byte(
                self.channel_state.square_2_config.load(Acquire),
                raw - 0xFF15,
            )),
            0xFF1A..=0xFF1E => Some(get_byte(
                self.channel_state.wave_config.load(Acquire),
                raw - 0xFF1A,
            )),
            0xFF30..=0xFF3F => Some(get_byte(
                *self.channel_state.wave_table.read(),
                raw - 0xFF30,
            )),
            _ => None,
        }
    }

    #[allow(clippy::unit_arg)]
    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        match raw {
            0xFF10..=0xFF14 => Some(atomic_set_byte(
                &self.channel_state.square_1_config,
                raw - 0xFF10,
                value,
            )),
            0xFF16..=0xFF19 => Some(atomic_set_byte(
                &self.channel_state.square_2_config,
                raw - 0xFF15,
                value,
            )),
            0xFF1A..=0xFF1E => Some(atomic_set_byte(
                &self.channel_state.wave_config,
                raw - 0xFF1A,
                value,
            )),
            0xFF30..=0xFF3F => {
                // Lock for an EXTREMELY brief period of time so as to never block the audio thread.
                let mut wave_table = self.channel_state.wave_table.write();
                set_byte(&mut *wave_table, raw - 0xFF30, value);
                Some(())
            }
            0xFF25 => Some(self.sound_enable.0 = value),
            _ => None,
        }
    }
}
