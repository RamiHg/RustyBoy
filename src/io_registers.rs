use bitfield::bitfield;
use num_derive::FromPrimitive;
use num_traits;

#[derive(FromPrimitive)]
pub enum Addresses {
    InterruptFired = 0xFF0F,
    InterruptEnable = 0xFFFF,
    TimerDiv = 0xFF04,      // DIV
    TimerCounter = 0xFF05,  // TIMA
    TimerModulo = 0xFF06,   // TMA
    TimerControl = 0xFF07,  // TAC
    SerialData = 0xFF01,    // SB
    SerialControl = 0xFF02, // SC
    // GPU Registers.
    LcdControl = 0xFF40,     // LCDC
    LcdStatus = 0xFF41,      // STAT
    ScrollY = 0xFF42,        // SCY
    ScrollX = 0xFF43,        // SCX
    LcdY = 0xFF44,           // LY
    LcdYCompare = 0xFF45,    // LYC
    BgPallette = 0xFF47,     // BGP
    SpritePalette0 = 0xFF48, // OBP0
    SpritePalette1 = 0xFF49, // OBP1
    WindowYPos = 0xFF4A,     // WY
    WindowXPos = 0xFF4B,     // WX
}

/// Base register trait. Describes registers: their location in memory, etc.
pub trait Register {
    const ADDRESS: i32;
}

#[derive(Clone, Copy, Debug, FromPrimitive)]
pub enum TimerFrequency {
    Every1024 = 0, // 4kHz
    Every16 = 1,   // ~262kHz
    Every64 = 2,   // 64kHz
    Every256 = 3,  // 16kHz
}

/// Interrupt Flag register (IF). 0xFF0F
bitfield! {
    pub struct InterruptFlag(u8);
    pub has_v_blank, set_v_blank: 0;
    pub has_lcdc, set_lcdc: 1;
    pub has_timer, set_timer: 2;
    pub has_serial, set_serial: 3;
    pub has_joypad, set_joypad: 4;
}

/// Timer Control register (TAC). 0xFF07
bitfield! {
    pub struct TimerControl(u8);
    impl Debug;
    u8;
    pub into TimerFrequency, frequency, set_frequency: 1, 0;
    pub enabled, set_enabled: 2;
}

bitfield! {
    pub struct SerialControl(u8);
    impl Debug;
    u8;
    pub is_transferring, set_transferring: 7;
}

/// Implements the Register trait.
#[macro_export]
macro_rules! declare_register {
    ($x:ident, $address:expr) => {
        // Implement the Register trait.
        impl<T> crate::io_registers::Register for $x<T> {
            const ADDRESS: i32 = $address as i32;
        }
    };
}

#[macro_export]
macro_rules! declare_register_u8 {
    ($x:ident, $address:expr) => {
        // Implement the Register trait.
        impl crate::io_registers::Register for $x {
            const ADDRESS: i32 = $address as i32;
        }

        impl Clone for $x {
            fn clone(&self) -> Self { *self }
        }
        impl Copy for $x {}
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

declare_register_u8!(InterruptFlag, Addresses::InterruptFired);
declare_register_u8!(TimerControl, Addresses::TimerControl);
declare_register_u8!(SerialControl, Addresses::SerialControl);

from_u8!(TimerFrequency);
