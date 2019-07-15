use arrayvec::ArrayVec;
use bitflags::bitflags;
use std::iter::Cycle;

use crate::apu::components::{Envelope, Sweep};
use crate::apu::registers::{NoiseConfig, SquareConfig, WaveConfig};
use crate::util::{timer, Timer};

const WAVE_DUTIES: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

fn packed_wave_to_array(packed: u128) -> [u8; 32] {
    let mut array = [0; 32];
    for i in 0..32 {
        let byte = i / 2;
        let nibble = (i + 1) % 2;
        let value = (super::get_byte(packed, byte) >> (nibble * 4)) & 0xF;
        array[i as usize] = value as u8;
    }
    array
}

bitflags! {
    pub struct ComponentCycle: i32 {
        const LENGTH    = 0b0100;
    }
}

pub trait Sound {
    fn update_from_reg(&mut self, reg: u64);
    fn update_to_reg(&self, reg: &mut u64);
    fn sample(&mut self, cycles: ComponentCycle) -> f32;
    fn is_done(&self) -> bool;
}

pub struct Square {
    config: SquareConfig,
    waveform_index: u8,
    envelope: Envelope,
    sweep: Option<Sweep>,
    freq_timer: Timer,
    is_done: bool,
}

impl Square {
    pub fn new(config: SquareConfig) -> Square {
        Square {
            config,
            waveform_index: 0,
            envelope: Envelope::new(config.envelope_mode(), config.envelope_counter().into()),
            sweep: Sweep::from_config(config),
            freq_timer: Square::make_freq_timer(config.freq()),
            is_done: false,
        }
    }

    fn make_freq_timer(freq: u16) -> Timer {
        timer((2048 - i32::from(freq)) * 4)
    }
}

impl Sound for Square {
    fn update_from_reg(&mut self, reg: u64) {
        let reset_timer = self.config.freq() != SquareConfig(reg).freq();
        self.config.0 = reg;
        if reset_timer {
            self.freq_timer = Square::make_freq_timer(self.config.freq());
        }
    }

    fn update_to_reg(&self, reg: &mut u64) {
        *reg = self.config.0;
    }

    fn is_done(&self) -> bool {
        self.is_done
    }

    fn sample(&mut self, cycles: ComponentCycle) -> f32 {
        debug_assert!(!self.is_done);
        let sample = WAVE_DUTIES[usize::from(self.config.duty())][usize::from(self.waveform_index)]
            as f32
            * self.config.volume() as f32;
        // Update waveform.
        if self.freq_timer.next().unwrap() == 0 {
            self.waveform_index = (self.waveform_index + 1) % 8;
            self.freq_timer = Square::make_freq_timer(self.config.freq());
        }
        // Update the envelope (volume).
        self.config
            .set_volume(self.envelope.clock(self.config.volume()));

        // Update the sweep (frequency).
        if let Some(sweep) = &mut self.sweep {
            if let Some(freq) = sweep.update(self.config.freq()) {
                self.config.set_freq(freq);
                if freq <= 2047 {
                    self.freq_timer = Square::make_freq_timer(freq);
                } else {
                    self.is_done = true;
                }
            }
        }
        // Length.
        if cycles.contains(ComponentCycle::LENGTH) {
            let new_len = self.config.length() + 1;
            if new_len >= 64 && self.config.is_timed() {
                self.is_done = true;
            }
            self.config.set_length(new_len);
        }
        sample as f32 / 15.0
    }
}

pub struct Wave {
    config: WaveConfig,
    waveform: ArrayVec<[u8; 32]>,
    waveform_index: u8,
    freq_timer: Timer,
    is_done: bool,
}

impl Wave {
    pub fn new(config: WaveConfig, packed_wave_table: u128) -> Wave {
        Wave {
            config,
            waveform: ArrayVec::from(packed_wave_to_array(packed_wave_table)),
            waveform_index: 0,
            freq_timer: Wave::make_freq_timer(config.freq()),
            is_done: false,
        }
    }

