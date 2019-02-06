use crate::cart::Cart;
use crate::registers::Register;
use crate::util;

// Useful tidbits:
// https://retrocomputing.stackexchange.com/questions/1178/what-is-this-unused-memory-range-in-the-game-boys-memory-map

pub enum RegisterAddr {
    Lcdc = 0xFF40,
    ScrollY = 0xFF42,
    ScrollX = 0xFF43,
    CurScln = 0xFF44,
    BgPalette = 0xFF47,

    InterruptEnable = 0xFFFF,
}

pub struct Memory {
    mem: [u8; 0x10000],
    pub cart: Box<Cart>,
}

#[derive(Clone)]
pub struct MemoryError {
    pub location: usize,
    pub reason: &'static str,
}
pub type Result<T> = core::result::Result<T, MemoryError>;

impl core::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "(0x{:X?}): {}.", self.location, self.reason)
    }
}
impl core::fmt::Debug for MemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "(0x{:X?}): {}.", self.location, self.reason)
    }
}
impl std::error::Error for MemoryError {}

#[derive(Debug)]
enum WriteableLocation {
    // 0000 to 8000 or 0xA000 to C000. MBC.
    Mbc,
    // 8000 to A000. Video RAM.
    VRam,
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

struct WriteableAddress(WriteableLocation, usize);

enum ReadableAddress {
    WriteableAddress(WriteableAddress),
}

// TODO: Clean up this API.
impl Memory {
    pub fn new(cart: Box<dyn Cart>) -> Memory {
        Memory {
            mem: [0; 0x10000],
            cart,
        }
    }

    pub fn set_starting_sequence(&mut self) {
        self.mem[0xFF05] = 0;
        self.mem[0xFF06] = 0;
        self.mem[0xFF07] = 0;
        self.mem[0xFF10] = 0x80;
        self.mem[0xFF11] = 0xBF;
        self.mem[0xFF12] = 0xF3;
        self.mem[0xFF14] = 0xBF;
        self.mem[0xFF16] = 0x3F;
        self.mem[0xFF17] = 0x00;
        self.mem[0xFF19] = 0xBF;
        self.mem[0xFF1A] = 0x7F;
        self.mem[0xFF1B] = 0xFF;
        self.mem[0xFF1C] = 0x9F;
        self.mem[0xFF1E] = 0xBF;
        self.mem[0xFF20] = 0xFF;
        self.mem[0xFF21] = 0x00;
        self.mem[0xFF22] = 0x00;
        self.mem[0xFF23] = 0xBF;
        self.mem[0xFF24] = 0x77;
        self.mem[0xFF25] = 0xF3;
        self.mem[0xFF26] = 0xF1;
        self.mem[0xFF40] = 0x91;
        self.mem[0xFF42] = 0x00;
        self.mem[0xFF43] = 0x00;
        self.mem[0xFF45] = 0x00;
        self.mem[0xFF47] = 0xFC;
        self.mem[0xFF48] = 0xFF;
        self.mem[0xFF49] = 0xFF;
        self.mem[0xFF4A] = 0x00;
        self.mem[0xFF4B] = 0x00;
        self.mem[0xFFFF] = 0x00;
    }

    /// Creates a WriteableAddress object from a given address.
    ///
    /// Verifies that an address is writeable, then finds the type of memory location
    /// that it corresponds to. Returns an Error if the raw address given is not writeable.
    fn translate_writeable_address(&self, raw: usize) -> Result<WriteableAddress> {
        use self::WriteableLocation::*;
        match raw {
            0x0000...0x7FFF | 0xA000...0xBFFF => Ok(WriteableAddress(Mbc, raw)),
            0x8000...0x9FFF => Ok(WriteableAddress(VRam, raw)),
            0xC000...0xDFFF => Ok(WriteableAddress(InternalRam, raw)),
            0xE000...0xFDFF => Ok(WriteableAddress(InternalRam, raw - 0x2000)),
            0xFE00...0xFE9F => Ok(WriteableAddress(OAM, raw)),
            0xFEA0...0xFEFF => Ok(WriteableAddress(UnusedOAM, raw)),
            (0xFF00...0xFF4B) | 0xFFFF => Ok(WriteableAddress(Registers, raw)),
            0xFF4C...0xFF7F => Ok(WriteableAddress(UnknownRegisters, raw)),
            0xFF80...0xFFFE => Ok(WriteableAddress(HighRam, raw)),
            _ => Err(MemoryError {
                location: raw,
                reason: "Address not writeable.",
            }),
        }
    }

    /// Creates a ReadableAddress from a given address.
    fn translate_readable_address(&self, raw: usize) -> Result<ReadableAddress> {
        match raw {
            _ => match self.translate_writeable_address(raw) {
                Ok(val) => Ok(ReadableAddress::WriteableAddress(val)),
                Err(_) => Err(MemoryError {
                    location: raw,
                    reason: "Address not readable.",
                }),
            },
        }
    }

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

    pub fn read(&self, raw_address: i32) -> i32 {
        assert!(raw_address >= 0 && raw_address <= 0xFFFF);
        self.read_general_8(raw_address as usize) as i32
    }

    /// Stores should theoretically never crash. So we do not return a Result.
    pub fn store(&mut self, raw_address: i32, value: i32) {
        debug_assert!(util::is_16bit(raw_address));
        debug_assert!(util::is_8bit(value));
        self.store_general_8(raw_address as usize, value as u8);
    }

    fn get_mut_8(&mut self, location: usize) -> &mut u8 {
        let WriteableAddress(_, addr) = self.translate_writeable_address(location).unwrap();
        &mut self.mem[addr]
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

    pub fn read_reg(&self, reg: RegisterAddr) -> u8 {
        self.read_general_8(reg as usize)
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

    pub fn store_reg(&mut self, reg: RegisterAddr, value: u8) {
        self.store_memory_8(reg as usize, value);
    }

    pub fn store_general_16(&mut self, location: usize, value: u16) {
        self.store_general_8(location, (value & 0xFF) as u8);
        self.store_general_8(location + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub fn read_general_16(&self, location: usize) -> u16 {
        self.read_general_8(location) as u16 | ((self.read_general_8(location + 1) as u16) << 8)
    }

    pub fn store_memory_8(&mut self, location: usize, value: u8) {
        self.mem[location] = value;
    }

    #[cfg(test)]
    pub fn mem(&mut self) -> &mut [u8; 0x10000] {
        &mut self.mem
    }
}
