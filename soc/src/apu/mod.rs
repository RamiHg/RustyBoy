use num_traits::PrimInt;
use spin::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::mmu;

mod channels;
mod components;
mod device;
mod registers;
mod sound;

pub const TCYCLE_FREQ: i32 = 4_194_304;

pub const LENGTH_COUNTER_PERIOD: i32 = TCYCLE_FREQ / 256;
pub const ENVELOPE_PERIOD: i32 = TCYCLE_FREQ / 64;
pub const SWEEP_PERIOD: i32 = TCYCLE_FREQ / 128;
pub const NOISE_PERIOD: i32 = 8;

pub type SharedWaveTable = Arc<RwLock<u128>>;

pub struct Apu {
    #[allow(dead_code)]
    device: Option<device::Device>,

    audio_regs: channels::SharedAudioRegs,
}

impl Default for Apu {
    fn default() -> Self {
        let audio_regs = channels::SharedAudioRegs::default();
        let maybe_device = device::Device::try_new(audio_regs.clone());
        if let Err(err) = &maybe_device {
            eprintln!("Could not initialize audio. Audio will be disabled. Error: {}", err);
        }
        Apu { device: maybe_device.ok(), audio_regs }
    }
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
            // Volume control (NR50)
            0xFF24 => Some(i32::from(self.audio_regs.volume_control.load(Acquire))),
            // Channel R/L mix (NR51)
            0xFF25 => Some(i32::from(self.audio_regs.sound_mix.load(Acquire))),
            // Sound status (NR52).
            0xFF26 => {
                //panic!("Ruh-oh!");
                Some(i32::from(self.audio_regs.sound_status.load(Acquire) | 0x70))
            }
            // Square 1
            0xFF10..=0xFF14 => {
                Some(get_byte(self.audio_regs.square_1_config.load(Acquire), raw - 0xFF10))
            }
            // Square 2
            0xFF16..=0xFF19 => {
                Some(get_byte(self.audio_regs.square_2_config.load(Acquire), raw - 0xFF15))
            }
            // Wave
            0xFF1A..=0xFF1E => {
                Some(get_byte(self.audio_regs.wave_config.load(Acquire), raw - 0xFF1A))
            }
            // Noise
            0xFF20..=0xFF23 => {
                Some(get_byte(self.audio_regs.noise_config.load(Acquire), raw - 0xFF1F))
            }
            // Wave table
            0xFF30..=0xFF3F => Some(get_byte(*self.audio_regs.wave_table.read(), raw - 0xFF30)),
            _ => None,
        }
    }

    #[allow(clippy::unit_arg)]
    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        use crate::util::AtomicInt;
        use std::sync::atomic::Ordering;
        let mmu::Address(_, raw) = address;
        match raw {
            // Volume control
            0xFF24 => Some(self.audio_regs.volume_control.store(value as u8, Ordering::Release)),
            // Channel R/ L mix
            0xFF25 => Some({
                self.audio_regs.sound_mix.store(value as u8, Ordering::Release);
            }),
            // Sound status (NR52)
            0xFF26 => Some({
                self.audio_regs.sound_status.weak_update_with(Ordering::Release, |x: u8| {
                    (x & !0x80) | (value as u8 & 0x80)
                });
            }),
            // Square 1
            0xFF10..=0xFF14 => {
                atomic_set_byte(&self.audio_regs.square_1_config, raw - 0xFF10, value);
                Some(())
            }
            // Square 2
            0xFF16..=0xFF19 => Some({
                atomic_set_byte(&self.audio_regs.square_2_config, raw - 0xFF15, value);
            }),
            // Wave
            0xFF1A..=0xFF1E => Some({
                atomic_set_byte(&self.audio_regs.wave_config, raw - 0xFF1A, value);
            }),
            // Noise
            0xFF20..=0xFF23 => Some({
                atomic_set_byte(&self.audio_regs.noise_config, raw - 0xFF1F, value);
            }),
            // Wave table
            0xFF30..=0xFF3F => {
                // Lock for an EXTREMELY brief period of time so as to never block the audio thread.
                let mut wave_table = self.audio_regs.wave_table.write();
                set_byte(&mut *wave_table, raw - 0xFF30, value);
                Some(())
            }

            _ => None,
        }
    }
}
