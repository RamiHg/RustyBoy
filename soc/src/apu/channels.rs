use bitfield::bitfield;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;
use sample::frame::Frame as _;

use super::registers::*;
use super::square::SquareWave;
use super::MCYCLE_FREQ;

pub type Frame = sample::frame::Mono<f32>;

#[derive(FromPrimitive)]
pub enum EventType {
    TriggerSquare2,
}

bitfield! {
    pub struct ChannelEvent(u64);
    u8;
    pub into EventType, event_type, set_event_type: 1, 0;
}

#[derive(Default)]
pub struct ChannelState {
    square_2: Option<SquareWave>,
}

impl ChannelState {
    // pub fn handle_event(&mut self, event: ChannelEvent) {
    //     match event {
    //         ChannelEvent::TriggerSquare2(square_config) => {
    //             self.square_2 = Some(SquareWave::new(SquareConfig(square_config)));
    //         }
    //     }
    // }

    pub fn elapsed_secs(&mut self, elapsed: f32) {
        // Everything is relative to the mcycle clock.
        let elapsed_ticks = (MCYCLE_FREQ as f32 / elapsed) as i32;
        self.elapsed_ticks(elapsed_ticks);
    }

    pub fn elapsed_ticks(&mut self, elapsed_ticks: i32) {
        if let Some(wave) = &mut self.square_2 {
            wave.timer.tick(elapsed_ticks);
            if wave.reset_on_done {
                self.square_2 = None;
            }
        }
    }

    pub fn next_sample(&mut self) -> Frame {
        let mut frame = Frame::equilibrium();
        if let Some(wave) = &mut self.square_2 {
            frame[0] += wave.get_sample();
        }
        frame
    }
}
