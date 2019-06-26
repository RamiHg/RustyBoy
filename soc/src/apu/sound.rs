use arrayvec::ArrayVec;
use sample::Signal as _;
use std::iter::Cycle;
use std::iter::FromIterator as _;

use crate::apu::registers::{EnvelopeMode, SquareConfig, SweepMode, WaveConfig};
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
    pub timer: CountdownTimer,
}

struct Sweep {
    pub mode: SweepMode,
    pub shift: i32,
    pub timer: Cycle<Timer>,
}

pub struct SoundSampler {
    waveform: ArrayVec<[u8; 32]>,
    waveform_index: i32,
    volume: f32,
    frequency: i32,
    envelope: Option<Envelope>,
    sweep: Option<Sweep>,
    stop_on_done: bool,
    // Timers
    freq_timer: Cycle<CountdownTimer>,
    length_timer: Cycle<Timer>,
    is_done: bool,
}

type Interpolator = sample::interpolate::Floor<sample::frame::Mono<f32>>;
pub type SoundSamplerSignal =
    sample::interpolate::Converter<sample::signal::FromIterator<SoundSampler>, Interpolator>;

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
        let envelope = envelope_setting.map(|(mode, time)| Envelope {
            mode,
            timer: CountdownTimer::new(time, super::ENVELOPE_PERIOD),
        });
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
            stop_on_done,
            freq_timer: SoundSampler::make_freq_timer(frequency, freq_multiplier),
            length_timer: timer(length * super::LENGTH_COUNTER_PERIOD).cycle(),
            is_done: false,
        }
    }

    pub fn into_signal(self) -> SoundSamplerSignal {
        let mut source = sample::signal::from_iter(self);
        let interp = Interpolator::from_source(&mut source);
        source.from_hz_to_hz(interp, super::BASE_FREQ as f64, super::SAMPLE_RATE.into())
    }

    fn make_freq_timer(freq: i32, multiplier: i32) -> std::iter::Cycle<CountdownTimer> {
        CountdownTimer::new(8, (2048 - freq) * multiplier).cycle()
    }
}

impl Iterator for SoundSampler {
    type Item = sample::frame::Mono<f32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }
        let sample = self.waveform[self.waveform_index as usize] as f32 * self.volume;
        if self.freq_timer.next().unwrap().is_some() {
            self.waveform_index = (self.waveform_index + 1) % self.waveform.len() as i32;
        }
        // Update the envelope (volume). Disabled on wave.
        if let Some(Envelope { mode, timer }) = &mut self.envelope {
            if let Some(Some(_)) = timer.next() {
                self.volume = match mode {
                    EnvelopeMode::Attenuate => (self.volume - 1.0).max(0.),
                    EnvelopeMode::Amplify => (self.volume + 1.).min(15.),
                }
            }
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
        if self.length_timer.next().unwrap() == 0 && self.stop_on_done {
            self.is_done = true;
        }
        Some([sample / 15.0])
    }
}
