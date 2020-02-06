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

pub use soundio2_backend::*;

mod soundio2_backend {
    extern crate soundio;

    use std::rc::Rc;

    use super::*;

    trait DeviceTrait {}

    pub struct Device {
        outstream: Option<Rc<soundio::OutStream<[f32; 2]>>>,
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
            let context = soundio::Context::new()?;
            let device = context.default_output_device()?;

            let (mut resampler, sampler_thread_kill) = threads::make_audio_threads(global_regs);

            let stream = device.open_outstream(
                soundio::StreamOptions::<[f32; 2]> {
                    sample_rate: soundio::SampleRate::NearestTo(DEVICE_RATE as i32),
                    desired_frames_per_buffer: Some(FRAMES_PER_BUFFER as i32),
                    ..Default::default()
                },
                Box::new(move |x| resampler.stream_callback(x)),
            )?;
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
