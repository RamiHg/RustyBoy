use num_traits::FromPrimitive as _;
use std::sync::mpsc;

use crate::io_registers::Addresses;
use crate::mmu;
use registers::*;
use square::*;

mod channels;
mod device;
mod registers;
mod square;

const MCYCLE_FREQ: i32 = 1048576;

pub const SAMPLE_RATE: f32 = 44100.0;
pub const LENGTH_COUNTER_PERIOD: i32 = MCYCLE_FREQ / 256; // 4096.

pub type FrameType = f32;

const MCYCLES_PER_SAMPLE: i32 = 24; // 1mhz to 44.1khz.

#[derive(Serialize, Deserialize)]
pub struct Apu {
    #[serde(skip)]
    device: device::Device,
    sample_timer: Timer,

    square_2_config: SquareConfig,
    sound_enable: SoundEnable,

    #[serde(skip)]
    channel_state: channels::ChannelState,
}

impl Apu {
    pub fn new() -> Apu {
        let device = device::Device::default();
        let tx = device.tx.clone();
        Apu {
            device,
            square_2_config: SquareConfig(0xBF00003F_u32),
            sound_enable: SoundEnable(0xF3),
            sample_timer: Timer::new(1, MCYCLES_PER_SAMPLE),

            channel_state: Default::default(),
        }
    }

    pub fn execute_mcycle(&mut self) {
        // if self.square_2_config.triggered() {
        //     self.channel_state
        //         .handle_event(channels::ChannelEvent::TriggerSquare2(
        //             self.square_2_config.0,
        //         ));
        //     self.square_2_config.set_triggered(false);
        // }
        self.channel_state.elapsed_ticks(1);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Timer {
    tick: i32,
    initial: i32,
}

impl Timer {
    pub fn new(counter: i32, period: i32) -> Timer {
        Timer {
            tick: counter * period,
            initial: counter * period,
        }
    }

    pub fn done(&self) -> bool {
        self.tick <= 0
    }

    pub fn restart(&mut self) {
        debug_assert_le!(self.tick, 0);
        self.tick += self.initial;
        debug_assert_gt!(self.tick, 0);
    }

    pub fn tick(&mut self, ticks: i32) {
        self.tick -= ticks;
    }
}

fn set_byte(reg: &mut u32, i: usize, value: i32) {
    let mut bytes = reg.to_le_bytes();
    bytes[i] = value as u8;
    *reg = u32::from_le_bytes(bytes);
}

fn get_byte(reg: u32, i: usize) -> i32 {
    reg.to_le_bytes()[i] as i32
}

impl mmu::MemoryMapped for Apu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        use crate::io_registers::Register as _;
        let mmu::Address(_, raw) = address;
        match Addresses::from_i32(raw) {
            Some(Addresses::NR21) => Some(get_byte(self.square_2_config.0, 0)),
            Some(Addresses::NR22) => Some(get_byte(self.square_2_config.0, 1)),
            Some(Addresses::NR23) => Some(get_byte(self.square_2_config.0, 2)),
            Some(Addresses::NR24) => Some(get_byte(self.square_2_config.0, 3)),
            Some(Addresses::NR51) => Some(self.sound_enable.0),
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw) = address;
        match Addresses::from_i32(raw) {
            // Square 2 config.
            Some(Addresses::NR21) => {
                set_byte(&mut self.square_2_config.0, 0, value);
                Some(())
            }
            Some(Addresses::NR22) => {
                set_byte(&mut self.square_2_config.0, 1, value);
                Some(())
            }
            Some(Addresses::NR23) => {
                set_byte(&mut self.square_2_config.0, 2, value);
                Some(())
            }
            Some(Addresses::NR24) => {
                set_byte(&mut self.square_2_config.0, 3, value);
                Some(())
            }
            // Sound enable.
            Some(Addresses::NR51) => {
                self.sound_enable.0 = value;
                Some(())
            }
            _ => None,
        }
    }
}
