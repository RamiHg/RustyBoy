use crate::mmu::MemoryBus;

use bitfield::bitfield;
use num_derive::FromPrimitive;
// use num_traits;

#[derive(FromPrimitive)]
pub enum Addresses {
    InterruptFired = 0xFF0F,
    InterruptEnable = 0xFFFF,
    Joypad = 0xFF00,        // P1
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
    BgPalette = 0xFF47,      // BGP
    SpritePalette0 = 0xFF48, // OBP0
    SpritePalette1 = 0xFF49, // OBP1
    WindowYPos = 0xFF4A,     // WY
    WindowXPos = 0xFF4B,     // WX
}

/// Interrupt Flag register (IF). 0xFF0F
bitfield! {
    pub struct InterruptFlag(i32);
    no default BitRange;
    pub has_v_blank, set_v_blank: 0;
    pub has_lcdc, set_lcdc: 1;
    pub has_timer, set_timer: 2;
    pub has_serial, set_serial: 3;
    pub has_joypad, set_joypad: 4;
}

bitfield! {
    pub struct SerialControl(i32);
    no default BitRange;
    impl Debug;
    u8;
    pub is_transferring, set_transferring: 7;
}

pub trait Register: AsRef<i32> + AsMut<i32> {
    const ADDRESS: i32;

    fn set_bus_or(&mut self, bus: &MemoryBus, or: i32) {
        *self.as_mut() = bus.writes_to(self.address()).unwrap_or(or);
    }

    fn or_bus(&self, bus: &MemoryBus) -> i32 {
        bus.writes_to(self.address()).unwrap_or(*self.as_ref())
    }

    fn set_from_bus(&mut self, bus: &MemoryBus) { self.set_bus_or(bus, *self.as_ref()); }

    fn address(&self) -> i32;
}

macro_rules! impl_bitfield_helpful_traits {
    ($Type:ident) => {
        impl Copy for $Type {}
        impl Clone for $Type {
            fn clone(&self) -> Self { *self }
        }

        impl serde::ser::Serialize for $Type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                i32::from(self.0).serialize(serializer)
            }
        }

        impl<'de> serde::de::Deserialize<'de> for $Type {
            fn deserialize<D>(deserializer: D) -> Result<$Type, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                use std::convert::TryInto as _;
                let inner = i32::deserialize(deserializer)?;
                Ok($Type(inner.try_into().unwrap()))
            }
        }

        impl Default for $Type {
            fn default() -> $Type { $Type(0) }
        }
    };
}

macro_rules! define_common_register {
    ($Type:ident, $address:expr) => {
        impl $crate::io_registers::Register for $Type {
            const ADDRESS: i32 = $address as i32;
            fn address(&self) -> i32 { $Type::ADDRESS }
        }
    };
}

macro_rules! define_typed_register {
    ($Type:ident, $address:expr) => {
        impl bitfield::BitRange<u8> for $Type {
            fn bit_range(&self, msb: usize, lsb: usize) -> u8 {
                (self.0 as u32).bit_range(msb, lsb)
            }
            fn set_bit_range(&mut self, msb: usize, lsb: usize, value: u8) {
                let mut tmp = self.0 as u32;
                tmp.set_bit_range(msb, lsb, value);
                self.0 = tmp as i32;
            }
        }

        impl AsRef<i32> for $Type {
            fn as_ref(&self) -> &i32 { &self.0 }
        }
        impl AsMut<i32> for $Type {
            fn as_mut(&mut self) -> &mut i32 { &mut self.0 }
        }

        impl_bitfield_helpful_traits!($Type);
        define_common_register!($Type, $address);
    };
}

macro_rules! define_int_register {
    ($Type:ident, $address:expr) => {
        // #[derive(Clone, Copy, Debug, PartialEq, Eq, Shrinkwrap, NewtypeAdd, NewtypeAddAssign)]
        // //#[shrinkwrap(mutable)]
        // pub struct $Type(pub i32);

        #[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
        pub struct $Type(pub i32);

        define_common_register!($Type, $address);

        impl std::cmp::PartialOrd<i32> for $Type {
            fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
                Some(self.0.cmp(other))
            }
        }
        impl std::cmp::PartialEq<i32> for $Type {
            fn eq(&self, other: &i32) -> bool { self.0.eq(other) }
        }
        impl std::ops::AddAssign<i32> for $Type {
            fn add_assign(&mut self, rhs: i32) { self.0 += rhs }
        }
        impl std::ops::Mul<i32> for $Type {
            type Output = i32;
            fn mul(self, rhs: i32) -> i32 { self.0 * rhs }
        }
        impl std::ops::Add<i32> for $Type {
            type Output = i32;
            fn add(self, rhs: i32) -> i32 { self.0 + rhs }
        }

        impl AsRef<i32> for $Type {
            fn as_ref(&self) -> &i32 { &self.0 }
        }
        impl AsMut<i32> for $Type {
            fn as_mut(&mut self) -> &mut i32 { &mut self.0 }
        }
        impl std::ops::Deref for $Type {
            type Target = i32;
            fn deref(&self) -> &i32 { &self.0 }
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

define_typed_register!(InterruptFlag, Addresses::InterruptFired);
define_typed_register!(SerialControl, Addresses::SerialControl);
