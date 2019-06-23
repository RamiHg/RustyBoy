use lazy_static::lazy_static;
use num_traits::FromPrimitive as _;
use sample::Signal as _;
use std::sync::Arc;

use super::{timer, CountdownTimer, Timer};
use crate::apu::registers::{EnvelopeMode, SquareConfig, SweepMode};

lazy_static! {
    static ref DUTIES: [Arc<[f32]>; 4] = [
        Arc::new([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
        Arc::new([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
        Arc::new([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
        Arc::new([0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]),
    ];
}

pub struct SoundSampler {
    waveform: Arc<[f32]>,
    waveform_index: i32,
    volume: i32,
    freq_setting: i32,
    envelope_mode: EnvelopeMode,
    stop_on_done: bool,
    sweep_shift: i32,
    sweep_mode: SweepMode,
    frequency: i32,
    // Timers
    freq_timer: std::iter::Cycle<CountdownTimer>,
    sweep_timer: std::iter::Cycle<Timer>,
    envelope_timer: CountdownTimer,
    length_timer: std::iter::Cycle<Timer>,
    is_done: bool,
}

pub type SoundSamplerSignal = sample::interpolate::Converter<
    sample::signal::FromIterator<SoundSampler>,
    sample::interpolate::Floor<sample::frame::Mono<f32>>,
>;

impl SoundSampler {
    pub fn from_square_config(config: SquareConfig) -> SoundSampler {
        SoundSampler {
            waveform: Arc::clone(&DUTIES[config.duty() as usize]),
            waveform_index: 0,
            volume: config.volume().into(),
            freq_setting: config.freq().into(),
            envelope_mode: config.envelope_mode(),
            stop_on_done: config.is_timed(),
            sweep_shift: config.sweep_shift() as i32,
            sweep_mode: config.sweep_mode(),
            frequency: config.freq() as i32,
            freq_timer: SoundSampler::make_freq_timer(config.freq() as i32),
            sweep_timer: timer(config.sweep_time() as i32 * super::SWEEP_PERIOD).cycle(),
            envelope_timer: CountdownTimer::new(
                config.envelope_counter().into(),
                super::ENVELOPE_PERIOD,
            ),
            length_timer: timer((64 - config.length() as i32) * super::LENGTH_COUNTER_PERIOD)
                .cycle(),
            is_done: false,
        }
    }

    pub fn to_signal(self) -> SoundSamplerSignal {
        let mut source = sample::signal::from_iter(self);
        let interp = sample::interpolate::Floor::from_source(&mut source);
        source.from_hz_to_hz(interp, super::BASE_FREQ as f64, super::SAMPLE_RATE.into())
    }

    fn make_freq_timer(freq: i32) -> std::iter::Cycle<CountdownTimer> {
        CountdownTimer::new(8, (2048 - freq) * 4).cycle()
    }
}

impl Iterator for SoundSampler {
    type Item = sample::frame::Mono<f32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        let sample = self.waveform[self.waveform_index as usize] * self.volume as f32 / 15.0;
        // Update the waveform.
        if self.freq_timer.next().unwrap().is_some() {
            self.waveform_index = (self.waveform_index + 1) % 8;
        }
        // Update the volume.
        if let Some(Some(_)) = self.envelope_timer.next() {
            self.volume = match self.envelope_mode {
                EnvelopeMode::Attenuate => std::cmp::max(self.volume - 1, 0),
                EnvelopeMode::Amplify => std::cmp::min(self.volume + 1, 15),
            }
        }
        // Update the frequency.
        if let Some(0) = self.sweep_timer.next() {
            let change = self.frequency >> (self.sweep_shift + 1);
            self.frequency += match self.sweep_mode {
                SweepMode::Increase => change,
                SweepMode::Decrease => -change,
            };
            if self.frequency < 0 || self.frequency > 2047 {
                self.is_done = true;
            } else {
                self.freq_timer = SoundSampler::make_freq_timer(self.frequency);
            }
        }
        // Update the duration.
        if self.length_timer.next().unwrap() == 0 && self.stop_on_done {
            self.is_done = true;
        }
        Some([sample])
    }
}
