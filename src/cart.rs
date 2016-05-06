use std::fs::File;
use std::io::prelude::Read;

pub enum CartType {
    Rom = 0,
}

pub struct Cart {
    mem : Vec<u8>,
}

impl Cart {
    pub fn read_file(&mut self, file_name: &str) {
        // TODO: Error handling
        let mut f = File::open(file_name).unwrap();
        f.read_to_end(&mut self.mem).unwrap();
    }
    
    pub fn cart_type(&self) -> CartType {
        match self.mem[0x147] {
            0   => CartType::Rom,
            _   => panic!("Unsupported cart type")
        }
    }
    
    pub fn rom_size(&self) -> u32 {
        match self.mem[0x148] {
            0   => 32 * 1024,
            _   => panic!("Unsupported rom size")
        }
    }
}
