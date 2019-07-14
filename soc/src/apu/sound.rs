use arrayvec::ArrayVec;
use bitflags::bitflags;
use std::iter::Cycle;
use std::iter::FromIterator as _;

use crate::apu::components::{Envelope, Noise, Sweep};
use crate::apu::registers::{EnvelopeMode, NoiseConfig, SquareConfig, WaveConfig};
use crate::util::{timer, CountdownTimer, Timer};

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

pub trait Wave {
    fn update_from_reg(&mut self, config: WaveConfig);
    fn update_to_reg(&self, config: &mut WaveConfig);
    fn sample(&mut self) -> Option<f32>;
}

bitflags! {
    pub struct ComponentCycle: i32 {
        const ENVELOPE  = 0b0010;
        const LENGTH    = 0b0100;
        const SWEEP     = 0b1000;
    }
}

pub trait Sound {
    fn update_from_reg(&mut self, reg: u64);
    fn update_to_reg(&self, reg: &mut u64);
    fn sample(&mut self, cycles: ComponentCycle) -> Option<f32>;
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

    fn sample(&mut self, cycles: ComponentCycle) -> Option<f32> {
        if self.is_done {
            return None;
        }
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
            .set_volume(self.envelope.clock(self.config.volume().into()) as u8);

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
        Some(sample as f32 / 15.0)
    }
}

pub struct SoundSampler {
    waveform: ArrayVec<[u8; 32]>,
    waveform_index: i32,
    volume: f32,
    frequency: i32,
    envelope: Option<Envelope>,
    sweep: Option<Sweep>,
    noise: Option<Noise>,
    stop_on_done: bool,
    // Timers
    freq_timer: Cycle<CountdownTimer>,
    length_timer: Cycle<Timer>,
    is_done: bool,
}

impl SoundSampler {
    pub fn from_wave_config(config: WaveConfig, packed_wave_table: u128) -> SoundSampler {
        // move to test?
        // debug_assert_eq!(
        //     packed_wave_to_array(0xefcdab89_67452301_u128),
        //     [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        // );
        let volume = if !config.enabled() || config.volume() == 0 {
            0.0
        } else {
            1.0 / config.volume() as f32
        };
        SoundSampler::from_settings(
            &packed_wave_to_array(packed_wave_table),
            volume,
            config.freq().into(),
            2,
            None,
            None,
            256 - config.length() as i32,
            config.is_timed(),
        )
    }

    pub fn from_noise_config(config: NoiseConfig) -> SoundSampler {
        let mut sampler = SoundSampler::from_settings(
            &[],
            config.volume().into(),
            0,
            1,
            Some((config.envelope_mode(), config.envelope_counter().into())),
            None,
            64 - config.length() as i32,
            config.is_timed(),
        );
        let mantissa = 2 * (config.divisor_code() as i32 + 1);
        debug_assert_lt!(config.shift(), 0xE);
        sampler.noise = Some(Noise {
            buzz: config.width_mode(),
            lfsr: 0x7FFF,
            timer: timer((mantissa << i32::from(config.shift())) * super::NOISE_PERIOD).cycle(),
        });
        sampler
    }

    #[allow(clippy::too_many_arguments)]
    fn from_settings(
        waveform: &[u8],
        volume: f32,
        frequency: i32,
        freq_multiplier: i32,
        envelope_setting: Option<(EnvelopeMode, i32)>,
        sweep: Option<Sweep>,
        length: i32,
        stop_on_done: bool,
    ) -> SoundSampler {
        let envelope = envelope_setting.map(|(mode, time)| Envelope::new(mode, time));
        SoundSampler {
            waveform: ArrayVec::from_iter(waveform.iter().cloned()),
            waveform_index: 0,
            volume,
            frequency,
            envelope,
            sweep,
            noise: None,
            stop_on_done,
            freq_timer: SoundSampler::make_freq_timer(frequency, freq_multiplier),
            length_timer: timer(length * super::LENGTH_COUNTER_PERIOD).cycle(),
            is_done: false,
        }
    }

    fn sample_waveform(&mut self) -> f32 {
        let sample = self.waveform[self.waveform_index as usize] as f32 * self.volume;
        if self.freq_timer.next().unwrap().is_some() {
            self.waveform_index = (self.waveform_index + 1) % self.waveform.len() as i32;
        }
        // Update the duration.
        if let Some(0) = self.length_timer.next() {
            if self.stop_on_done {
                self.is_done = true;
            }
        }
        sample
    }

    fn make_freq_timer(freq: i32, multiplier: i32) -> std::iter::Cycle<CountdownTimer> {
        CountdownTimer::new(8, (2048 - freq) * multiplier).cycle()
    }
}

impl Iterator for SoundSampler {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.is_done {
            return None;
        }
        // TODO: Refactor so that we don't have this ugly logic.
        let sample = if let Some(noise) = &mut self.noise {
            let sample = noise.sample();
            noise.clock();
            sample
        } else {
            let sample = self.waveform[self.waveform_index as usize];
            if self.freq_timer.next().unwrap().is_some() {
                self.waveform_index = (self.waveform_index + 1) % self.waveform.len() as i32;
            }
            sample
        } as f32
            * self.volume;
        // Update the envelope (volume). Disabled on wave.
        if let Some(envelope) = &mut self.envelope {
            self.volume = envelope.clock(self.volume as i32) as f32;
        }
        // Update the duration.
        if let Some(0) = self.length_timer.next() {
            if self.stop_on_done {
                self.is_done = true;
            }
        }
        Some(sample / 15.0)
    }
}
