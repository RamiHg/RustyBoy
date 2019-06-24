use crate::mmu;

pub struct Cart {
    mem: Vec<u8>,
    ram: Vec<u8>,
    // Cart state registers.
    enable_ram: bool,
    rom_bank_lower_bits: i32,
    ram_upper_rom_bits: i32,
    is_ram_banking_mode: bool,
}

impl Cart {
    pub fn from_mem(mem: Vec<u8>, ram_size: usize) -> Cart {
        Cart {
            mem,
            ram: vec![0; ram_size],
            enable_ram: false,
            rom_bank_lower_bits: 1,
            ram_upper_rom_bits: 0,
            is_ram_banking_mode: false,
        }
    }

    fn translate_rom_bank_read(&self, raw_address: i32, rom_upper_bits: i32) -> i32 {
        // First, compute the rom bank number (TODO: Can cache between bank selects).
        debug_assert_eq!(self.rom_bank_lower_bits & 0xE0, 0);
        debug_assert_eq!(rom_upper_bits & 0xFC, 0);
        let rom_bank = i32::from(self.rom_bank_lower_bits | (rom_upper_bits << 5));
        let read_address = rom_bank * super::ROM_BANK_SIZE + (raw_address - 0x4000);
        self.mem(read_address)
    }

    fn translate_ram_bank_read(&self, raw_address: i32, ram_bank: i32) -> i32 {
        // Return 0xFF if RAM is disabled.
        if !self.enable_ram {
            return 0xFF;
        }
        self.ram(self.translate_ram_addr(raw_address, ram_bank))
    }

    fn translate_ram_addr(&self, raw_address: i32, ram_bank: i32) -> i32 {
        debug_assert_lt!(ram_bank, 4);
        ram_bank * super::RAM_BANK_SIZE + (raw_address - 0xA000)
    }

    fn translate_read(&self, raw_address: i32) -> Option<i32> {
        let (rom_upper_bits, ram_bank) = self.banks();
        match raw_address {
            // ROM Bank 0
            0x0000..=0x3FFF => Some(self.mem(raw_address)),
            // Switchable ROM Bank
            0x4000..=0x7FFF => Some(self.translate_rom_bank_read(raw_address, rom_upper_bits)),
            // Switchable RAM Bank
            0xA000..=0xBFFF => Some(self.translate_ram_bank_read(raw_address, ram_bank)),
            _ => None,
        }
    }

    fn mem(&self, addr: i32) -> i32 {
        self.mem[addr as usize] as i32
    }
    fn ram(&self, addr: i32) -> i32 {
        self.ram[addr as usize] as i32
    }

    fn banks(&self) -> (i32, i32) {
        if self.is_ram_banking_mode {
            (0, self.ram_upper_rom_bits)
        } else {
            (self.ram_upper_rom_bits, 0)
        }
    }
}

impl mmu::MemoryMapped for Cart {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(_, raw_address) = address;
        self.translate_read(raw_address)
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(_, raw_address) = address;
        let (_, ram_bank) = self.banks();
        match raw_address {
            // RAM.
            0xA000..=0xBFFF => {
                if self.enable_ram {
                    let addr = self.translate_ram_addr(raw_address, ram_bank) as usize;
                    self.ram[addr] = value as u8;
                }
                Some(())
            }
            // RAM Enable.
            0x0000..=0x1FFF => {
                if !self.ram.is_empty() {
                    self.enable_ram = (value & 0x0F) == 0x0A;
                }
                Some(())
            }
            // ROM Bank (lower bits).
            0x2000..=0x3FFF => {
                self.rom_bank_lower_bits = match value & 0x1F {
                    0 => 1,
                    num => num,
                };
                Some(())
            }
            // RAM Bank/ROM Bank (upper bits).
            0x4000..=0x5FFF => {
                self.ram_upper_rom_bits = value & 0x03;
                Some(())
            }
            // ROM/RAM Mode.
            0x6000..=0x7FFF => {
                self.is_ram_banking_mode = (value & 1) != 0;
                Some(())
            }
            _ => None,
        }
    }
}
