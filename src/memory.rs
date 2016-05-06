pub enum Register {
    Lcdc = 0xFF40,
    ScrollY = 0xFF42,
    ScrollX = 0xFF43,
    CurScln = 0xFF44,
    BgPalette = 0xFF47,
}

pub struct Memory {
    mem : [u8; 8*1024],
}

impl Memory {
    pub fn read_general_8(&self, location : usize) -> u8 {
        self.mem[location]
    }
    
    pub fn read_reg(&self, reg: Register) -> u8 {
        self.mem[reg as usize]
    }

    pub fn store_general_8(&mut self, location : usize, value : u8) {
        // TODO: Simply ignore read-only locations
        assert!(location != Register::CurScln as usize);
        
        self.mem[location] = value;
    }

    pub fn store_general_16(&mut self, location: usize, value: u16) {
        self.mem[location] = (value & 0xFF) as u8;
        self.mem[location + 1] = ((value >> 8) & 0xFF) as u8;
    }

    pub fn read_general_16(&self, location: usize) -> u16 {
        self.mem[location] as u16 | ((self.mem[location+1] as u16) << 8)
    }
}