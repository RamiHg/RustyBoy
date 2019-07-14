use arrayvec::ArrayVec;
use sample::Frame as _;
use std::iter::Cycle;
use std::sync::{atomic::AtomicU64, atomic::AtomicU8, Arc};

use super::registers::*;
use super::sound::{ComponentCycle, Sound, SoundSampler, Square};
use super::SharedWaveTable;
use crate::util::{iterate_bits, timer, Timer};

pub type StereoFrame = sample::frame::Stereo<f32>;

// pub enum SoundType {
//     Square1,
//     Square2,
//     Wave,
// }

pub enum ChannelEvent {
    TriggerSquare1(SquareConfig),
    TriggerSquare2(SquareConfig),
    TriggerWave(WaveConfig),
    TriggerNoise(NoiseConfig),
}

/// A local snapshot of ChannelState taken at the beginning of the audio callback. We take a
/// snapshot in order to prevent reading the various atomic configs hundreds of time per sample.
/// This is mainly used to handle length counter reloading.
/// The only actually shared variable is SoundStatus, which needs to be updated when a sound
/// finishes.
#[derive(Default, Clone)]
pub struct CachedAudioRegs {
    pub global_sound_status: Arc<AtomicU8>,
    pub sound_mix: ChannelMixConfig,
    pub volume_control: VolumeControl,
    pub square_1_config: SquareConfig,
    pub square_2_config: SquareConfig,
    pub wave_config: WaveConfig,
    pub noise_config: NoiseConfig,
}

impl CachedAudioRegs {
    pub fn new(global_sound_status: Arc<AtomicU8>) -> CachedAudioRegs {
        CachedAudioRegs {
            global_sound_status,
            ..Default::default()
        }
    }

    pub fn sync_from_shared(&mut self, state: &SharedAudioRegs) {
        use std::sync::atomic::Ordering;
        self.sound_mix = ChannelMixConfig(state.sound_mix.load(Ordering::Acquire));
        self.volume_control = VolumeControl(state.volume_control.load(Ordering::Acquire));
        self.square_1_config = SquareConfig(state.square_1_config.load(Ordering::Acquire));
        self.square_2_config = SquareConfig(state.square_2_config.load(Ordering::Acquire));
        self.wave_config = WaveConfig(state.wave_config.load(Ordering::Acquire));
        self.noise_config = NoiseConfig(state.noise_config.load(Ordering::Acquire));
    }

    pub fn sync_to_shared(&self, prev_state: &CachedAudioRegs, state: &mut SharedAudioRegs) {
        let ordering = std::sync::atomic::Ordering::Relaxed;
        // TODO: Only updating square for now.
        state.square_1_config.compare_and_swap(
            prev_state.square_1_config.0,
            self.square_1_config.0,
            ordering,
        );
        state.square_2_config.compare_and_swap(
            prev_state.square_2_config.0,
            self.square_2_config.0,
            ordering,
        );
    }
}

#[derive(Default, Clone)]
pub struct SharedAudioRegs {
    pub sound_mix: Arc<AtomicU8>,
    pub sound_status: Arc<AtomicU8>,
    pub volume_control: Arc<AtomicU8>,
    pub square_1_config: Arc<AtomicU64>,
    pub square_2_config: Arc<AtomicU64>,
    pub wave_config: Arc<AtomicU64>,
    pub noise_config: Arc<AtomicU64>,
    pub wave_table: SharedWaveTable,
}

impl SharedAudioRegs {
    pub fn poll_events(&mut self) -> ArrayVec<[ChannelEvent; 4]> {
        let mut events = ArrayVec::new();
        if let Some(config) = SharedAudioRegs::poll_event(&mut self.square_1_config) {
            events.push(ChannelEvent::TriggerSquare1(SquareConfig(config)));
        }
        if let Some(config) = SharedAudioRegs::poll_event(&mut self.square_2_config) {
            events.push(ChannelEvent::TriggerSquare2(SquareConfig(config)));
        }
        if let Some(config) = SharedAudioRegs::poll_event(&mut self.wave_config) {
            events.push(ChannelEvent::TriggerWave(WaveConfig(config)));
        }
        if let Some(config) = SharedAudioRegs::poll_event(&mut self.noise_config) {
            events.push(ChannelEvent::TriggerNoise(NoiseConfig(config)));
        }
        events
    }

    fn poll_event(config: &mut Arc<AtomicU64>) -> Option<u64> {
        // https://bartoszmilewski.com/2008/12/01/c-atomics-and-memory-ordering/
        use std::sync::atomic::Ordering;
        let mut current_value = CommonSoundConfig(config.load(Ordering::Acquire));
        loop {
            // Exit now if we don't have any audio to trigger.
            let mut new_value = current_value;
            if !new_value.triggered() {
                return None;
            }
            new_value.set_triggered(false);
            match config.compare_exchange_weak(
                current_value.0,
                new_value.0,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(new_value.0),
                Err(x) => {
                    current_value = CommonSoundConfig(x);
                    strict_fail!(
                        "Actually have contention with main thread! On {:?}",
                        current_value
                    );
                }
            }
        }
    }
}

