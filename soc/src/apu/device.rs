use portaudio as pa;
use sample;
use std::sync::Arc;

use super::channels::{ChannelMixer, ChannelState};

const FRAMES_PER_BUFFER: usize = 32;

#[allow(dead_code)]
pub struct Device {
    pa: pa::PortAudio,
    pa_stream: pa::stream::Stream<pa::stream::NonBlocking, pa::stream::Output<f32>>,
}

impl Device {
    pub fn try_new(channel_state: ChannelState) -> Result<Device, pa::Error> {
        let pa = pa::PortAudio::new()?;
        let settings = pa.default_output_stream_settings::<f32>(
            1,
            super::SAMPLE_RATE as f64,
            FRAMES_PER_BUFFER as u32,
        )?;
        // Create the channel for communicating with the APU.
        let mut thread = AudioThread::new(channel_state);
        let mut pa_stream =
            pa.open_non_blocking_stream(settings, move |args| thread.stream_callback(args))?;
        pa_stream.start()?;
        Ok(Device { pa, pa_stream })
    }
}

struct AudioThread {
    channel_state: ChannelState,
    mixer: ChannelMixer,
    last_time: f64,
}

impl AudioThread {
    pub fn new(channel_state: ChannelState) -> AudioThread {
        let mixer = ChannelMixer::new(Arc::clone(&channel_state.wave_table));
        AudioThread {
            channel_state,
            mixer,
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
            *out_frame = self.mixer.next_sample();
        }
        pa::Continue
    }

    fn handle_events(&mut self) {
        self.channel_state
            .poll_events()
            .into_iter()
            .for_each(|x| self.mixer.handle_event(x));
    }
}
