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

    pub fn clock(&mut self, volume: i32) -> i32 {
        if let Some(0) = self.timer.next() {
            match self.mode {
                EnvelopeMode::Attenuate => (volume - 1).max(0),
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

pub struct Noise {
    pub buzz: bool,
    pub lfsr: u16,
    pub timer: Cycle<Timer>,
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
