/// Implements basic MBC3 support. Does not support RTC - returns 0 when reading any
/// RTC-related registers.
use crate::cart;
use crate::mmu;

pub struct Cart {
    mem: Vec<u8>,
    ram: Vec<u8>,
    enable_ram: bool,
    rom_bank: i32,
    ram_bank: i32,

    num_ram_banks: i32,
    num_rom_banks: i32,
}

impl Cart {
    pub fn from_mem(mem: Vec<u8>, ram_size: usize) -> Cart {
        debug_assert_gt!(ram_size, 0);
        let rom_size = mem.len() as i32;
        Cart {
            mem,
            ram: vec![0; ram_size],
            enable_ram: false,
            rom_bank: 1,
            ram_bank: 0,

            num_ram_banks: ram_size as i32 / cart::RAM_BANK_SIZE,
            num_rom_banks: rom_size / cart::ROM_BANK_SIZE,
        }
    }

    fn translate_rom_bank_read(&self, raw_address: i32) -> i32 {
        if self.rom_bank < self.num_rom_banks {
            self.mem(self.rom_bank * cart::ROM_BANK_SIZE + (raw_address - 0x4000))
        } else {
            0xFF
        }
    }

    fn translate_ram_bank_read(&self, raw_address: i32) -> i32 {
        if self.ram_bank < self.num_ram_banks {
            self.ram(self.ram_bank * cart::RAM_BANK_SIZE + (raw_address - 0xA000))
        } else {
            0xFF
        }
    }

    fn translate_read(&self, raw_address: i32) -> Option<i32> {
        match raw_address {
            0x0000..=0x3FFF => Some(self.mem(raw_address)),
            0x4000..=0x7FFF => Some(self.translate_rom_bank_read(raw_address)),
            0xA000..=0xBFFF => {
                if self.ram_bank < 8 {
                    Some(self.translate_ram_bank_read(raw_address))
                } else {
                    // TODO: Do we care about the RTC?
                    Some(0x00)
                }
            }
            _ => None,
        }
    }

    fn mem(&self, addr: i32) -> i32 {
        self.mem[addr as usize] as i32
    }
    fn ram(&self, addr: i32) -> i32 {
        self.ram[addr as usize] as i32
    }
}

impl mmu::MemoryMapped for Cart {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw_address) = address;
        self.translate_read(raw_address)
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw_address) = address;
        match raw_address {
            // RAM.
            0xA000..=0xBFFF => {
                if self.enable_ram && self.ram_bank < self.num_ram_banks {
                    let addr = self.ram_bank * cart::RAM_BANK_SIZE + (raw_address - 0xA000);
                    self.ram[addr as usize] = value as u8;
                }
                Some(())
            }
            // RAM Enable.
            0x0000..=0x1FFF => {
                self.enable_ram = (value & 0x0F) == 0x0A;
                Some(())
            }
            // ROM Bank.
            0x2000..=0x3FFF => {
                self.rom_bank = match value & 0x7F {
                    0 => 1,
                    num => num,
                };
                Some(())
            }
            // RAM Bank.
            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0C;
                Some(())
            }
            // RTC Stuff.
            0x6000..=0x7FFF => Some(()),
            _ => None,
        }
    }
}
