use portaudio as pa;
use sample;

use super::channels::{ChannelMixer, SharedAudioRegs};

const FRAMES_PER_BUFFER: usize = 32;
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

use sdr::fir;

const PRETEND_SAMPLE_RATE: f32 = 4400_000.0;
const PRETEND_NEED_RATE: f32 = 48_000.0;

struct AudioThread {
    mixer: ChannelMixer,
    last_time: f64,
    filter: fir::FIR<f32>,
    filter2: fir::FIR<f32>,
}

impl AudioThread {
    pub fn new(audio_regs: SharedAudioRegs) -> AudioThread {
        let mixer = ChannelMixer::new(audio_regs);
        AudioThread {
            mixer,
            last_time: -1.0,
            //filter: fir::FIR::resampler(4, 524_288, 11_025),
            filter: fir::FIR::resampler(46, 32, 1),
            filter2: fir::FIR::resampler(375 * 2, 512, 375),
        }
    }

    pub fn stream_callback(
        &mut self,
        args: pa::OutputStreamCallbackArgs<PaOutType>,
    ) -> pa::stream::CallbackResult {
        let _now = std::time::Instant::now();
        let pa::OutputStreamCallbackArgs { buffer, time, .. } = args;
        let elapsed_secs = time.current - self.last_time;
        self.last_time = time.current;
        self.mixer.handle_events();
        let buffer: &mut [[PaOutType; 2]] = sample::slice::to_frame_slice_mut(buffer).unwrap();

        //let need = ((super::BASE_FREQ as f32 / super::SAMPLE_RATE as f32) * 64.0) as i32;
        let need = 87 * FRAMES_PER_BUFFER;
        let bufs = (0..1398)
            .map(|_| {
                self.mixer.next_sample();
                self.mixer.next_sample()[0]
            })
            .collect::<Vec<f32>>();
        let mut res = self.filter2.process(&self.filter.process(&bufs));
        let last = *res.last().unwrap();

        for out_frame in buffer.iter_mut() {
            //let sample = self.mixer.next_sample();
            let sample = if res.is_empty() {
                //print!("bad ");
                last
            } else {
                res.remove(0)
            };
            //print!("{} ", sample);
            *out_frame = [sample, sample];
        }
        //println!("left {}", res.len());
        if _now.elapsed().as_micros() as f32 / 1000.0 > 0.1 {
            //println!("Took {} ms", _now.elapsed().as_micros() as f32 / 1000.0);
        }
        pa::Continue
    }

}
