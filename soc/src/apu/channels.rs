use arrayvec::ArrayVec;
use sample::Frame as _;
use std::sync::{atomic::AtomicU64, atomic::AtomicU8, Arc};

use super::registers::*;
use super::sound::{SoundSampler, SoundSamplerSignal};
use super::SharedWaveTable;
use crate::util::iterate_bits;

pub type MonoFrame = sample::frame::Mono<f32>;
pub type StereoFrame = sample::frame::Stereo<f32>;

pub enum SoundType {
    Square1,
    Square2,
    Wave,
}

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
    pub sound_mix: u8,
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

    pub fn update_state(&mut self, state: &mut SharedAudioRegs) {
        use std::sync::atomic::Ordering;
        self.sound_mix = state.sound_mix.load(Ordering::Acquire);
        self.volume_control = VolumeControl(state.volume_control.load(Ordering::Acquire));
        self.square_1_config = SquareConfig(state.square_1_config.load(Ordering::Acquire));
        self.square_2_config = SquareConfig(state.square_2_config.load(Ordering::Acquire));
        self.wave_config = WaveConfig(state.wave_config.load(Ordering::Acquire));
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
    pub fn poll_events(&mut self) -> ArrayVec<[ChannelEvent; 3]> {
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
                    panic!(
                        "Actually have contention with main thread! On {:?}",
                        current_value
                    );
                }
            }
        }
    }
}

use std::cell::RefCell;
use std::rc::Rc;

pub struct ChannelMixer {
    global_regs: SharedAudioRegs,
    cached_regs: Rc<RefCell<CachedAudioRegs>>,
    square_1: Option<SoundSamplerSignal>,
    square_2: Option<SoundSamplerSignal>,
    wave: Option<SoundSamplerSignal>,
    noise: Option<SoundSamplerSignal>,
}

impl ChannelMixer {
    pub fn new(global_regs: SharedAudioRegs) -> ChannelMixer {
        let global_sound_status = global_regs.sound_status.clone();
        ChannelMixer {
            global_regs,
            cached_regs: Rc::new(RefCell::new(CachedAudioRegs::new(global_sound_status))),
            square_1: None,
            square_2: None,
            wave: None,
            noise: None,
        }
    }

    pub fn handle_events(&mut self) {
        self.cached_regs
            .borrow_mut()
            .update_state(&mut self.global_regs);
        for event in self.global_regs.poll_events() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: ChannelEvent) {
        use ChannelEvent::*;
        match event {
            TriggerSquare1(config) => {
                self.square_1 = Some(SoundSampler::from_square_config(config).into_signal());
            }
            TriggerSquare2(config) => {
                self.square_2 = Some(SoundSampler::from_square_config(config).into_signal());
            }
            TriggerWave(config) => {
                let wave_table: u128 = *self.global_regs.wave_table.try_read().unwrap();
                self.wave = Some(SoundSampler::from_wave_config(config, wave_table).into_signal());
            }
            TriggerNoise(config) => {}
        }
    }

    pub fn next_sample(&mut self) -> StereoFrame {
        use sample::Signal;
        // First, collect all the mono frames.
        let mono_frames = [
            self.square_1.iter_mut(),
            self.square_2.iter_mut(),
            self.wave.iter_mut(),
            None.iter_mut(),
        ]
        .iter_mut()
        .flatten()
        .map(|wave| wave.next())
        .collect::<ArrayVec<[MonoFrame; 4]>>();

        let mut frame = StereoFrame::equilibrium();
        let mut add_to_frame = |idx, bits| {
            for (mono, _) in mono_frames
                .iter()
                .zip(iterate_bits(bits))
                .filter(|&(_, is_on)| is_on)
            {
                frame[idx] += mono[0] / 4.0;
            }
        };
        let sound_mix = self.cached_regs.borrow_mut().sound_mix;
        let volume_control = self.cached_regs.borrow_mut().volume_control;
        // Mix in the right channel.
        add_to_frame(1, sound_mix);
        // And the left channel.
        add_to_frame(0, sound_mix >> 4);
        // Scale left/right volumes.
        frame = frame.mul_amp([
            volume_control.left() as f32 + 1.0,
            volume_control.right() as f32 + 1.0,
        ]);
        debug_assert_le!(frame[0], 15.0);
        debug_assert_ge!(frame[0], 0.0);
        debug_assert_le!(frame[1], 15.0);
        debug_assert_ge!(frame[1], 0.0);
        frame = frame.scale_amp(1.0 / 15.0);
        // // And finally, clamp.
        // frame[0] = frame[0].min(1.0).max(-1.0);
        // frame[1] = frame[1].min(1.0).max(-1.0);
        //dbg!(frame);
        frame
    }
}
