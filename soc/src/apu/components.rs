use std::iter::Cycle;

use crate::apu::registers::{EnvelopeMode, SquareConfig};
use crate::util::{timer, Timer};

pub struct Envelope {
    pub mode: EnvelopeMode,
    pub timer: Cycle<Timer>,
}

impl Envelope {
    pub fn new(mode: EnvelopeMode, time: i32) -> Envelope {
        Envelope {
            mode,
            timer: timer(time * super::ENVELOPE_PERIOD).cycle(),
        }
    }

    pub fn clock(&mut self, volume: u8) -> u8 {
        if let Some(0) = self.timer.next() {
            match self.mode {
                EnvelopeMode::Attenuate => volume.saturating_sub(1),
                EnvelopeMode::Amplify => (volume + 1).min(15),
            }
        } else {
            volume
        }
    }
}

pub struct Sweep {
    pub shift: i32,
    pub negate: bool,
    pub timer: Cycle<Timer>,
}

impl Sweep {
    pub fn from_config(config: SquareConfig) -> Option<Sweep> {
        if config.sweep_time() > 0 && config.sweep_shift() > 0 {
            Some(Sweep {
                shift: config.sweep_shift().into(),
                negate: config.sweep_negate(),
                timer: timer(super::SWEEP_PERIOD * i32::from(config.sweep_time())).cycle(),
            })
        } else {
            None
        }
    }

    pub fn update(&mut self, frequency: u16) -> Option<u16> {
        if let Some(0) = self.timer.next() {
            let change = frequency >> self.shift;
            Some(if self.negate {
                frequency.wrapping_sub(change)
            } else {
                frequency + change
            })
        } else {
            None
        }
    }
}
