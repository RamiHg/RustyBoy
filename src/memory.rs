pub struct Memory {
    mem : [u8; 8*1024],
}

impl Memory {
    pub fn read_general_8(&self, location : usize) -> u8 {
        self.mem[location]
    }

    pub fn store_general_8(&mut self, location : usize, value : u8) {
        self.mem[location] = value;
    }
}