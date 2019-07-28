use libsamplerate::{src_delete, src_new, src_process, SRC_STATE_tag, SRC_DATA, SRC_SINC_FASTEST};
use ringbuf::{Consumer, Producer};
use std::collections::VecDeque;
use std::os::raw::c_long;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use super::device::{DEVICE_RATE, FRAMES_PER_BUFFER};
use super::mixer::{ChannelMixer, SharedAudioRegs, StereoFrame};

use slice_deque::SliceDeque;

/// The Nyquist rate of the audio system. I.e., twice the maximum theoretical frequency, which is
/// 1MiHz.
const MIN_SAMPLE_RATE: f32 = 4_194_304.0 / 2.0;

const IDEAL_SAMPLE_RATE: f32 = 64_000.0;

const SHARED_RINGBUFFER_SIZE: usize =
    (MIN_SAMPLE_RATE / DEVICE_RATE * FRAMES_PER_BUFFER as f32 * 2.0) as usize;

pub fn make_audio_threads(audio_regs: SharedAudioRegs) -> (Box<Resampler>, Arc<AtomicBool>) {
    let (sample_producer, sample_consumer) =
        ringbuf::RingBuffer::<StereoFrame>::new(SHARED_RINGBUFFER_SIZE).split();
    let resampler = Box::new(Resampler::new(sample_consumer));
    let sampler_kill = SamplerThread::spawn(audio_regs, sample_producer);
    (resampler, sampler_kill)
}

/// Resamples audio from the sampled MIN_SAMPLE_RATE into DEVICE_RATE (48kHz). Is called by the
/// device callback when audio samples are requested.
pub struct Resampler {
    resampler: *mut SRC_STATE_tag,
    resample_dst_scratch: Vec<StereoFrame>,
    sample_buffer: VecDeque<StereoFrame>,

    sample_receiver: Consumer<StereoFrame>,
    sample_src_buffer: SliceDeque<StereoFrame>,
}

impl Resampler {
    pub fn new(sample_receiver: Consumer<StereoFrame>) -> Resampler {
        let mut error: i32 = 0;
        let resampler = unsafe { src_new(SRC_SINC_FASTEST as i32, 2, &mut error) };
        assert_eq!(error, 0);

        Resampler {
            // mixer,
            resampler,
            resample_dst_scratch: vec![StereoFrame::default(); FRAMES_PER_BUFFER * 2 + 16],
            sample_buffer: VecDeque::with_capacity(FRAMES_PER_BUFFER * 2 + 1),

            sample_receiver,
            sample_src_buffer: SliceDeque::with_capacity(SHARED_RINGBUFFER_SIZE),
        }
    }