use std::cell::RefCell;

pub struct ChannelMixer {
    global_regs: SharedAudioRegs,
    cached_regs: RefCell<CachedAudioRegs>,
    square_1: Option<Square>,
    square_2: Option<Square>,
    wave: Option<SoundSampler>,
    noise: Option<SoundSampler>,

    length_timer: Cycle<Timer>,
}

impl ChannelMixer {
    pub fn new(global_regs: SharedAudioRegs) -> ChannelMixer {
        let global_sound_status = global_regs.sound_status.clone();
        ChannelMixer {
            global_regs,
            cached_regs: RefCell::new(CachedAudioRegs::new(global_sound_status)),
            square_1: None,
            square_2: None,
            wave: None,
            noise: None,
            length_timer: timer(super::LENGTH_COUNTER_PERIOD).cycle(),
        }
    }

    pub fn on_sample_begin(&mut self) {
        self.handle_events();
        // Update all current sounds with any changes from the audio registers.
        let cached_regs = self.cached_regs.borrow();
        if let Some(square) = &mut self.square_1 {
            square.update_from_reg(cached_regs.square_1_config.0);
        }
        if let Some(square) = &mut self.square_2 {
            square.update_from_reg(cached_regs.square_2_config.0);
        }
    }

    pub fn on_sample_end(&mut self) {
        // Update the global registers based on the current sound state.
        let prev_state = self.cached_regs.clone().into_inner();
        let mut cached_regs = self.cached_regs.borrow_mut();
        if let Some(square) = &mut self.square_1 {
            square.update_to_reg(&mut cached_regs.square_1_config.0);
        }
        if let Some(square) = &mut self.square_2 {
            square.update_to_reg(&mut cached_regs.square_2_config.0);
        }
        cached_regs.sync_to_shared(&prev_state, &mut self.global_regs);
    }

    fn handle_events(&mut self) {
        for event in self.global_regs.poll_events() {
            self.handle_event(event);
        }
        // TODO: Can combine in one pass.
        self.cached_regs
            .borrow_mut()
            .sync_from_shared(&mut self.global_regs);
    }

    fn handle_event(&mut self, event: ChannelEvent) {
        use ChannelEvent::*;
        match event {
            TriggerSquare1(config) => {
                self.square_1 = Some(Square::new(config));
            }
            TriggerSquare2(config) => {
                self.square_2 = Some(Square::new(config));
            }
            TriggerWave(config) => {
                let wave_table: u128 = *self.global_regs.wave_table.try_read().unwrap();
                self.wave = Some(SoundSampler::from_wave_config(config, wave_table));
            }
            TriggerNoise(config) => {
                self.noise = Some(SoundSampler::from_noise_config(config));
            }
        }
    }

    pub fn next_sample(&mut self) -> StereoFrame {
        // First, collect all the mono frames.
        // TODO: Will be simplified once all sounds are implemented as traits.
        let mut component_cycles = ComponentCycle::empty();
        if self.length_timer.next().unwrap() == 0 {
            component_cycles |= ComponentCycle::LENGTH;
        }
        let mono_frames = [
            self.square_1.as_mut().map(|s| s.sample(component_cycles)),
            self.square_2.as_mut().map(|s| s.sample(component_cycles)),
            self.wave.as_mut().map(Iterator::next),
            self.noise.as_mut().map(Iterator::next),
        ]
        .iter()
        .cloned()
        .map(Option::unwrap_or_default)
        .map(Option::unwrap_or_default)
        .collect::<ArrayVec<[f32; 4]>>();

        let sound_mix = self.cached_regs.borrow_mut().sound_mix;

        let mut frame = [0.0, 0.0];
        let mut add_to_frame = |idx, bits| {
            for (mono, _) in mono_frames
                .iter()
                .zip(iterate_bits(bits))
                .filter(|&(_, is_on)| is_on)
            {
                frame[idx] += mono / 4.0;
            }
        };
        let volume_control = self.cached_regs.borrow_mut().volume_control;
        // Mix in the right channel.
        add_to_frame(1, sound_mix.0);
        // And the left channel.
        add_to_frame(0, sound_mix.0 >> 4);
        // Scale left/right volumes.
        frame[0] *= volume_control.left() as f32 + 1.0;
        frame[1] *= volume_control.right() as f32 + 1.0;
        debug_assert_le!(frame[0], 15.0);
        debug_assert_ge!(frame[0], 0.0);
        debug_assert_le!(frame[1], 15.0);
        debug_assert_ge!(frame[1], 0.0);
        frame = frame.scale_amp(1.0 / 15.0);
        frame
    }
}