    fn make_freq_timer(freq: u16) -> Timer {
        timer((2048 - i32::from(freq)) * 2)
    }
}

impl Sound for Wave {
    fn update_from_reg(&mut self, reg: u64) {
        let reset_timer = self.config.freq() != WaveConfig(reg).freq();
        self.config.0 = reg;
        if reset_timer {
            self.freq_timer = Wave::make_freq_timer(self.config.freq());
        }
    }

    fn update_to_reg(&self, reg: &mut u64) {
        *reg = self.config.0;
    }

    fn is_done(&self) -> bool {
        self.is_done
    }

    fn sample(&mut self, cycles: ComponentCycle) -> f32 {
        debug_assert!(!self.is_done);
        let volume = if !self.config.enabled() || self.config.volume() == 0 {
            0.0
        } else {
            1.0 / self.config.volume() as f32
        };
        let sample = self.waveform[usize::from(self.waveform_index)] as f32 * volume;
        // Update waveform.
        if self.freq_timer.next().unwrap() == 0 {
            self.waveform_index = (self.waveform_index + 1) % 32;
            self.freq_timer = Wave::make_freq_timer(self.config.freq());
        }
        // Length.
        if cycles.contains(ComponentCycle::LENGTH) {
            let new_len = self.config.length() + 1;
            if new_len >= 256 && self.config.is_timed() {
                self.is_done = true;
            }
            self.config.set_length(new_len);
        }
        sample / 15.0
    }
}

pub struct Noise {
    config: NoiseConfig,
    envelope: Envelope,
    lfsr: u16,
    timer: Cycle<Timer>,
    is_done: bool,
}

impl Noise {
    pub fn new(config: NoiseConfig) -> Noise {
        Noise {
            config,
            envelope: Envelope::new(config.envelope_mode(), config.envelope_counter().into()),
            lfsr: 0x7FFF,
            timer: Noise::make_freq_timer(config.divisor_code(), config.shift()),
            is_done: false,
        }
    }

    fn make_freq_timer(divisor_code: u8, shift: u8) -> Cycle<Timer> {
        let mantissa = 2 * (divisor_code as i32 + 1);
        timer((mantissa << i32::from(shift)) * super::NOISE_PERIOD).cycle()
    }

    fn clock(&mut self) {
        if let Some(0) = self.timer.next() {
            let mut lfsr = self.lfsr;
            // XOR the low two bits.
            let new_bit = (lfsr & 1) ^ ((lfsr >> 1) & 1);
            // Shift right and stick the result in the new high bit.
            lfsr = (lfsr >> 1) | (new_bit << 14);
            debug_assert_ge!(lfsr.leading_zeros(), 1);
            if self.config.width_mode() {
                // Also stick in 7th bit.
                lfsr = (lfsr & !(1 << 6)) | (new_bit << 6);
            }
            self.lfsr = lfsr;
        }
    }
}

impl Sound for Noise {
    fn update_from_reg(&mut self, reg: u64) {
        let reg = NoiseConfig(reg);
        let reset_timer =
            self.config.divisor_code() != reg.divisor_code() || self.config.shift() != reg.shift();
        self.config.0 = reg.0;
        if reset_timer {
            self.timer = Noise::make_freq_timer(self.config.divisor_code(), self.config.shift());
        }
    }

    fn update_to_reg(&self, reg: &mut u64) {
        *reg = self.config.0;
    }

    fn is_done(&self) -> bool {
        self.is_done
    }

    fn sample(&mut self, cycles: ComponentCycle) -> f32 {
        debug_assert!(!self.is_done);
        let sample = (!self.lfsr & 1) as u8 * self.config.volume();
        self.clock();
        self.config
            .set_volume(self.envelope.clock(self.config.volume()));
        if cycles.contains(ComponentCycle::LENGTH) {
            let new_len = self.config.length() + 1;
            if new_len >= 64 && self.config.is_timed() {
                self.is_done = true;
            }
            self.config.set_length(new_len);
        }
        sample as f32 / 15.0
    }
}
