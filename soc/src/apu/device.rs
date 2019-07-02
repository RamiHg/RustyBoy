use portaudio as pa;
use sample;
use std::collections::VecDeque;

use super::channels::{ChannelMixer, SharedAudioRegs};

const FRAMES_PER_BUFFER: usize = 32;
const NUM_CHANNELS: i32 = 2;

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
            NUM_CHANNELS,
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
    last_time: f64,
    elapsed_time: f64,
    resampler: rust_liquid_dsp::filter::MultiStageResampler<f32>,

    sample_buffer: VecDeque<f32>,
    resample_src_scratch: Vec<f32>,
    resample_dst_scratch: Vec<f32>,

    f_writer: Option<std::io::BufWriter<std::fs::File>>,
}

impl AudioThread {
    pub fn new(audio_regs: SharedAudioRegs) -> AudioThread {
        let mixer = ChannelMixer::new(audio_regs);
        let resampler =
            rust_liquid_dsp::filter::MultiStageResampler::new(DEVICE_RATE / MIN_SAMPLE_RATE, 80.0);
        AudioThread {
            mixer,
            last_time: -1.0,
            elapsed_time: 0.0,
            resampler,
            sample_buffer: VecDeque::with_capacity(FRAMES_PER_BUFFER + 1),
            resample_src_scratch: vec![0.0; 2048],
            resample_dst_scratch: vec![0.0; 128],
            f_writer: Some(std::io::BufWriter::with_capacity(
                1 * 1024 * 1024,
                std::fs::File::create("samples.bin").unwrap(),
            )),
        }
    }

    fn debug_write_sample(&mut self, sample: f32) {
        use std::io::Write;
        if let Some(writer) = &mut self.f_writer {
            debug_assert_le!(sample, 1.0);
            let byte = (sample * 255.0).round() as u8;
            writer.write_all(&[byte]).unwrap();
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<PaOutType>,
    ) -> pa::stream::CallbackResult {
        self.f_writer = None;
        let _now = std::time::Instant::now();
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        let elapsed_secs = if self.last_time >= 0.0 {
            time.current - self.last_time
        } else {
            0.0
        };
        self.elapsed_time += elapsed_secs;
        self.last_time = time.current;
        self.mixer.handle_events();
        let buffer: &mut [[PaOutType; 2]] = sample::slice::to_frame_slice_mut(buffer).unwrap();

        if super::use_lowpass() {
            // Clear the scratch buffer and sample the amount of sampled needed to get an amortized
            // FRAMES_PER_BUFFER samples per callback.
            self.resample_src_scratch.clear();
            const MCYCLES_TO_SAMPLE: i32 =
                (MIN_SAMPLE_RATE / DEVICE_RATE * FRAMES_PER_BUFFER as f32 + 1.0) as i32;
            for _ in 0..MCYCLES_TO_SAMPLE {
                // Skip every other sample (to downsample to 2MiHz).
                self.mixer.next_sample();
                let sample = self.mixer.next_sample()[0];
                self.debug_write_sample(sample);
                self.resample_src_scratch.push(sample);
            }
            // Resample the samples down to the device sample rate.
            let written_samples = self
                .resampler
                .filter(
                    self.resample_src_scratch.as_mut_slice(),
                    self.resample_dst_scratch.as_mut_slice(),
                )
                .unwrap() as usize;
            // Copy over the samples to our ring buffer.
            debug_assert_le!(written_samples, self.sample_buffer.capacity());
            self.sample_buffer
                .extend(&self.resample_dst_scratch[..written_samples]);
        } else {
            for _ in 0..FRAMES_PER_BUFFER {
                self.sample_buffer.push_back(self.mixer.next_sample()[0]);
            }
        };

        if self.elapsed_time > 15.0 && self.f_writer.is_some() {
            self.f_writer = None;
            println!("Done!");
        }

        for out_frame in buffer.iter_mut() {
            let sample = self.sample_buffer.pop_front().unwrap();
            //print!("{} ", sample);
            *out_frame = [sample, sample];
        }
        if _now.elapsed().as_micros() as f32 / 1000.0 > 0.1 {
            //println!("Took {} ms", _now.elapsed().as_micros() as f32 / 1000.0);
        }
        pa::Continue
    }
}
