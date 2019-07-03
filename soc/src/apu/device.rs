use portaudio as pa;
use rust_liquid_dsp::filter::MultiStageResampler;
use std::collections::VecDeque;

use super::channels::{ChannelMixer, SharedAudioRegs, StereoFrame};

const FRAMES_PER_BUFFER: usize = 32;
const NUM_CHANNELS: usize = 2;

/// The Nyquist rate of the audio system. I.e., twice the maximum theoretical frequency, which is
/// 1MiHz.
const MIN_SAMPLE_RATE: f32 = 4_194_304.0 / 2.0;
/// The sampling rate chosen for the device.
pub const DEVICE_RATE: f32 = 48_000.0;

type PaOutType = f32;

#[allow(dead_code)]
pub struct Device {
    pa: pa::PortAudio,
    pa_stream: pa::stream::Stream<pa::stream::NonBlocking, pa::stream::Output<PaOutType>>,
}

impl Device {
    pub fn try_new(global_regs: SharedAudioRegs) -> Result<Device, pa::Error> {
        let pa = pa::PortAudio::new()?;
        let mut settings = pa.default_output_stream_settings::<PaOutType>(
            NUM_CHANNELS as i32,
            DEVICE_RATE.into(),
            FRAMES_PER_BUFFER as u32,
        )?;
        settings.flags |= pa::stream::flags::CLIP_OFF | pa::stream::flags::DITHER_OFF;
        // Create the channel for communicating with the APU.
        let mut thread = AudioThread::new(global_regs);
        let mut pa_stream =
            pa.open_non_blocking_stream(settings, move |args| thread.stream_callback(args))?;
        pa_stream.start()?;
        Ok(Device { pa, pa_stream })
    }
}

struct AudioThread {
    mixer: ChannelMixer,
    resampler: [MultiStageResampler<f32>; 2],

    sample_buffer: VecDeque<StereoFrame>,
    resample_src_scratch: [Vec<f32>; NUM_CHANNELS],
    resample_dst_scratch: [Vec<f32>; NUM_CHANNELS],
}

impl AudioThread {
    pub fn new(audio_regs: SharedAudioRegs) -> AudioThread {
        let mixer = ChannelMixer::new(audio_regs);
        let resampler = MultiStageResampler::new(DEVICE_RATE / MIN_SAMPLE_RATE, 80.0);
        AudioThread {
            mixer,
            resampler: [resampler.clone(), resampler],
            sample_buffer: VecDeque::with_capacity(FRAMES_PER_BUFFER + 1),
            resample_src_scratch: [vec![0.0; 2048], vec![0.0; 2048]],
            resample_dst_scratch: [vec![0.0; 128], vec![0.0; 128]],
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<PaOutType>,
    ) -> pa::stream::CallbackResult {
        let _now = std::time::Instant::now();
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        self.mixer.handle_events();
        // Clear the scratch buffer and sample the amount of sampled needed to get an amortized
        // FRAMES_PER_BUFFER samples per callback.
        self.resample_src_scratch[0].clear();
        self.resample_src_scratch[1].clear();
        const MCYCLES_TO_SAMPLE: i32 =
            (MIN_SAMPLE_RATE / DEVICE_RATE * FRAMES_PER_BUFFER as f32 + 1.0) as i32;
        for _ in 0..MCYCLES_TO_SAMPLE {
            // Skip every other sample (to downsample to 2MiHz).
            self.mixer.next_sample();
            let sample = self.mixer.next_sample();
            self.resample_src_scratch[0].push(sample[0]);
            self.resample_src_scratch[1].push(sample[1]);
        }
        // Resample the samples down to the device sample rate.
        let _resample_time = std::time::Instant::now();
        let mut written_samples = 0;
        for channel in 0..NUM_CHANNELS {
            written_samples = self.resampler[channel]
                .filter(
                    self.resample_src_scratch[channel].as_mut_slice(),
                    self.resample_dst_scratch[channel].as_mut_slice(),
                )
                .unwrap() as usize;
            debug_assert_le!(written_samples, self.sample_buffer.capacity());
        }
        let frames = self.resample_dst_scratch[0][0..written_samples]
            .iter()
            .zip(&self.resample_dst_scratch[1][0..written_samples]);
        self.sample_buffer.extend(frames.map(|(x, y)| [*x, *y]));
        // Finally, write out the samples to the buffer.
        let buffer: &mut [[PaOutType; 2]] = sample::slice::to_frame_slice_mut(buffer).unwrap();
        for out_frame in buffer.iter_mut() {
            let sample = self.sample_buffer.pop_front().unwrap();
            //print!("{} ", sample);
            *out_frame = sample;
        }
        pa::Continue
    }
}
