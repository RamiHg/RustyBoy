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

const WANTED_LATENCY: f64 = 1.0 / DEVICE_RATE as f64 * FRAMES_PER_BUFFER as f64;

pub use platform::*;

#[cfg(not(feature = "use_soundio"))]
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

#[cfg(feature = "use_soundio")]
mod platform {
    use libsoundio_sys::*;
    use simple_error::bail;
    use spin::Mutex as SpinMutex;
    use std::error::Error;
    use std::os::raw::{c_char, c_int};

    use super::*;

    fn c_char_as_str<'a>(value: *const c_char) -> std::borrow::Cow<'a, str> {
        if !value.is_null() {
            unsafe { std::ffi::CStr::from_ptr(value).to_string_lossy() }
        } else {
            "NULL".into()
        }
    }

    fn err_as_str<'a>(err: c_int) -> std::borrow::Cow<'a, str> {
        unsafe { c_char_as_str(soundio_strerror(err)) }
    }

    /// State that is read by the audio callback. It is absolutely crucial that we do not reference
    /// data in that that thread that has been freed by Rust. We therefore pay a small cost at each
    /// callback to acquire a spin lock.
    struct SharedState {
        pub audio_thread: SpinMutex<Option<AudioThread>>,
    }

    pub struct Device {
        ctx: *mut SoundIo,
        shared_state: Box<SharedState>,
    }

    impl Device {
        pub fn try_new(global_regs: SharedAudioRegs) -> Result<Device, Box<std::error::Error>> {
            let ctx = unsafe {
                let ctx = soundio_create();
                if ctx.is_null() {
                    bail!("Could not create soundio context.");
                }
                let err = soundio_connect(ctx);
                if err != 0 {
                    bail!("Could not connect to any backend: {}", err_as_str(err));
                }
                soundio_flush_events(ctx);
                ctx
            };
            // Try using the default output device. If that fails, or if the default output device
            // does not support what we want, try all in sequence.
            let maybe_device_idx = unsafe { soundio_default_output_device_index(ctx) };
            if maybe_device_idx == -1 {
                bail!("No output devices available.");
            }
            let mut maybe_device = Device::get_device_if_supported(ctx, maybe_device_idx);
            if maybe_device.is_none() {
                eprintln!(
                    "Default device does not support stereo float32 samples at a {} sample rate. \
                     Will try to open first available device.",
                    DEVICE_RATE
                );
                maybe_device = (0..unsafe { soundio_output_device_count(ctx) })
                    .filter_map(|idx| Device::get_device_if_supported(ctx, idx))
                    .next();
                if maybe_device.is_none() {
                    bail!("Could not find any compatible devices.");
                }
            }
            let device = maybe_device.unwrap();
            trace!(
                target: "audio", "Opened device \"{}\".",
                unsafe { c_char_as_str((*device).name) });
            // Create the shared state.
            let mut shared_state = Box::new(SharedState {
                audio_thread: SpinMutex::new(Some(AudioThread::new(global_regs))),
            });
            let out_stream = Device::create_output_stream(device, shared_state.as_mut())?;
            // Start it.
            let err = unsafe { soundio_outstream_start(out_stream) };
            if err != 0 {
                bail!("Could not start output stream: {}", err_as_str(err));
            }

            // Ok(Device { output_stream })
            Ok(Device { ctx, shared_state })
        }

        fn get_device_if_supported(ctx: *mut SoundIo, idx: i32) -> Option<*mut SoundIoDevice> {
            unsafe {
                let device = soundio_get_output_device(ctx, idx);
                assert!(!device.is_null());
                if soundio_device_supports_format(device, SoundIoFormat::SoundIoFormatFloat32LE)
                    != 0
                    && soundio_device_supports_layout(
                        device,
                        soundio_channel_layout_get_builtin(
                            SoundIoChannelLayoutId::SoundIoChannelLayoutIdStereo as i32,
                        ),
                    ) != 0
                    && soundio_device_supports_sample_rate(device, DEVICE_RATE as i32) != 0
                {
                    Some(device)
                } else {
                    soundio_device_unref(device);
                    None
                }
            }
        }

