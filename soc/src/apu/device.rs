use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::mixer::SharedAudioRegs;
use super::threads;

/// The sampling rate chosen for the device.
pub const DEVICE_RATE: f32 = 48_000.0;

#[cfg(target_os = "windows")]
pub const FRAMES_PER_BUFFER: usize = 2048;
#[cfg(not(target_os = "windows"))]
pub const FRAMES_PER_BUFFER: usize = 256;

pub use audiohal_backend::*;

mod audiohal_backend {
    extern crate audiohal;

    use super::*;

    pub struct Device {
        outstream: Option<audiohal::Stream<[f32; 2]>>,
        sampler_thread_kill: Arc<AtomicBool>,
    }

    impl Drop for Device {
        fn drop(&mut self) {
            // Turn off the sampler thread.
            self.sampler_thread_kill.store(true, std::sync::atomic::Ordering::Relaxed);
            self.outstream = None;
        }
    }

    impl Device {
        pub fn try_new(global_regs: SharedAudioRegs) -> Result<Device, Box<dyn std::error::Error>> {
            let (mut resampler, sampler_thread_kill) = threads::make_audio_threads(global_regs);

            let mut stream = audiohal::Host::with_default_backend()?
                .default_output_device()?
                .open_outstream(audiohal::StreamOptions {
                    sample_rate: audiohal::SampleRate::Exact(DEVICE_RATE as i32),
                    frames_per_buffer: Some(FRAMES_PER_BUFFER as i32),
                    callback: Box::new(move |x| resampler.stream_callback(x)),
                    ..Default::default()
                })?;
            stream.start()?;

            return Ok(Device { outstream: Some(stream), sampler_thread_kill });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    /// Make sure we're properly cleaning up after destroying the devices.
    #[test]
    fn stress_test_device_create_destroy() {
        // TODO: Cleanup. If no audio devices, don't bother with stress test.
        {
            if Device::try_new(SharedAudioRegs::default()).is_err() {
                return;
            }
        }
        for _ in 0..10 {
            let audio_regs = SharedAudioRegs::default();
            let _device = Device::try_new(audio_regs).unwrap();
        }
        // Do it while sleeping in between.
        for _ in 0..10 {
            let audio_regs = SharedAudioRegs::default();
            let _device = Device::try_new(audio_regs).unwrap();
            // Sleep for a bit.
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
