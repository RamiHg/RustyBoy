use portaudio as pa;
use sample;

use super::channels::{ChannelMixer, SharedAudioRegs};

const FRAMES_PER_BUFFER: usize = 64;
const NUM_CHANNELS: i32 = 2;

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
            super::SAMPLE_RATE as f64,
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

    writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
    num: i32,
}

impl AudioThread {
    pub fn new(audio_regs: SharedAudioRegs) -> AudioThread {
        let mixer = ChannelMixer::new(audio_regs);
        AudioThread {
            mixer,
            last_time: -1.0,
            writer: Some(
                hound::WavWriter::create(
                    "sound.wav",
                    hound::WavSpec {
                        channels: 2,
                        sample_rate: super::SAMPLE_RATE as u32,
                        bits_per_sample: 16,
                        sample_format: hound::SampleFormat::Int,
                    },
                )
                .unwrap(),
            ),
            num: 0,
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<PaOutType>,
    ) -> pa::stream::CallbackResult {
        self.writer = None;

        let _now = std::time::Instant::now();
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        let elapsed_secs = time.current - self.last_time;
        if self.last_time >= 0.0 && elapsed_secs > 0.0 {
            //self.channel_state.elapsed_secs(elapsed_secs);
        }
        self.last_time = time.current;
        self.mixer.handle_events();
        let buffer: &mut [[PaOutType; 2]] = sample::slice::to_frame_slice_mut(buffer).unwrap();
        for out_frame in buffer {
            let sample = self.mixer.next_sample();
            if let Some(writer) = &mut self.writer {
                let conv = |x| (x * std::i16::MAX as f32) as i16;
                writer.write_sample(conv(sample[0])).unwrap();
                writer.write_sample(conv(sample[1])).unwrap();
                self.num += 1;
                if self.num > 44100 * 10 {
                    println!("Finished");
                    writer.flush().unwrap();
                    self.writer = None;
                }

            }
            *out_frame = sample;
        }
        if _now.elapsed().as_micros() as f32 / 1000.0 > 0.1 {
            // println!("Took {} ms", _now.elapsed().as_micros() as f32 / 1000.0);
        }
        pa::Continue
    }

}
