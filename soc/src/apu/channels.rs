use arrayvec::ArrayVec;
use sample::Frame as _;
use sample::Signal as _;
use std::sync::{atomic::AtomicU64, Arc};

use super::registers::*;
use super::sound::{SoundSampler, SoundSamplerSignal};
use super::SharedWaveTable;

pub type Frame = sample::frame::Mono<f32>;

pub enum ChannelEvent {
    TriggerSquare1(SquareConfig),
    TriggerSquare2(SquareConfig),
    TriggerWave(WaveConfig),
}

#[derive(Default, Clone)]
pub struct ChannelState {
    pub square_1_config: Arc<AtomicU64>,
    pub square_2_config: Arc<AtomicU64>,
    pub wave_config: Arc<AtomicU64>,
    pub wave_table: SharedWaveTable,
}

impl ChannelState {
    pub fn poll_events(&mut self) -> ArrayVec<[ChannelEvent; 3]> {
        let mut events = ArrayVec::new();
        if let Some(config) = ChannelState::poll_event(&mut self.square_1_config) {
            events.push(ChannelEvent::TriggerSquare1(SquareConfig(config)));
        }
        if let Some(config) = ChannelState::poll_event(&mut self.square_2_config) {
            events.push(ChannelEvent::TriggerSquare2(SquareConfig(config)));
        }
        if let Some(config) = ChannelState::poll_event(&mut self.wave_config) {
            events.push(ChannelEvent::TriggerWave(WaveConfig(config)));
        }
        events
    }

    fn poll_event(config: &mut Arc<AtomicU64>) -> Option<u64> {
        // https://bartoszmilewski.com/2008/12/01/c-atomics-and-memory-ordering/
        use std::sync::atomic::Ordering;
        let mut current_value = CommonSoundConfig(config.load(Ordering::Relaxed));
        // Early exit if there's nothing to do.
        if !current_value.triggered() {
            return None;
        }
        loop {
            let mut new_value = current_value;
            new_value.set_triggered(false);
            match config.compare_exchange_weak(
                current_value.0,
                new_value.0,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(new_value.0),
                Err(x) => {
                    current_value = CommonSoundConfig(x);
                    panic!(
                        "Actually have contention with main thread! On {:?}",
                        current_value
                    );
                }
            }
        }
    }
}

pub struct ChannelMixer {
    wave_table: SharedWaveTable,
    square_1: Option<SoundSamplerSignal>,
    square_2: Option<SoundSamplerSignal>,
    wave: Option<SoundSamplerSignal>,
}

impl ChannelMixer {
    pub fn new(wave_table: SharedWaveTable) -> ChannelMixer {
        ChannelMixer {
            wave_table,
            square_1: None,
            square_2: None,
            wave: None,
        }
    }

    pub fn handle_event(&mut self, event: ChannelEvent) {
        use ChannelEvent::*;
        match event {
            TriggerSquare1(config) => {
                self.square_1 = Some(SoundSampler::from_square_config(config).into_signal());
            }
            TriggerSquare2(config) => {
                self.square_2 = Some(SoundSampler::from_square_config(config).into_signal());
            }
            TriggerWave(config) => {
                let wave_table: u128 = *self.wave_table.read();
                self.wave = Some(SoundSampler::from_wave_config(config, wave_table).into_signal());
            }
        }
    }

    pub fn next_sample(&mut self) -> Frame {
        let mut frame = Frame::equilibrium();
        if let Some(wave) = &mut self.square_1 {
            frame[0] += wave.next()[0] / 6.0;
        }
        if let Some(wave) = &mut self.square_2 {
            frame[0] += wave.next()[0] / 6.0;
        }
        if let Some(wave) = &mut self.wave {
            frame[0] += wave.next()[0] / 6.0;
        }
        frame
    }
}
