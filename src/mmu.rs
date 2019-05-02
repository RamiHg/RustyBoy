use crate::{io_registers::Register, util};

use crate::error::{self, Result};
use crate::io_registers;
use crate::system::Interrupts;

// Useful tidbits:
// https://retrocomputing.stackexchange.com/questions/1178/what-is-this-unused-memory-range-in-the-game-boys-memory-map

#[derive(Clone, Copy, Debug)]
pub enum Location {
    // 0000 to 8000. Mbc ROM.
    MbcRom,
    // 8000 to A000. Video RAM.
    VRam,
    // 0xA000 to 0xC000. Mbc switchable RAM.
    MbcRam,
    // C000 to E000 (and echo implicitly handled). Internal RAM.
    InternalRam,
    // FE00 to FEA0. Sprite Attrib Memory.
    OAM,
    // FEA0 to FF00. Unused OAM memory.
    UnusedOAM,
    // FF00 to FF4C. Also covers FFFF (IE register).
    Registers,
    // FF4C to FF80. Unknown registers.
    UnknownRegisters,
    // FF80 to FFFF. Internal (High) RAM.
    HighRam,
}

#[derive(Clone, Copy, Debug)]
pub struct Address(pub Location, pub i32);

impl Address {
    pub fn from_raw(raw: i32) -> Result<Address> {
        debug_assert!(util::is_16bit(raw));
        use Location::*;
        match raw {
            0x0000...0x7FFF => Ok(Address(MbcRom, raw)),
            0x8000...0x9FFF => Ok(Address(VRam, raw)),
            0xA000...0xBFFF => Ok(Address(MbcRam, raw)),
            0xC000...0xDFFF => Ok(Address(InternalRam, raw)),
            0xE000...0xFDFF => Ok(Address(InternalRam, raw - 0x2000)),
            0xFE00...0xFE9F => Ok(Address(OAM, raw)),
            0xFEA0...0xFEFF => Ok(Address(UnusedOAM, raw)),
            (0xFF00...0xFF4B) | 0xFFFF => Ok(Address(Registers, raw)),
            0xFF4C...0xFF7F => Ok(Address(UnknownRegisters, raw)),
            0xFF80...0xFFFE => Ok(Address(HighRam, raw)),
            _ => Err(error::Type::InvalidOperation(format!(
                "Address {:X?} is invalid.",
                raw
            ))),
        }
    }
}

pub trait MemoryMapped {
    //fn handles(&self, address: Address) -> bool;
    fn read(&self, address: Address) -> Option<i32>;
    fn write(&mut self, address: Address, value: i32) -> Option<()>;
}

#[derive(Debug)]
pub struct MemoryBus {
    pub t_state: i32,

    pub read_latch: bool,
    pub write_latch: bool,
    pub address_latch: i32,

    pub data_latch: i32,
}

impl MemoryBus {
    pub fn writes_to(&self, address: i32) -> Option<i32> {
        if self.t_state == 4 && self.write_latch && self.address_latch == address {
            Some(self.data_latch)
        } else {
            None
        }
    }
}

pub trait MemoryMapped2 {
    fn default_next_state(&self, bus: &MemoryBus) -> (Box<Self>);
    fn execute_tcycle(self: Box<Self>, memory_bus: &MemoryBus) -> (Box<Self>, Interrupts);

    fn read_register(&self, address: io_registers::Addresses) -> Option<i32>;
    fn write_register(&mut self, address: io_registers::Addresses, value: i32) -> Option<()>;
}

/// Holds the internal RAM, as well as register values that don't need to be managed by their
/// components directly.
pub struct Memory {
    mem: [u8; 0x10000],
}

impl MemoryMapped for Memory {
    fn read(&self, address: Address) -> Option<i32> {
        let Address(location, raw) = address;
        use Location::*;
        match location {
            VRam | InternalRam | OAM | Registers | HighRam => Some(self.mem[raw as usize].into()),
            UnusedOAM => Some(0),
            UnknownRegisters => Some(0xFF),
            _ => None,
        }
    }

    fn write(&mut self, address: Address, value: i32) -> Option<()> {
        debug_assert!(util::is_8bit(value));
        let Address(location, raw) = address;
        use Location::*;
        match location {
            VRam | InternalRam | OAM | Registers | HighRam | UnusedOAM => {
                // if raw == 0xFFFF {
                //     println!("Setting IE to {:b}", value);
                // }
                self.mem[raw as usize] = value as u8;
                Some(())
            }
            UnknownRegisters => Some(()),
            // UnusedOAM => Some(()),
            _ => None,
        }
    }
}

#[cfg(test)]
impl Memory {
    pub fn raw_read(&self, addr: i32) -> i32 { self.mem[addr as usize] as i32 }
}

impl Memory {
    pub fn new() -> Memory { Memory { mem: [0; 0x10000] } }

    /// Temporary while the entire system moves away form direct memory access.
    pub fn read(&self, raw_address: i32) -> i32 {
        let address = Address::from_raw(raw_address).unwrap();
        MemoryMapped::read(self, address).unwrap()
    }
    pub fn store(&mut self, raw_address: i32, value: i32) {
        let address = Address::from_raw(raw_address).unwrap();
        MemoryMapped::write(self, address, value).unwrap();
    }

    pub fn get_mut_8(&mut self, raw_address: i32) -> &mut u8 { &mut self.mem[raw_address as usize] }

    pub fn get_mut_register<'a, A, T>(&mut self, cons: T) -> A
    where
        A: Register,
        T: core::ops::FnOnce(&'a mut [u8]) -> A,
    {
        // I need to be able to return multiple mutable references to different registers when I
        // KNOW that they point to different locations in memory. Therefore, the hacky unsafe.
        // Let me know if you can think of a better way!
        cons(unsafe { std::slice::from_raw_parts_mut(self.get_mut_8(A::ADDRESS), 1) })
    }
}
