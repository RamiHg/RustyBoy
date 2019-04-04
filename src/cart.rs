use std::{fs::File, io::prelude::Read};

use crate::memory::Result;

#[derive(Debug)]
enum MbcVersion {
    None,
    Mbc1,
}

#[derive(Debug)]
struct CartType {
    mbc: MbcVersion,
    has_ram: bool,
}

impl CartType {
    fn from_setting(cart_type_value: u8) -> CartType {
        match cart_type_value {
            0x00 => CartType {
                mbc: MbcVersion::None,
                has_ram: false,
            },
            0x01 => CartType {
                mbc: MbcVersion::Mbc1,
                has_ram: false,
            },
            _ => panic!("Unsupported cart type {}", cart_type_value),
        }
    }
}

fn get_rom_size(setting: u8) -> usize {
    match setting {
        0...8 => (32 * 1024) << (setting as usize),
        _ => panic!("Unsupported rom size setting: {}", setting),
    }
}

fn get_ram_size(setting: u8) -> usize {
    match setting {
        0 => 0,
        _ => panic!("Unsupported ram size setting: {}", setting),
    }
}

pub trait Cart {
    fn read(&self, raw_address: usize) -> Result<u8>;
    fn write(&mut self, raw_address: usize, value: u8) -> Result<()>;
}

pub fn from_file(file_name: &str) -> Box<dyn Cart> {
    let mut f = File::open(file_name).unwrap();
    let mut file_contents = Vec::<u8>::new();
    f.read_to_end(&mut file_contents).unwrap();
    from_file_contents(file_contents)
}

fn from_file_contents(file_contents: Vec<u8>) -> Box<dyn Cart> {
    let cart_type = CartType::from_setting(file_contents[0x0147]);
    let rom_size = get_rom_size(file_contents[0x148]);
    let ram_size = get_ram_size(file_contents[0x149]);
    let mut mem = vec![0; rom_size];
    // Copy over contents from file into memory.
    mem[0..file_contents.len()].copy_from_slice(file_contents.as_slice());
    assert_eq!(mem[0x148], file_contents[0x148]);

    println!("{:?}", file_contents[0x0147]);
    match cart_type.mbc {
        MbcVersion::None => Box::new(none::Cart::from_mem(mem, ram_size)),
        MbcVersion::Mbc1 => Box::new(mbc1::Cart::from_mem(mem, ram_size)),
    }
}

mod none {
    use crate::memory::Result;

    pub struct Cart {
        mem: Vec<u8>,
    }

    impl Cart {
        pub fn from_mem(mem: Vec<u8>, _: usize) -> Cart { Cart { mem } }
    }

    impl super::Cart for Cart {
        fn write(&mut self, _raw_address: usize, _value: u8) -> Result<()> { Ok(()) }

        fn read(&self, raw_address: usize) -> Result<u8> {
            match raw_address {
                0x0000...0x7FFF => Ok(self.mem[raw_address]),
                0xA000...0xBFFF => Ok(0xFF),
                _ => panic!("TODO {}", raw_address),
            }
        }
    }
}

mod mbc1 {
    use crate::memory::{MemoryError, Result};

    pub struct Cart {
        mem: Vec<u8>,
        ram_size: usize,
        // Cart state registers.
        enable_ram: bool,
        rom_bank: u8,
        ram_upper_rom_bits: u8,
        is_ram_banking_mode: bool,
    }

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

    enum ReadeableLocation {
        // 0000 - 3FFF
        RomBank0,
        // 4000 - 7FFF
        SwitchableRomBank,
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

        fn translate_writeable_address(&self, raw_address: usize) -> Result<WriteableLocation> {
            use WriteableLocation::*;
            match raw_address {
                0x0000...0x1FFF => Ok(RamEnable),
                0x2000...0x3FFF => Ok(RomBank),
                0x4000...0x5FFF => Ok(RamOrUpperRomBank),
                0x6000...0x7FFF => Ok(RomRamMode),
                _ => Err(MemoryError {
                    location: raw_address,
                    reason: "Invalid write address.",
                }),
            }
        }

        fn translate_rom_bank_read(&self, raw_address: usize) -> Result<u8> {
            // First, compute the rom bank number (TODO: Can cache between bank selects).
            assert_eq!(self.rom_bank & 0xE0, 0);
            assert!(self.rom_bank != 0);
            assert_eq!(self.ram_upper_rom_bits & 0xFC, 0);
            assert_eq!(self.is_ram_banking_mode, false);
            // 0b000xxxxx | 0b0xx00000
            let rom_bank: u8 = self.rom_bank | (self.ram_upper_rom_bits << 5);
            let read_address =
                (rom_bank as usize) * (16 * 1024) + ((raw_address - 0x4000) as usize);
            assert!(read_address < self.mem.len());
            Ok(self.mem[read_address])
        }
    }

    impl super::Cart for Cart {
        fn write(&mut self, raw_address: usize, value: u8) -> Result<()> {
            let location = self.translate_writeable_address(raw_address)?;
            match location {
                WriteableLocation::RomBank => {
                    // This register is used to select the lower 5 bits of the ROM bank number.
                    // 0 gets mapped to 1.
                    self.rom_bank = match value & 0x1F {
                        0 => 1,
                        num => {
                            println!("Setting to {}", num);
                            num
                        }
                    };
                    Ok(())
                }
                WriteableLocation::RamOrUpperRomBank => {
                    // Selects the upper 2 bits of the ROM bank, or the RAM bank number.
                    self.ram_upper_rom_bits = value & 0x03;
                    Ok(())
                }
                WriteableLocation::RamEnable => {
                    self.enable_ram = (value & 0x0F) == 0x0A;
                    Ok(())
                }
                _ => Err(MemoryError {
                    location: raw_address as usize,
                    reason: "Writeable location not yet implemented.",
                }),
            }
        }

        fn read(&self, raw_address: usize) -> Result<u8> {
            assert!(!self.is_ram_banking_mode);
            assert!(!self.enable_ram);
            match raw_address {
                0x0000...0x3FFF => Ok(self.mem[raw_address]),
                0x4000...0x7FFF => self.translate_rom_bank_read(raw_address),
                0xA000...0xBFFF => {
                    assert!(!self.enable_ram);
                    Ok(0xFF)
                }
                _ => panic!("TODO {}", raw_address),
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::Result;
    use crate::error;
    use crate::memory::MemoryError;

    pub struct Cart;
    impl super::Cart for Cart {
        fn read(&self, raw_address: usize) -> Result<u8> { Ok(0xFF) }
        fn write(&mut self, raw_address: usize, _: u8) -> Result<()> {
            Err(MemoryError {
                location: raw_address,
                reason: "Cannot write to ErrorCart!",
            })
        }
    }

    pub struct DynamicCart {
        mem: Vec<u8>,
    }
    impl DynamicCart {
        pub fn new() -> DynamicCart {
            DynamicCart {
                mem: vec![0; 0x8000],
            }
        }
    }
    impl super::Cart for DynamicCart {
        fn read(&self, raw_address: usize) -> Result<u8> { Ok(self.mem[raw_address]) }
        fn write(&mut self, raw_address: usize, value: u8) -> Result<()> {
            self.mem[raw_address] = value;
            Ok(())
        }
    }
}
