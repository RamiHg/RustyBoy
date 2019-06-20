use std::sync::mpsc;

use portaudio as pa;
use sample;

use super::FrameType;

const FRAMES_PER_BUFFER: usize = 64;

pub struct Device {
    pa: pa::PortAudio,
    pa_stream: pa::stream::Stream<pa::stream::NonBlocking, pa::stream::Output<FrameType>>,

    pub tx: mpsc::Sender<FrameType>,
}

impl Default for Device {
    fn default() -> Device {
        let pa = pa::PortAudio::new()
            .unwrap_or_else(|e| panic!("Error while attempting to create audio device: {}", e));
        let settings = pa
            .default_output_stream_settings::<FrameType>(
                1,
                super::SAMPLE_RATE as f64,
                FRAMES_PER_BUFFER as u32,
            )
            .unwrap();
        // Create the channel for communicating with the APU.
        let (tx, rx) = mpsc::channel();
        let mut stream = Stream::new(rx);
        let mut pa_stream = pa
            .open_non_blocking_stream(settings, move |args| stream.stream_callback(args))
            .unwrap();
        pa_stream.start().unwrap();
        Device { pa, pa_stream, tx }
    }
}

struct Stream {
    rx: mpsc::Receiver<FrameType>,
    sample_buf: sample::ring_buffer::Bounded<Box<[FrameType]>>,
}

impl Stream {
    pub fn new(rx: mpsc::Receiver<FrameType>) -> Stream {
        Stream {
            rx,
            sample_buf: sample::ring_buffer::Bounded::boxed_slice(4096 * 100),
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<f32>,
    ) -> pa::stream::CallbackResult {
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        let buffer: &mut [[f32; 1]] = sample::slice::to_frame_slice_mut(buffer).unwrap();

        //while self.sample_buf.len() < FRAMES_PER_BUFFER {
        for sample in self.rx.try_iter() {
            //assert!(self.sample_buf.push(sample).is_none());
            self.sample_buf.push(sample);
        }
        //}
        if self.sample_buf.len() >= FRAMES_PER_BUFFER * 4 {
            for out_frame in buffer {
                out_frame[0] = self.sample_buf.pop().unwrap();
            }
        } else {
            for out_frame in buffer {
                out_frame[0] = 0.0;
            }
        }

        pa::Continue
    }
}
