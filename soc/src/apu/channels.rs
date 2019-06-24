use bitfield::bitfield;
use num_derive::FromPrimitive;
use sample::Frame as _;
use sample::Signal as _;

use super::registers::*;
use super::square::*;
use super::SharedWaveTable;

pub type Frame = sample::frame::Mono<f32>;

#[derive(FromPrimitive)]
pub enum EventType {
    TriggerSquare1,
    TriggerSquare2,
    TriggerWave,
}
from_u8!(EventType);

bitfield! {
    pub struct ChannelEvent(u64);
    u8;
    pub into EventType, event_type, set_event_type: 1, 0;
    pub payload_low, set_payload_low: 31, 24;
    pub u32, payload_high, set_payload_high: 63, 32;
}

#[derive(Default)]
pub struct ChannelState {
    wave_table: SharedWaveTable,
    square_1: Option<SoundSamplerSignal>,
    square_2: Option<SoundSamplerSignal>,
    wave: Option<SoundSamplerSignal>,
}

impl ChannelState {
    pub fn new(wave_table: SharedWaveTable) -> ChannelState {
        ChannelState {
            wave_table,
            ..Default::default()
        }
    }

    pub fn handle_event(&mut self, event: ChannelEvent) {
        match event.event_type() {
            EventType::TriggerSquare1 => {
                let config = SquareConfig::from_low_high(event.payload_low(), event.payload_high());
                // self.square_1 = Some(SoundSampler::from_square_config(config).to_signal());
            }
            EventType::TriggerSquare2 => {
                let config = SquareConfig::from_low_high(event.payload_low(), event.payload_high());
                self.square_2 = Some(SoundSampler::from_square_config(config).to_signal());
            }
            EventType::TriggerWave => {
                let config = WaveConfig::from_low_high(event.payload_low(), event.payload_high());
                let wave_table: u128 = *self.wave_table.read();
                //  self.wave = Some(SoundSampler::from_wave_config(config, wave_table).to_signal());
            }
        }
    }

    pub fn next_sample(&mut self) -> Frame {
        let mut frame = Frame::equilibrium();
        if let Some(wave) = &mut self.square_1 {
            // frame[0] += wave.next()[0] / 6.0;
        }
        if let Some(wave) = &mut self.square_2 {
            frame[0] += wave.next()[0] / 6.0;
        }
        if let Some(wave) = &mut self.wave {
            //frame[0] += wave.next()[0] / 6.0;
        }
        frame
    }
}
