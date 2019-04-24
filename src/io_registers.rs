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
    Dma = 0xFF46,            // DMA
    BgPallette = 0xFF47,     // BGP
    SpritePalette0 = 0xFF48, // OBP0
    SpritePalette1 = 0xFF49, // OBP1
    WindowYPos = 0xFF4A,     // WY
    WindowXPos = 0xFF4B,     // WX
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

bitfield! {
    pub struct SerialControl(u8);
    impl Debug;
    u8;
    pub is_transferring, set_transferring: 7;
}

pub struct MemoryBus {}

pub trait Register: AsRef<u32> + AsMut<u32> {
    const ADDRESS: i32;

    fn set_bus_or(&mut self, bus: &MemoryBus, or: i32) {
        // TODO
        *self.as_mut() = or as u32;
    }

    fn or_bus(&self, bus: &MemoryBus) -> i32 { 0 }

    fn set_bus(&mut self, bus: &MemoryBus) { *self.as_mut() = 1;// todo }
}

#[macro_export]
macro_rules! define_typed_register {
    ($Type:ident, $address:expr) => {
        use crate::io_registers::Register;

        impl AsRef<u32> for $Type {
            fn as_ref(&self) -> &u32 { &self.0 }
        }
        impl AsMut<u32> for $Type {
            fn as_mut(&mut self) -> &mut u32 { &mut self.0 }
        }

        impl Register for $Type {
            const ADDRESS: i32 = $address as i32;
        }
    };
}


#[macro_export]
macro_rules! declare_register_u8 {
    ($x:ident, $address:expr) => {
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
declare_register_u8!(SerialControl, Addresses::SerialControl);

