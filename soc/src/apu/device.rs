use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::mixer::SharedAudioRegs;
use super::threads::{self, Resampler};

/// The sampling rate chosen for the device.
pub const DEVICE_RATE: f32 = 48_000.0;

const WANTED_LATENCY: f64 = 1.0 / DEVICE_RATE as f64 * FRAMES_PER_BUFFER as f64;

pub use soundio_backend::*;

// Theoretically to have multiple identical implementations on different sound backends.
mod soundio_backend {
    use libsoundio_sys::*;
    use simple_error::bail;
    use std::error::Error;
    use std::os::raw::{c_char, c_int};

    use super::*;

    #[cfg(target_os = "windows")]
    pub const FRAMES_PER_BUFFER: usize = 2048;
    #[cfg(not(target_os = "windows"))]
    pub const FRAMES_PER_BUFFER: usize = 256;

    const SAMPLE_FORMAT: SoundIoFormat = SoundIoFormat::SoundIoFormatFloat32LE;

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

    pub struct Device {
        ctx: *mut SoundIo,
        device: *mut SoundIoDevice,
        out_stream: *mut SoundIoOutStream,
        _resampler: Box<Resampler>,
        sampler_thread_kill: Arc<AtomicBool>,
    }

    impl Drop for Device {
        fn drop(&mut self) {
            // Disable the stream.
            unsafe { soundio_outstream_destroy(self.out_stream) };
            // Turn off the sampler thread.
            self.sampler_thread_kill.store(true, std::sync::atomic::Ordering::Relaxed);
            // Destroy the device and context.
            unsafe {
                soundio_device_unref(self.device);
                soundio_destroy(self.ctx);
            }
        }
    }

    impl Device {
        pub fn try_new(global_regs: SharedAudioRegs) -> Result<Device, Box<dyn std::error::Error>> {
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
            let device_index = unsafe { soundio_default_output_device_index(ctx) };
            if device_index == -1 {
                bail!("No output devices available.");
            }
            let maybe_device = Device::get_device_if_supported(ctx, device_index).or_else(|| {
                eprintln!(
                    "Default device does not support stereo float32 samples at a {} sample rate. \
                     Will try to open first available device.",
                    DEVICE_RATE
                );
                (0..unsafe { soundio_output_device_count(ctx) })
                    .filter_map(|idx| Device::get_device_if_supported(ctx, idx))
                    .next()
            });
            if maybe_device.is_none() {
                bail!("Could not find any compatible devices.");
            }
            let device = maybe_device.unwrap();
            trace!(
                target: "audio", "Opened device \"{}\".",
                unsafe { c_char_as_str((*device).name) });
            let (mut resampler, sampler_thread_kill) = threads::make_audio_threads(global_regs);
            // Create the output stream.
            let out_stream = Device::create_output_stream(device, resampler.as_mut())?;
            // Start it.
            let err = unsafe { soundio_outstream_start(out_stream) };
            if err != 0 {
                bail!("Could not start output stream: {}", err_as_str(err));
            }
            Ok(Device { ctx, device, out_stream, _resampler: resampler, sampler_thread_kill })
        }

        fn get_device_if_supported(ctx: *mut SoundIo, idx: i32) -> Option<*mut SoundIoDevice> {
            unsafe {
                let device = soundio_get_output_device(ctx, idx);
                assert!(!device.is_null());
                if soundio_device_supports_format(device, SAMPLE_FORMAT) != 0
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
            resampler: &mut Resampler,
        ) -> Result<*mut SoundIoOutStream, Box<dyn Error>> {
            let out_stream = unsafe { soundio_outstream_create(device) };
            if out_stream.is_null() {
                bail!("Could not allocate memory for SoundIoOutStream.");
            }
            let mut out_stream = unsafe { &mut *out_stream };
            // Set the resampler as the shared state.
            out_stream.userdata = resampler as *mut _ as *mut _;
            // Set the stream properties that we want.
            out_stream.format = SAMPLE_FORMAT;
            out_stream.sample_rate = DEVICE_RATE as i32;
            // Request the (very low) latency rate that we want.
            out_stream.software_latency = WANTED_LATENCY;
            out_stream.write_callback = Device::write_callback;
            out_stream.underflow_callback = Some(Device::underflow_callback);
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
                target: "audio", "Created output stream with latency {}ms, sample rate {}.",
                out_stream.software_latency * 1000.0, out_stream.sample_rate);
            Ok(out_stream)
        }

        extern "C" fn write_callback(
            stream: *mut SoundIoOutStream,
            frame_count_min: c_int,
            frame_count_max: c_int,
        ) {
            assert!(!stream.is_null());
            let resampler: &mut Resampler = unsafe { &mut *((*stream).userdata as *mut _) };
            // Begin writing.
            let mut sound_areas = std::ptr::null_mut();
            let mut frame_count =
                (FRAMES_PER_BUFFER as c_int).max(frame_count_min).min(frame_count_max);
            let err = unsafe {
                soundio_outstream_begin_write(stream, &mut sound_areas, &mut frame_count)
            };
            // Handle errors during write begin.
            if err != 0 {
                // Only end streaming if the error was not a simple underflow error.
                if err != SoundIoError::SoundIoErrorUnderflow as c_int {
                    panic!(
                        "Received error from soundio_outstream_begin_write: {}. Ending streaming.",
                        err_as_str(err)
                    );
                } else {
                    trace!(target: "audio", "Underflowed audio.");
                }
                return;
            }
            // Even though libsoundio exposes L and R as two different channels, we know that
            // they are interleaved. So we treat them as interleaved. This might break some
            // esoteric platform, so we assert that the sample size is indeed 8 bytes.
            let sound_area = unsafe { &mut *sound_areas };
            if sound_area.step != 8 {
                panic!(
                    "Left and right audio samples are not interleaved. Sample size is {} bytes. \
                     This is unexpected. Ending streaming.",
                    sound_area.step
                );
            }
            let buffer = unsafe {
                #[allow(clippy::cast_ptr_alignment)]
                std::slice::from_raw_parts_mut(sound_area.ptr as *mut f32, frame_count as usize * 2)
            };
            resampler.stream_callback(buffer);
            let err = unsafe { soundio_outstream_end_write(stream) };
            if err != 0 {
                trace!(target: "audio", "soundio_outstream_end_write error: {}", err_as_str(err));
            }
        }

        extern "C" fn underflow_callback(_stream: *mut SoundIoOutStream) {
            eprintln!("Audio underflowed. Is machine overloaded? If not, please file a bug.");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    /// Make sure we're properly cleaning up after destroying the devices.
    #[test]
    fn stress_test_device_create_destroy() {
        for _ in 0..10 {
            let audio_regs = SharedAudioRegs::default();
            let device = Device::try_new(audio_regs).unwrap();
        }
        // Do it while sleeping in between.
        for _ in 0..10 {
            let audio_regs = SharedAudioRegs::default();
            let device = Device::try_new(audio_regs).unwrap();
            // Sleep for a bit.
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
