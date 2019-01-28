use std::fs::File;
use std::io::prelude::Read;

enum MbcVersion {
    None,
    Mbc1,
}

struct CartType {
    mbc: MbcVersion,
    has_ram: bool,
}

pub struct Cart {
    mem: Vec<u8>,
    cart_type: CartType,
}

impl Cart {
    pub fn new() -> Cart {
        Cart { mem: Vec::new() }
    }

    pub fn initialize_from_file(&mut self, file_name: &str) {
        let mut f = File::open(file_name).unwrap();
        let mut file_contents = Vec::<u8>::new();
        f.read_to_end(&mut file_contents).unwrap();
    }

    fn translate_cart_type(cart_type_value: u8) -> CartType {
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

    fn initialize_memory(&mut self, file_contents: Vec<u8>) {
        self.cart_type = Cart::translate_cart_type(file_contents[0x0147]);
        match self.cart_type.mbc {
            MbcVersion::Mbc1 => (),
        }
    }

    pub fn read_file(&mut self, file_name: &str) {
        // TODO: Error handling
        let mut f = File::open(file_name).unwrap();
        f.read_to_end(&mut self.mem).unwrap();
    }

    pub fn cart_type(&self) -> CartType {
        match self.mem[0x147] {
            0 => CartType::Rom,
            _ => panic!("Unsupported cart type: {}", self.mem[0x147]),
        }
    }

    pub fn rom_size(&self) -> u32 {
        match self.mem[0x148] {
            0 => 32 * 1024,
            _ => panic!("Unsupported rom size"),
        }
    }
}
