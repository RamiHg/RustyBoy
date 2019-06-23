use num_traits::FromPrimitive as _;
use num_traits::PrimInt;
use std::cell::Cell;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::io_registers::Addresses;
use crate::mmu;
use registers::*;
use square::*;

mod channels;
mod device;
mod registers;
mod square;

pub const TCYCLE_FREQ: i32 = 4194304;
pub const MCYCLE_FREQ: i32 = 1048576;

pub const SAMPLE_RATE: f32 = 44_100.0;

const SOUND_DOWNSAMPLE: i32 = 1;
pub const BASE_FREQ: i32 = TCYCLE_FREQ / SOUND_DOWNSAMPLE;

pub const LENGTH_COUNTER_PERIOD: i32 = TCYCLE_FREQ / 256 / SOUND_DOWNSAMPLE;
pub const ENVELOPE_PERIOD: i32 = TCYCLE_FREQ / 64 / SOUND_DOWNSAMPLE;
pub const SWEEP_PERIOD: i32 = TCYCLE_FREQ / 128 / SOUND_DOWNSAMPLE;

#[derive(Serialize, Deserialize)]
pub struct Apu {
    #[serde(skip)]
    device: Option<device::Device>,
    #[serde(skip)]
    event_handler: Arc<AtomicU64>,
    square_1_config: SquareConfig,
    square_2_config: SquareConfig,
    wave_config: WaveConfig,
    sound_enable: SoundEnable,

    wave_table: Arc<Cell<u128>>,

    #[serde(skip)]
    channel_state: channels::ChannelState,
}

impl Apu {
    pub fn new() -> Apu {
        let event_handler = Arc::new(AtomicU64::new(0));
        let wave_table = Arc::new(Cell::new(0));
        let maybe_device =
            device::Device::try_new(Arc::clone(&event_handler), Arc::clone(&wave_table));
        if let Err(err) = maybe_device {
            println!(
                "Audio device is not available. Audio will be disabled. Error: {}",
                err
            );
        }
        Apu {
            device: maybe_device.ok(),
            event_handler,
            square_2_config: SquareConfig(0xBF00003F_00_u64),
            square_1_config: SquareConfig(0),
            wave_config: WaveConfig(0),
            sound_enable: SoundEnable(0xF3),
            wave_table,

            channel_state: Default::default(),
        }
    }

    pub fn execute_mcycle(&mut self) {
        if self.square_1_config.triggered() {
            self.trigger_event(channels::EventType::TriggerSquare1, self.square_1_config.0);
            self.square_1_config.set_triggered(false);
        } else if self.square_2_config.triggered() {
            self.trigger_event(channels::EventType::TriggerSquare2, self.square_2_config.0);
            self.square_2_config.set_triggered(false);
        } else if self.wave_config.triggered() {
            self.trigger_event(channels::EventType::TriggerWave, self.wave_config.0);
            self.wave_config.set_triggered(false);
        }
        //self.channel_state.elapsed_ticks(1);
    }

    fn trigger_event(&mut self, event_type: channels::EventType, config: u64) {
        let mut event = channels::ChannelEvent(0);
        event.set_event_type(event_type as u8);
        event.set_payload_low(low_bits(config));
        event.set_payload_high(high_bits(config));
        self.event_handler
            .store(event.0, std::sync::atomic::Ordering::Release);
        self.channel_state.handle_event(event);
    }
}

pub type Timer = std::iter::Rev<std::ops::Range<i32>>;
pub fn timer(count: i32) -> Timer {
    (0..count).rev()
}

#[derive(Clone, Debug)]
pub struct CountdownTimer {
    counter: i32,
    timer: std::iter::Cycle<Timer>,
}

impl Iterator for CountdownTimer {
    type Item = Option<i32>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.counter > 0 {
            Some(if self.timer.next().unwrap() == 0 {
                self.counter -= 1;
                Some(self.counter)
            } else {
                None
            })
        } else {
            None
        }
    }
}

impl CountdownTimer {
    pub fn new(counter: i32, period: i32) -> CountdownTimer {
        CountdownTimer {
            counter,
            timer: timer(period).cycle(),
        }
    }
}
// TODO: Move to util..
pub fn low_bits(x: u64) -> u8 {
    (x & 0xFF) as u8
}

pub fn high_bits(x: u64) -> u32 {
    (x >> 8) as u32
}

fn set_byte<T: PrimInt>(reg: &mut T, i: i32, value: i32) {
    let i = i as usize;
    let mask = T::from(0xFF).unwrap() << (8 * i);
    *reg = (*reg & !mask) | (T::from(value).unwrap() << (8 * i));
}

fn get_byte<T: PrimInt>(reg: T, i: i32) -> i32 {
    use num_traits::cast::NumCast;
    let i = i as usize;
    <i32 as NumCast>::from((reg >> (8 * i)) & T::from(0xFF).unwrap()).unwrap()
}

impl mmu::MemoryMapped for Apu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        use crate::io_registers::Register as _;
        let mmu::Address(_, raw) = address;
        match raw {
            0xFF10..=0xFF14 => Some(get_byte(self.square_1_config.0, raw - 0xFF10)),
            0xFF16..=0xFF19 => Some(get_byte(self.square_2_config.0, raw - 0xFF15)),
            0xFF30..=0xFF3F => Some(get_byte(self.wave_table.get(), raw - 0xFF30)),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        match raw {
            0xFF10..=0xFF14 => Some(set_byte(&mut self.square_1_config.0, raw - 0xFF10, value)),
            0xFF16..=0xFF19 => Some(set_byte(&mut self.square_2_config.0, raw - 0xFF15, value)),
            0xFF30..=0xFF3F => {
                let mut wave_table = self.wave_table.get();
                set_byte(&mut wave_table, raw - 0xFF30, value);
                self.wave_table.set(wave_table);
                Some(())
            }
            0xFF25 => Some(self.sound_enable.0 = value),
            _ => None,
        }
    }
}
