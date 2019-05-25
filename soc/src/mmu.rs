use crate::util;

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

    pub fn writes_to_reg(&self, reg: impl io_registers::Register) -> bool {
        self.writes_to(reg.address()).is_some()
    }

    pub fn reads_from(&self, reg: impl io_registers::Register) -> bool {
        reg.address() == self.address_latch
    }

    pub fn maybe_read(&mut self, reg: impl io_registers::Register) {
        if self.read_latch && self.reads_from(reg) {
            self.data_latch = reg.value();
        }
    }
}

pub trait MemoryMapped2 {
    fn execute_tcycle(self: Box<Self>, memory_bus: &MemoryBus) -> (Box<Self>, Interrupts);
}

/// Holds the internal RAM, as well as register values that don't need to be managed by their
/// components directly.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Memory {
    mem: Vec<u8>,
}

impl MemoryMapped for Memory {
    fn read(&self, address: Address) -> Option<i32> {
        let Address(location, raw) = address;
        use Location::*;
        match location {
            Registers if raw == io_registers::Addresses::InterruptFired as i32 => {
                let flag = self.mem[raw as usize] as i32;
                Some((flag & 0x1F) | 0xE0)
            }
            Registers
                if raw == io_registers::Addresses::LcdControl as i32
                    || raw == io_registers::Addresses::LcdStatus as i32
                    || raw == io_registers::Addresses::LcdY as i32 =>
            {
                None
            }
            InternalRam | Registers | HighRam => Some(self.mem[raw as usize].into()),
            UnusedOAM => panic!(),
            UnknownRegisters => Some(0xFF),
            _ => None,
        }
    }

    fn write(&mut self, address: Address, value: i32) -> Option<()> {
        debug_assert!(util::is_8bit(value));
        let Address(location, raw) = address;
        use Location::*;
        match location {
            Registers if raw == io_registers::Addresses::InterruptEnable as i32 => {
                self.mem[raw as usize] = value as u8;
                Some(())
            }
            // Registers
            //     if raw == io_registers::Addresses::LcdControl
            //         || raw == io_registers::Addresses::LcdStatus
            //         || raw == io_registers::Addresses::LcdY =>
            // {
            //     None
            // }
            InternalRam | Registers | HighRam => {
                self.mem[raw as usize] = value as u8;
                Some(())
            }
            UnknownRegisters => Some(()),
            UnusedOAM => Some(()),
            _ => panic!(),
        }
    }
}

#[cfg(test)]
impl Memory {
    pub fn raw_read(&self, addr: i32) -> i32 { self.mem[addr as usize] as i32 }
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            mem: vec![0; 0x10000],
        }
    }

    /// Temporary while the entire system moves away form direct memory access.
    pub fn read(&self, raw_address: i32) -> i32 {
        let address = Address::from_raw(raw_address).unwrap();
        MemoryMapped::read(self, address).unwrap()
    }
    pub fn store(&mut self, raw_address: i32, value: i32) {
        let address = Address::from_raw(raw_address).unwrap();
        MemoryMapped::write(self, address, value).unwrap();
    }
}
