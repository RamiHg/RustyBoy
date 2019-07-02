use arrayvec::ArrayVec;
use sample::Signal as _;
use std::iter::Cycle;
use std::iter::FromIterator as _;

use crate::apu::registers::{EnvelopeMode, NoiseConfig, SquareConfig, SweepMode, WaveConfig};
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

struct Envelope {
    pub mode: EnvelopeMode,
    pub timer: Cycle<Timer>,
}

struct Sweep {
    pub mode: SweepMode,
    pub shift: i32,
    pub timer: Cycle<Timer>,
}

struct Noise {
    pub buzz: bool,
    pub lfsr: u16,
    pub timer: Cycle<Timer>,
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

type Interpolator = sample::interpolate::Floor<sample::frame::Mono<f32>>;
pub type SoundSamplerSignal =
    sample::interpolate::Converter<sample::signal::FromIterator<SoundSampler>, Interpolator>;

//pub type SoundSamplerSignal = sample::signal::FromIterator<SoundSampler>;

impl SoundSampler {
    pub fn from_square_config(config: SquareConfig) -> SoundSampler {
        let sweep_setting = if config.sweep_time() > 0 {
            Some((
                config.sweep_mode(),
                config.sweep_shift().into(),
                config.sweep_time().into(),
            ))
        } else {
            None
        };
        //dbg!(config);
        SoundSampler::from_settings(
            &WAVE_DUTIES[config.duty() as usize],
            config.volume().into(),
            config.freq().into(),
            4,
            Some((config.envelope_mode(), config.envelope_counter().into())),
            sweep_setting,
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
        sweep_setting: Option<(SweepMode, i32, i32)>,
        length: i32,
        stop_on_done: bool,
    ) -> SoundSampler {
        let envelope = envelope_setting.map(|(mode, time)| Envelope::new(mode, time));
        let sweep = sweep_setting.map(|(mode, shift, time)| Sweep {
            mode,
            shift,
            timer: timer(time * super::SWEEP_PERIOD).cycle(),
        });
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

    pub fn into_signal(self) -> SoundSamplerSignal {
        let mut source = sample::signal::from_iter(self);
        let interp = Interpolator::from_source(&mut source);
        let dest = if super::use_lowpass() {
            super::BASE_FREQ as f64
        } else {
            super::device::DEVICE_RATE as f64
        };
        source.from_hz_to_hz(interp, super::BASE_FREQ as f64, dest)
    }

    fn make_freq_timer(freq: i32, multiplier: i32) -> std::iter::Cycle<CountdownTimer> {
        CountdownTimer::new(8, (2048 - freq) * multiplier).cycle()
    }
}

impl Envelope {
    pub fn new(mode: EnvelopeMode, time: i32) -> Envelope {
        Envelope {
            mode,
            timer: timer(time * super::ENVELOPE_PERIOD).cycle(),
        }
    }

    pub fn clock(&mut self, volume: f32) -> f32 {
        if let Some(0) = self.timer.next() {
            match self.mode {
                EnvelopeMode::Attenuate => (volume - 1.0).max(0.),
                EnvelopeMode::Amplify => (volume + 1.).min(15.),
            }
        } else {
            volume
        }
    }
}

impl Noise {
    pub fn sample(&self) -> u8 {
        (!self.lfsr & 1) as u8
    }

    pub fn clock(&mut self) {
        if let Some(0) = self.timer.next() {
            let mut lfsr = self.lfsr;
            // XOR the low two bits.
            let new_bit = (lfsr & 1) ^ ((lfsr >> 1) & 1);
            // Shift right and stick the result in the new high bit.
            lfsr = (lfsr >> 1) | (new_bit << 14);
            debug_assert_ge!(lfsr.leading_zeros(), 1);
            if self.buzz {
                // Also stick in 7th bit.
                lfsr = (lfsr & !(1 << 6)) | (new_bit << 6);
            }
            self.lfsr = lfsr;
        }
    }
}

impl Iterator for SoundSampler {
    type Item = sample::frame::Mono<f32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None; //Some([0.0]);
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
        // Update the sweep (frequency). Disabled on wave.
        if let Some(Sweep { mode, shift, timer }) = &mut self.sweep {
            if let Some(0) = timer.next() {
                let change = self.frequency >> *shift;
                self.frequency += match mode {
                    SweepMode::Increase => change,
                    SweepMode::Decrease => -change,
                };
                if self.frequency < 0 || self.frequency > 2047 {
                    self.is_done = true;
                } else {
                    // Hard-coding multiplier to 4 since it is disabled on wave.
                    self.freq_timer = SoundSampler::make_freq_timer(self.frequency, 4);
                }
            }
        }
        // Update the duration.
        if let Some(0) = self.length_timer.next() {
            if self.stop_on_done {
                self.is_done = true;
            }
        }
        Some([sample / 15.0])
    }
}
