use bitfield::bitfield;
use num_derive::FromPrimitive;
use num_traits;

pub enum Addresses {
    InterruptFired = 0xFF0F,
    InterruptEnable = 0xFFFF,
    TimerDiv = 0xFF04,     // DIV
    TimerCounter = 0xFF05, // TIMA
    TimerModulo = 0xFF06,  // TMA
    TimerControl = 0xFF07, // TAC
}

/// Base register trait. Describes registers: their location in memory, etc.
pub trait Register {
    const ADDRESS: usize;
}

#[derive(FromPrimitive, PartialEq, Debug)]
pub enum LcdcModeFlag {
    HBlank,
    VBlank,
    ReadingOAM,
    TransferingToLCD,
}

#[derive(FromPrimitive)]
pub enum TimerFrequency {
    Every1024 = 0, // 4kHz
    Every16 = 1,   // ~262kHz
    Every64 = 2,   // 64kHz
    Every256 = 3,  // 16kHz
}

/// LCD Status Register (STAT). 0xFF41.
bitfield! {
    pub struct LcdStatus([u8]);
    u8;
    pub into LcdcModeFlag, mode, set_mode: 1, 0;
    is_coincidence_flag, set_is_coincidence_flag: 2;
    pub enable_hblank_int, set_enable_hblank_int: 3;
    pub enable_vblank_int, set_vnable_hblank_int: 4;
    pub enable_oam_int, set_enable_oam_int: 5;
    pub enable_coincident_int, set_enable_coincident_int: 6;
}

/// LCD Control Register (LCDC). 0xFF00.
bitfield! {
    pub struct LcdControl([u8]);
    u8;
    enable_bg, _: 0;
    enable_sprites, _: 1;
    sprite_size_select, _: 2;
    pub bg_map_select, _: 3;
    pub bg_set_select, _: 4;
    enable_window, _: 5;
    window_map_select, _: 6;
    // Stopping display must be performed during vblank only.
    pub enable_display, _: 7;
}

/// Interrupt Flag register (IF). 0xFF0F
bitfield! {
    pub struct InterruptFlag([u8]);
    pub has_v_blank, set_v_blank: 0;
    pub has_lcdc, set_lcdc: 1;
    pub has_timer, set_timer: 2;
    pub has_serial_io_complete, set_serial_io_complete: 3;
    pub has_joypad, set_joypad: 4;
}

/// Timer Control register (TAC). 0xFF07
bitfield! {
    pub struct TimerControl([u8]);
    u8;
    pub frequency, set_frequency: 1, 0;
    pub enabled, set_enabled: 2;
}

/// Implements the Register trait.
macro_rules! declare_register {
    ($x:ident, $address:expr) => {
        // Implement the Register trait.
        impl<T> Register for $x<T> {
            const ADDRESS: usize = $address as usize;
        }
    };
}

/// Implements the std::Convert::From<u8> trait.
macro_rules! from_u8 {
    ($x:ident) => {
        // Implement conversion from u8.
        impl core::convert::From<u8> for $x {
            fn from(flag: u8) -> $x { num_traits::FromPrimitive::from_u8(flag).unwrap() }
        }
    };
}

declare_register!(LcdStatus, 0xFF41);
declare_register!(LcdControl, 0xFF00);
declare_register!(InterruptFlag, Addresses::InterruptFired);
declare_register!(TimerControl, Addresses::TimerControl);

from_u8!(LcdcModeFlag);
from_u8!(TimerFrequency);