        fn create_output_stream(
            device: *mut SoundIoDevice,
            shared_state: &mut SharedState,
        ) -> Result<*mut SoundIoOutStream, Box<Error>> {
            let out_stream = unsafe { soundio_outstream_create(device) };
            if out_stream.is_null() {
                bail!("Could not allocate memory for SoundIoOutStream.");
            }
            let mut out_stream = unsafe { &mut *out_stream };
            // Set the user data as the shared state.
            out_stream.userdata = shared_state as *mut _ as *mut _;
            // Set the stream properties that we want.
            out_stream.format = SoundIoFormat::SoundIoFormatFloat32LE;
            out_stream.sample_rate = DEVICE_RATE as i32;
            // Request the (very low) latency rate that we want.
            out_stream.software_latency = WANTED_LATENCY;
            out_stream.write_callback = Device::write_callback;
            let err = unsafe { soundio_outstream_open(out_stream) };
            if err != 0 {
                unsafe { soundio_outstream_destroy(out_stream) };
                bail!("Could not open SoundIoOutStream: {}", err_as_str(err));
            }
            if out_stream.layout.channel_count != 2 {
                unsafe { soundio_outstream_destroy(out_stream) };
                bail!(
                    "Unexepected channel count. Expected stereo (2), got {}",
                    out_stream.layout.channel_count
                );
            }
            trace!(
                target: "audio", "Created output stream with latency {}, sample rate {}.",
                out_stream.software_latency, out_stream.sample_rate);
            Ok(out_stream)
        }

        extern "C" fn write_callback(
            stream: *mut SoundIoOutStream,
            frame_count_min: c_int,
            frame_count_max: c_int,
        ) {
            assert!(!stream.is_null());
            let shared_state: &mut SharedState = unsafe { &mut *((*stream).userdata as *mut _) };
            let mut maybe_audio_thread = shared_state.audio_thread.lock();
            if maybe_audio_thread.is_none() {
                // This is a signal that execution should end.
                return;
            }
            let audio_thread = maybe_audio_thread.as_mut().unwrap();
            // Begin writing.
            let mut sound_areas = std::ptr::null_mut();
            let mut frame_count: c_int = FRAMES_PER_BUFFER as c_int;
            let err = unsafe {
                soundio_outstream_begin_write(stream, &mut sound_areas, &mut frame_count)
            };
            // Handle errors during write begin.
            if err != 0 {
                // Only end streaming if the error was not a simple underflow error.
                if err != SoundIoError::SoundIoErrorUnderflow as c_int {
                    eprintln!(
                        "Received error from soundio_outstream_begin_write: {}. Ending streaming.",
                        err_as_str(err)
                    );
                    *maybe_audio_thread = None;
                } else {
                    trace!(target: "audio", "Underflowed audio.");
                }
                trace!(target: "audio", "soundio_outstream_begin_write error: {}", err_as_str(err));
                return;
            }
            // Even though libsoundio exposes L and R as two different channels, we know that
            // they are interleaved. So we treat them as interleaved. This might break some
            // esoteric platform, so we assert that the sample size is indeed 8 bytes.
            let sound_area = unsafe { &*sound_areas };
            if sound_area.step != 8 {
                eprintln!(
                    "Left and right audio samples are not interleaved. Sample size is {} bytes. \
                     This is unexpected. Ending streaming.",
                    sound_area.step
                );
                *maybe_audio_thread = None;
                return;
            }
            let buffer = unsafe {
                #[allow(clippy::cast_ptr_alignment)]
                std::slice::from_raw_parts_mut(sound_area.ptr as *mut f32, frame_count as usize * 2)
            };
            audio_thread.stream_callback(buffer);
            // Ignoring errors from end write, since we don't call begin_write this callback again.
            unsafe { soundio_outstream_end_write(stream) };
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
        assert_le!(data.output_frames_gen as usize, self.resample_dst_scratch.len());
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
