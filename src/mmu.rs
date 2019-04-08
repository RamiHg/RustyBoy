use crate::{cart::Cart, io_registers::Register, util};

use crate::error::{self, Result};

// Useful tidbits:
// https://retrocomputing.stackexchange.com/questions/1178/what-is-this-unused-memory-range-in-the-game-boys-memory-map

// pub enum RegisterAddr {
//     Lcdc = 0xFF40,
//     ScrollY = 0xFF42,
//     ScrollX = 0xFF43,
//     CurScln = 0xFF44,
//     BgPalette = 0xFF47,

//     InterruptEnable = 0xFFFF,
// }

pub struct Memory {
    mem: [u8; 0x10000],
    pub cart: Box<Cart>,
}

#[derive(Debug)]
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

pub struct Address(Location, i32);

impl Address {
    pub fn from_raw(raw: i32) -> Result<Address> {
        assert!(util::is_16bit(raw));
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
    fn handles(&self, address: Address) -> bool;
    fn read(&self, address: Address) -> Option<i32>;
    fn write(&mut self, address: Address, value: i32) -> Option<Result<()>>;
}

/// Holds the internal RAM, as well as register values that don't need to be managed by their
/// components directly.
pub struct Ram {
    mem: [u8; 0x10000],
}

impl MemoryMapped for Ram {
    fn handles(&self, address: Address) -> bool {
        let Address(location, raw) = address;
        use Location::*;
        match location {
            VRam | InternalRam | OAM | Registers | HighRam => true,
            _ => false,
        }
    }

    fn read(&self, address: Address) -> Option<i32> {
        let Address(location, raw) = address;
        use Location::*;
        match location {
            VRam | InternalRam | OAM | Registers | HighRam => Some(self.mem[raw as usize].into()),
            _ => None,
        }
    }

    fn write(&mut self, address: Address, value: i32) -> Option<Result<()>> {
        debug_assert!(util::is_8bit(value));
        let Address(location, raw) = address;
        use Location::*;
        match location {
            VRam | InternalRam | OAM | Registers | HighRam => {
                self.mem[raw as usize] = value as u8;
                Some(Ok(()))
            }
            _ => None,
        }
    }
}

impl Memory {
    // pub fn set_starting_sequence(&mut self) {
    //     self.mem[0xFF05] = 0;
    //     self.mem[0xFF06] = 0;
    //     self.mem[0xFF07] = 0;
    //     self.mem[0xFF10] = 0x80;
    //     self.mem[0xFF11] = 0xBF;
    //     self.mem[0xFF12] = 0xF3;
    //     self.mem[0xFF14] = 0xBF;
    //     self.mem[0xFF16] = 0x3F;
    //     self.mem[0xFF17] = 0x00;
    //     self.mem[0xFF19] = 0xBF;
    //     self.mem[0xFF1A] = 0x7F;
    //     self.mem[0xFF1B] = 0xFF;
    //     self.mem[0xFF1C] = 0x9F;
    //     self.mem[0xFF1E] = 0xBF;
    //     self.mem[0xFF20] = 0xFF;
    //     self.mem[0xFF21] = 0x00;
    //     self.mem[0xFF22] = 0x00;
    //     self.mem[0xFF23] = 0xBF;
    //     self.mem[0xFF24] = 0x77;
    //     self.mem[0xFF25] = 0xF3;
    //     self.mem[0xFF26] = 0xF1;
    //     self.mem[0xFF40] = 0x91;
    //     self.mem[0xFF42] = 0x00;
    //     self.mem[0xFF43] = 0x00;
    //     self.mem[0xFF45] = 0x00;
    //     self.mem[0xFF47] = 0xFC;
    //     self.mem[0xFF48] = 0xFF;
    //     self.mem[0xFF49] = 0xFF;
    //     self.mem[0xFF4A] = 0x00;
    //     self.mem[0xFF4B] = 0x00;
    //     self.mem[0xFFFF] = 0x00;
    // }

    pub fn read_general_8(&self, raw_address: usize) -> u8 {
        match self.translate_readable_address(raw_address).unwrap() {
            ReadableAddress::WriteableAddress(WriteableAddress(location, addr)) => {
                match location {
                    WriteableLocation::Mbc => self.cart.read(addr).unwrap(),
                    WriteableLocation::UnusedOAM => {
                        // TODO: Manual says this location is restricted to when OAM is not being
                        // accessed by hardware. Enforce this somehow.
                        0
                    }
                    WriteableLocation::UnknownRegisters => 0xFF,
                    _ => self.mem[addr],
                }
            }
        }
    }

    pub fn store_general_8(&mut self, raw: usize, value: u8) {
        let WriteableAddress(location, addr) = self.translate_writeable_address(raw).unwrap();
        if addr == 0xFF01 {
            print!("{}", value as char);
        }
        match location {
            WriteableLocation::Mbc => self.cart.write(raw, value).unwrap(),
            _ => self.mem[addr] = value,
        }
    }

    pub fn read_register<A, T>(&self, cons: T) -> A
    where
        A: Register,
        T: core::ops::FnOnce([u8; 1]) -> A,
    {
        cons([self.read_general_8(A::ADDRESS)])
    }

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

    pub fn store_general_16(&mut self, location: usize, value: u16) {
        self.store_general_8(location, (value & 0xFF) as u8);
        self.store_general_8(location + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub fn read_general_16(&self, location: usize) -> u16 {
        u16::from(self.read_general_8(location))
            | (u16::from(self.read_general_8(location + 1)) << 8)
    }

    #[cfg(test)]
    pub fn mem(&mut self) -> &mut [u8; 0x10000] { &mut self.mem }
}
