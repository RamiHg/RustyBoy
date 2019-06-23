use portaudio as pa;
use sample;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use super::channels::{ChannelEvent, ChannelState, Frame};

const FRAMES_PER_BUFFER: usize = 64;

pub struct Device {
    pa: pa::PortAudio,
    pa_stream: pa::stream::Stream<pa::stream::NonBlocking, pa::stream::Output<f32>>,
}

impl Device {
    pub fn try_new(event_handler: Arc<AtomicU64>) -> Result<Device, pa::Error> {
        let pa = pa::PortAudio::new()?;
        let settings = pa.default_output_stream_settings::<f32>(
            1,
            super::SAMPLE_RATE as f64,
            FRAMES_PER_BUFFER as u32,
        )?;
        // Create the channel for communicating with the APU.
        let mut thread = AudioThread::new(event_handler);
        let mut pa_stream =
            pa.open_non_blocking_stream(settings, move |args| thread.stream_callback(args))?;
        pa_stream.start()?;
        Ok(Device { pa, pa_stream })
    }
}

struct AudioThread {
    event_handler: Arc<AtomicU64>,
    channel_state: ChannelState,
    last_time: f64,
}

impl AudioThread {
    pub fn new(event_handler: Arc<AtomicU64>) -> AudioThread {
        AudioThread {
            event_handler,
            channel_state: Default::default(),
            last_time: -1.0,
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<f32>,
    ) -> pa::stream::CallbackResult {
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        let elapsed_secs = time.current - self.last_time;
        if self.last_time >= 0.0 && elapsed_secs > 0.0 {
            //self.channel_state.elapsed_secs(elapsed_secs);
        }
        self.last_time = time.current;
        self.handle_events();
        let buffer: &mut [[f32; 1]] = sample::slice::to_frame_slice_mut(buffer).unwrap();
        for out_frame in buffer {
            *out_frame = self.channel_state.next_sample();
        }
        pa::Continue
    }

    fn handle_events(&mut self) {
        use std::sync::atomic::Ordering;
        // In practice it's impossible for the main thread to write more than one event
        // every 4 mcycles (a ton of real CPU cycles). But let's handle the worst case
        // (e.g. audio thread gets suspended).
        let mut current_value = 0;
        loop {
            current_value = self
                .event_handler
                .compare_and_swap(current_value, 0, Ordering::AcqRel);
            if current_value != 0 {
                self.channel_state.handle_event(ChannelEvent(current_value));
            } else {
                break;
            }
        }
    }
}
