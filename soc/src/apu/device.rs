use libsamplerate::{src_delete, src_new, src_process, SRC_STATE_tag, SRC_DATA, SRC_SINC_FASTEST};
use std::collections::VecDeque;
use std::os::raw::c_long;

use super::channels::{ChannelMixer, SharedAudioRegs, StereoFrame};

const FRAMES_PER_BUFFER: usize = 32;
const NUM_CHANNELS: usize = 2;

/// The Nyquist rate of the audio system. I.e., twice the maximum theoretical frequency, which is
/// 1MiHz.
const MIN_SAMPLE_RATE: f32 = 4_194_304.0 / 2.0;
/// The sampling rate chosen for the device.
pub const DEVICE_RATE: f32 = 48_000.0;

pub use platform::*;

mod platform {
    use super::*;
    use portaudio as pa;
    #[allow(dead_code)]
    pub struct Device {
        pa: pa::PortAudio,
        pa_stream: pa::stream::Stream<pa::stream::NonBlocking, pa::stream::Output<f32>>,
    }

    impl Device {
        pub fn try_new(global_regs: SharedAudioRegs) -> Result<Device, pa::Error> {
            let pa = pa::PortAudio::new()?;
            let mut settings = pa.default_output_stream_settings::<f32>(
                NUM_CHANNELS as i32,
                DEVICE_RATE.into(),
                FRAMES_PER_BUFFER as u32,
            )?;
            settings.flags |= pa::stream::flags::CLIP_OFF | pa::stream::flags::DITHER_OFF;
            // Create the channel for communicating with the APU.
            let mut thread = AudioThread::new(global_regs);
            let mut pa_stream = pa.open_non_blocking_stream(settings, move |args| {
                let pa::OutputStreamCallbackArgs { buffer, .. } = args;
                thread.stream_callback(buffer);
                pa::Continue
            })?;
            pa_stream.start()?;
            Ok(Device { pa, pa_stream })
        }
    }
}

struct AudioThread {
    mixer: ChannelMixer,
    resampler: *mut SRC_STATE_tag,
    resample_src_scratch: Vec<StereoFrame>,
    resample_dst_scratch: Vec<StereoFrame>,
    sample_buffer: VecDeque<StereoFrame>,
}

impl AudioThread {
    pub fn new(audio_regs: SharedAudioRegs) -> AudioThread {
        let mixer = ChannelMixer::new(audio_regs);

        let mut error: i32 = 0;
        let resampler = unsafe { src_new(SRC_SINC_FASTEST as i32, 2, &mut error) };
        assert_eq!(error, 0);

        AudioThread {
            mixer,
            resampler,
            resample_src_scratch: vec![StereoFrame::default(); 44 * FRAMES_PER_BUFFER],
            resample_dst_scratch: vec![StereoFrame::default(); FRAMES_PER_BUFFER + 16],
            sample_buffer: VecDeque::with_capacity(FRAMES_PER_BUFFER + 1),
        }
    }

    pub fn stream_callback(&mut self, buffer: &mut [f32]) {
        let _now = std::time::Instant::now();
        let buffer: &mut [StereoFrame] = sample::slice::to_frame_slice_mut(buffer)
            .expect("Couldn't convert output buffer to stereo.");
        let frames_per_buffer = buffer.len();
        self.mixer.on_sample_begin();
        // Clear the scratch buffer and sample the amount of sampled needed to get an amortized
        // FRAMES_PER_BUFFER samples per callback.
        const MCYCLES_TO_SAMPLE: i32 =
            (MIN_SAMPLE_RATE / DEVICE_RATE * FRAMES_PER_BUFFER as f32 + 1.0) as i32;
        self.resample_src_scratch.clear();
        for _ in 0..MCYCLES_TO_SAMPLE {
            // Skip every other sample (to downsample to 2MiHz).
            self.mixer.next_sample();
            let sample = self.mixer.next_sample();
            self.resample_src_scratch.push(sample);
        }
        // Resample the samples down to the device sample rate.
        let _resample_time = std::time::Instant::now();
        let mut data = SRC_DATA {
            data_in: self.resample_src_scratch.as_ptr() as *const _,
            data_out: self.resample_dst_scratch.as_mut_ptr() as *mut _,
            input_frames: self.resample_src_scratch.len() as c_long,
            output_frames: self.resample_dst_scratch.len() as c_long,
            input_frames_used: 0,
            output_frames_gen: 0,
            end_of_input: 0,
            src_ratio: (DEVICE_RATE / MIN_SAMPLE_RATE) as f64,
        };
        let result = unsafe { src_process(self.resampler, &mut data) };
        assert_le!(
            data.output_frames_gen as usize,
            self.resample_dst_scratch.len()
        );
        debug_assert_eq!(result, 0);
        let frames: &[StereoFrame] = sample::slice::to_frame_slice(
            &self.resample_dst_scratch[..data.output_frames_gen as usize],
        )
        .expect("Couldn't convert to stereo.");
        debug_assert_ge!(self.sample_buffer.capacity(), frames.len());
        // TODO: Can probably remove this copy. Not that it matters.
        self.sample_buffer.extend(frames.iter());
        // Update any global state.
        self.mixer.on_sample_end();
        // println!(
        //     "Took {} ms",
        //     _resample_time.elapsed().as_micros() as f32 / 1000.0
        // );
        // Finally, write out the samples to the buffer.
        for out_frame in buffer.iter_mut() {
            let sample = self.sample_buffer.pop_front();
            let sample = sample.unwrap_or_default();
            *out_frame = sample;
        }
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        unsafe {
            src_delete(self.resampler);
        }
    }
}
