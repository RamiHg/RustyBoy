use std::{fs::File, io::prelude::Read};

use crate::mmu;

mod mbc1;
mod mbc3;

pub type Cart = dyn mmu::MemoryMapped;

pub const ROM_BANK_SIZE: i32 = 16 * 1024;
pub const RAM_BANK_SIZE: i32 = 8 * 1024;

#[derive(Debug)]
enum MbcVersion {
    None,
    Mbc1,
    Mbc3,
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
            0x02 | 0x03 => CartType {
                mbc: MbcVersion::Mbc1,
                has_ram: true,
            },
            0x08 | 0x09 => CartType {
                mbc: MbcVersion::None,
                has_ram: true,
            },
            0x0F | 0x11 => CartType {
                mbc: MbcVersion::Mbc3,
                has_ram: false,
            },
            0x10 | 0x12 | 0x13 => CartType {
                mbc: MbcVersion::Mbc3,
                has_ram: true,
            },
            _ => panic!("Unsupported cart type {}", cart_type_value),
        }
    }
}

fn get_rom_size(setting: u8) -> usize {
    match setting {
        0..=8 => (32 * 1024) << (setting as usize),
        _ => panic!("Unsupported rom size setting: {}", setting),
    }
}

fn get_ram_size(setting: u8) -> usize {
    (match setting {
        0 => 0,
        1 => panic!("Test 2kb ram size"),
        2 => 8,
        3 => 32,
        4 => 128,
        5 => 64,
        _ => panic!("Unsupported ram size setting: {}", setting),
    }) * 1024
}

pub fn read_file(file_name: &str) -> Vec<u8> {
    let mut f = File::open(file_name).unwrap();
    let mut file_contents = Vec::new();
    f.read_to_end(&mut file_contents).unwrap();
    file_contents
}

pub fn from_file(file_name: &str) -> Box<dyn mmu::MemoryMapped> {
    from_file_contents(&read_file(file_name))
}

pub fn from_file_contents(file_contents: &[u8]) -> Box<dyn mmu::MemoryMapped> {
    let cart_type = CartType::from_setting(file_contents[0x0147]);
    let rom_size = get_rom_size(file_contents[0x148]);
    let ram_size = if cart_type.has_ram {
        get_ram_size(file_contents[0x149])
    } else {
        0
    };
    let mut mem = vec![0; rom_size];
    // Copy over contents from file into memory.
    mem[0..file_contents.len()].copy_from_slice(file_contents);
    assert_eq!(mem[0x148], file_contents[0x148]);
    match cart_type.mbc {
        MbcVersion::None => Box::new(none::Cart::from_mem(mem, ram_size)),
        MbcVersion::Mbc1 => Box::new(mbc1::Cart::from_mem(mem, ram_size)),
        MbcVersion::Mbc3 => Box::new(mbc3::Cart::from_mem(mem, ram_size)),
    }
}

mod none {
    use crate::mmu;

    pub struct Cart {
        mem: Vec<u8>,
    }

    impl Cart {
        pub fn from_mem(mem: Vec<u8>, _: usize) -> Cart {
            Cart { mem }
        }
    }

    impl mmu::MemoryMapped for Cart {
        fn write(&mut self, address: mmu::Address, _: i32) -> Option<()> {
            let mmu::Address(location, _) = address;
            match location {
                mmu::Location::MbcRom | mmu::Location::MbcRam => Some(()),
                _ => None,
            }
        }

        fn read(&self, address: mmu::Address) -> Option<i32> {
            let mmu::Address(location, raw) = address;
            match location {
                mmu::Location::MbcRom => Some(self.mem[raw as usize] as i32),
                mmu::Location::MbcRam => Some(0xFF),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::mmu;

    pub struct DynamicCart {
        mem: Vec<u8>,
    }
    impl DynamicCart {
        pub fn new() -> DynamicCart {
            DynamicCart {
                mem: vec![0; 0xC000],
            }
        }
    }
    impl mmu::MemoryMapped for DynamicCart {
        fn read(&self, address: mmu::Address) -> Option<i32> {
            let mmu::Address(location, raw) = address;
            match location {
                mmu::Location::MbcRom | mmu::Location::MbcRam => {
                    Some(self.mem[raw as usize] as i32)
                }
                _ => None,
            }
        }
        fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
            let mmu::Address(location, raw) = address;
            match location {
                mmu::Location::MbcRom | mmu::Location::MbcRam => {
                    self.mem[raw as usize] = value as u8;
                    Some(())
                }
                _ => None,
            }
        }
    }
}
