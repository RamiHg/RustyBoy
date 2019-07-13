use arrayvec::ArrayVec;
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

pub trait Square {
    /// Updates internal state from any change that might have happened in the register.
    fn update_from_reg(&mut self, config: SquareConfig);
    fn update_to_reg(&self, config: &mut SquareConfig);
    fn sample(&mut self) -> Option<f32>;
}

impl Iterator for Square {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        self.sample()
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

impl Square for SoundSampler {
    fn update_from_reg(&mut self, config: SquareConfig) {
        let new_freq = i32::from(config.freq());
        if new_freq != self.frequency {
            self.frequency = new_freq;
            self.freq_timer = SoundSampler::make_freq_timer(self.frequency, 4);
        }
        // TODO: Make this more robust - actually store remaining length period.
        if config.length() == 0 {
            self.is_done = true;
        }
    }

    fn update_to_reg(&self, config: &mut SquareConfig) {
        config.set_freq(self.frequency as u16);
    }

    fn sample(&mut self) -> Option<f32> {
        if self.is_done {
            return None;
        }
        let sample = self.sample_waveform();
        // Update the envelope (volume). Disabled on wave.
        if let Some(envelope) = &mut self.envelope {
            self.volume = envelope.clock(self.volume);
        }
        // Update the sweep (frequency). Disabled on wave.
        if let Some(sweep) = &mut self.sweep {
            self.frequency = sweep.update(self.frequency);
            if self.frequency >= 0 && self.frequency <= 2047 {
                self.freq_timer = SoundSampler::make_freq_timer(self.frequency, 4);
            }
        }
        Some(sample / 15.0)
    }
}

impl SoundSampler {
    pub fn from_square_config(config: SquareConfig) -> SoundSampler {
        SoundSampler::from_settings(
            &WAVE_DUTIES[config.duty() as usize],
            config.volume().into(),
            config.freq().into(),
            4,
            Some((config.envelope_mode(), config.envelope_counter().into())),
            Sweep::from_config(config),
            64 - config.length() as i32,
            config.is_timed(),
        )
    }

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
            debug_assert_le!(sample, 1);
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
            self.volume = envelope.clock(self.volume);
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