    pub fn stream_callback(&mut self, buffer: &mut [f32]) {
        let _now = std::time::Instant::now();
        let buffer: &mut [StereoFrame] = sample::slice::to_frame_slice_mut(buffer)
            .expect("Couldn't convert output buffer to stereo.");
        // Clear the scratch buffer and sample the amount of samples needed to satisfy the buffer.
        let mcycles_to_sample =
            (MIN_SAMPLE_RATE / DEVICE_RATE * buffer.len() as f32 + 1.0) as usize;
        let prev_size = self.sample_src_buffer.len();
        unsafe {
            self.sample_src_buffer.move_tail(mcycles_to_sample as isize);
        }
        let num_available_samples = self
            .sample_receiver
            .pop_slice(&mut self.sample_src_buffer[prev_size..mcycles_to_sample])
            .unwrap_or_default();
        debug_assert_ge!(self.sample_src_buffer.capacity(), num_available_samples);
        if self.sample_src_buffer.len() < mcycles_to_sample {
            trace!(target: "audio", "Sample buffer underrun. Got {} out of {} needed samples. Skipping frame.",
            num_available_samples, mcycles_to_sample);
            buffer.iter_mut().for_each(|x| *x = StereoFrame::default());
            return;
        }
        // Resample the samples down to the device sample rate.
        let _resample_time = std::time::Instant::now();
        let mut data = SRC_DATA {
            data_in: self.sample_src_buffer.as_slice().as_ptr() as *const _,
            data_out: self.resample_dst_scratch.as_mut_ptr() as *mut _,
            input_frames: mcycles_to_sample as c_long,
            output_frames: self.resample_dst_scratch.len() as c_long,
            input_frames_used: 0,
            output_frames_gen: 0,
            end_of_input: 0,
            src_ratio: (DEVICE_RATE / MIN_SAMPLE_RATE) as f64,
        };
        let result = unsafe { src_process(self.resampler, &mut data) };
        assert_le!(data.output_frames_gen as usize, self.resample_dst_scratch.len());
        debug_assert_eq!(result, 0);
        unsafe {
            self.sample_src_buffer.move_head(data.input_frames_used as isize);
        }
        let frames: &[StereoFrame] = sample::slice::to_frame_slice(
            &self.resample_dst_scratch[..data.output_frames_gen as usize],
        )
        .expect("Couldn't convert to stereo.");
        debug_assert_ge!(self.sample_buffer.capacity(), frames.len());
        // TODO: Can probably remove this copy. Not that it matters.
        self.sample_buffer.extend(frames.iter());
        // Update any global state.
        // Finally, write out the samples to the buffer.
        for out_frame in buffer.iter_mut() {
            let sample = self.sample_buffer.pop_front();
            let sample = sample.unwrap_or_default();
            *out_frame = sample;
        }
        // println!("Took {:#?} total. {:#?} in resampling", _now.elapsed(), _resample_time.elapsed(),);
    }
}

impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe {
            src_delete(self.resampler);
        }
    }
}

/// Sampler thread that periodically polls the system to check for updates to audio registers, and
/// to produce samples that will be used by the callback thread.
struct SamplerThread {
    kill_signal: Arc<AtomicBool>,

    mixer: ChannelMixer,
    sample_producer: Producer<StereoFrame>,
    scratch: Vec<StereoFrame>,
}

impl SamplerThread {
    fn spawn(
        audio_regs: SharedAudioRegs,
        sample_producer: Producer<StereoFrame>,
    ) -> Arc<AtomicBool> {
        let kill_signal = Arc::new(AtomicBool::new(false));
        let mut sampler = SamplerThread {
            kill_signal: Arc::clone(&kill_signal),
            mixer: ChannelMixer::new(audio_regs),
            sample_producer,
            scratch: Vec::new(),
        };
        thread::spawn(move || sampler.audio_loop());
        kill_signal
    }

    fn audio_loop(&mut self) {
        use std::time::Instant;

        const APU_SAMPLES_PER_NS: f32 = MIN_SAMPLE_RATE / 1e9;
        let ideal_ns_per_wakeup =
            std::time::Duration::from_nanos((1e9 / IDEAL_SAMPLE_RATE).ceil() as u64);

        let mut timer = Instant::now();
        loop {
            let elapsed_ns = timer.elapsed();
            timer += elapsed_ns;
            let elapsed_ns = elapsed_ns.as_nanos() as f32;

            let num_to_sample = (elapsed_ns * APU_SAMPLES_PER_NS).ceil() as usize;

            self.mixer.on_sample_begin();
            self.scratch.clear();
            for _ in 0..num_to_sample {
                // Skip every other sample to downsample from 4MiHz to 2MiHz.
                self.mixer.next_sample();
                self.scratch.push(self.mixer.next_sample());
            }
            let num_written = loop {
                let write_result = self.sample_producer.push_slice(self.scratch.as_slice());
                if let Err(ringbuf::PushSliceError::Full) = write_result {
                    // Simply sleep and try again.
                    std::thread::sleep(ideal_ns_per_wakeup);
                } else {
                    break write_result.unwrap();
                }
            };
            debug_assert_eq!(num_written, self.scratch.len());
            self.mixer.on_sample_end();
            if let Some(time_to_sleep) = ideal_ns_per_wakeup.checked_sub(timer.elapsed()) {
                std::thread::sleep(time_to_sleep);
            }
        }
    }
}
