use super::Timer;
use crate::apu::registers;

const DUTIES: [[f32; 8]; 4] = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0],
];

pub struct SquareWave {
    pub signal: Box<dyn sample::Signal<Frame = sample::frame::Mono<f32>>>,
    pub timer: Timer,
    pub reset_on_done: bool,
}

impl SquareWave {
    pub fn new(config: registers::SquareConfig) -> SquareWave {
        SquareWave {
            signal: Box::new(SquareWave::signal(
                config.duty() as i32,
                config.freq() as i32,
            )),
            timer: Timer::new(64 - config.length() as i32, super::LENGTH_COUNTER_PERIOD),
            reset_on_done: config.is_timed(),
        }
    }

    pub fn get_sample(&mut self) -> f32 {
        self.signal.next()[0] - 0.5
    }

    fn signal(
        duty_type: i32,
        freq_setting: i32,
    ) -> impl sample::Signal<Frame = sample::frame::Mono<f32>> {
        use sample::signal::Signal as _;
        let duties = &DUTIES[duty_type as usize];
        let samples = duties.iter().map(|x| [*x]).cycle();
        let mut source = sample::signal::from_iter(samples);
        let sampled_frequency = 4194304.0 / ((2048.0 - freq_setting as f32) * 4.0);
        let interp = sample::interpolate::Floor::from_source(&mut source);
        //let interp =
        //     sample::interpolate::Sinc::new(sample::ring_buffer::Fixed::from(vec![[0.0]; 64]));
        source.from_hz_to_hz(interp, sampled_frequency.into(), super::SAMPLE_RATE.into())
    }
}
