use crate::mmu;

enum WriteableLocation {
    // 0000 - 1FFF
    RamEnable,
    // 2000 - 3FFF
    RomBank,
    // 4000 - 5FFF
    RamOrUpperRomBank,
    // 6000 - 7FFF
    RomRamMode,
    // A000 - BFFF
    SwitchableRam,
}

impl WriteableLocation {
    fn from_raw(addr: i32) -> Option<WriteableLocation> {
        use WriteableLocation::*;
        match addr {
            0x0000...0x1FFF => Some(RamEnable),
            0x2000...0x3FFF => Some(RomBank),
            0x4000...0x5FFF => Some(RamOrUpperRomBank),
            0x6000...0x7FFF => Some(RomRamMode),
            0xA000...0xBFFF => Some(SwitchableRam),
            _ => None,
        }
    }
}

pub struct Cart {
    mem: Vec<u8>,
    #[allow(dead_code)]
    ram_size: usize,
    // Cart state registers.
    enable_ram: bool,
    rom_bank: u8,
    ram_upper_rom_bits: u8,
    is_ram_banking_mode: bool,
}

impl Cart {
    pub fn from_mem(mem: Vec<u8>, ram_size: usize) -> Cart {
        Cart {
            mem,
            ram_size,
            enable_ram: false,
            rom_bank: 1,
            ram_upper_rom_bits: 0,
            is_ram_banking_mode: false,
        }
    }

    fn translate_rom_bank_read(&self, raw_address: i32) -> Option<i32> {
        // First, compute the rom bank number (TODO: Can cache between bank selects).
        debug_assert_eq!(self.rom_bank & 0xE0, 0);
        debug_assert_ne!(self.rom_bank, 0);
        debug_assert_eq!(self.ram_upper_rom_bits & 0xFC, 0);
        debug_assert_eq!(self.is_ram_banking_mode, false);
        // 0b000xxxxx | 0b0xx00000
        let rom_bank = i32::from(self.rom_bank | (self.ram_upper_rom_bits << 5));
        let read_address = rom_bank * (16 * 1024) + (raw_address - 0x4000);
        debug_assert!(read_address < self.mem.len() as i32);
        Some(self.mem[read_address as usize] as i32)
    }
}

impl mmu::MemoryMapped for Cart {
    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw_address) = address;
        let writeable_location = WriteableLocation::from_raw(raw_address)?;
        match writeable_location {
            WriteableLocation::RomBank => {
                // This register is used to select the lower 5 bits of the ROM bank number.
                // 0 gets mapped to 1.
                self.rom_bank = match value & 0x1F {
                    0 => 1,
                    num => num as u8,
                };
                Some(())
            }
            WriteableLocation::RamOrUpperRomBank => {
                // Selects the upper 2 bits of the ROM bank, or the RAM bank number.
                self.ram_upper_rom_bits = (value & 0x03) as u8;
                Some(())
            }
            WriteableLocation::RamEnable => {
                self.enable_ram = (value & 0x0F) == 0x0A;
                Some(())
            }
            _ => panic!("Writeable location not yet implemented."),
        }
    }

    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw_address) = address;
        let writeable_location = WriteableLocation::from_raw(raw_address)?;
        use WriteableLocation::*;
        match writeable_location {
            RamEnable | RomBank => Some(self.mem[raw_address as usize] as i32),
            RamOrUpperRomBank | RomRamMode => self.translate_rom_bank_read(raw_address),
            SwitchableRam => {
                assert!(!self.enable_ram);
                Some(0xFF)
            }
        }
    }
}
