use cart::Cart;
use std::char;

pub enum Register {
    Lcdc = 0xFF40,
    ScrollY = 0xFF42,
    ScrollX = 0xFF43,
    CurScln = 0xFF44,
    BgPalette = 0xFF47,

    InterruptFlag = 0xFF0F,
    InterruptEnable = 0xFFFF,

    LcdStatus = 0xFF41,
}

pub struct Memory {
    mem : [u8; 0x10000],
    pub cart: Cart
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            mem: [0; 0x10000],
            cart: Cart::new(),
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

    pub fn read_general_8(&self, location : usize) -> u8 {
        match location as u16 {
            0x0000 ... 0x3FFF => {
                // Cartridge ROM - Bank 0
                self.cart.mem[location]
            }
            0x4000 ... 0x7FFF => {
                // Cartridge switchable banks - not supported yet
                //panic!("not supported");
                //self.cart.mem[location - 0x0150]
                self.cart.mem[location]
            }
            0x8000 ... 0x9FFF => {
                // Character RAM, BG map data 1/2
                self.mem[location]
            }
            0xA000 ... 0xBFFF => {
                // Cartridge RAM - not supported yet
               // panic!("Don't support cartridge ram")
               self.cart.mem[location]
            }
            0xC000 ... 0xDFFF => {
                // Internal RAM bank 0 and 1
                self.mem[location]
            }
            0xE000 ... 0xFDFF => {
                // RAM echo
                self.mem[location - 0x2000]
            }
            0xFE00 ... 0xFE9F => {
                // Object Attribute Memory
                self.mem[location]
            }
            0xFEA0 ... 0xFEFF => {
                self.mem[location]
                //panic!("unusable memory 0x{:x}", location);
            }
            0xFF00 ... 0xFF7F => { 
                // Hardware I/O registers. Not sure what to do here yet
                self.mem[location]
            }
            0xFF80 ... 0xFFFE => {
                // Zero page
                self.mem[location]
            }
            0xFFFF => {
                // Interrupt enable flag - not sure how we'll implement this
                self.mem[location]
            }
            _ => {
                panic!("Invalid memory being accessed: 0x{:x}", location);
            }
        }
    }

    pub fn store_general_8(&mut self, location : usize, value : u8) {
        assert!(location <= 0xFFFF);
        //assert!(location != Register::InterruptFlag as usize);
        assert!(location != Register::CurScln as usize);

        if location == 0xff01 || location == 0xff02 {
            print!("{}", char::from_u32(value as u32).unwrap());
        }
        
        match location as u16 {
            0x0000 ... 0x00FF => {
                panic!("Can't set this memory {:x}", location)
            }
            0x2000 ... 0x3FFF => {
                // ROM Bank selector
                panic!("Selecting bank {}", value);
                return;
            }
            0x8000 ... 0x97FF => {
                //println!("Writing tilemaps")
            }
            0x9800 ... 0x9FFF => {
            }
            0xA000 ... 0xBFFF => {
                // Cartridge RAM
                //panic!("Cartridge RAM not yet implemented");
                self.cart.mem[location] = value;
                return;
            }
            0xC000 ... 0xDFFF => {
                // Internal RAM (Switchable in CGB)
            }
            0xE000 ... 0xFDFF => {
                // Echo RAM. Are we allowed to write here?
                //panic!("Echo RAM?")
            }
            0xFE00 ... 0xFE9F => {
                // Object Attribute Map
            }
            0xFEA0 ... 0xFEFF => {
                panic!("Writing into unusable memory")
            }
            0xFF00 ... 0xFF7F => {
                // Hardware I/O registers.
            }
            0xFF80 ... 0xFFFE => {
                // Zero page
            }
            0xFFFF => {
            }
            _ => { panic!("Location is either unimplemented or not writable 0x{:x}", location) }
        }
        
        self.mem[location] = value;
    }
    
    pub fn read_reg(&self, reg: Register) -> u8 {
        self.read_general_8(reg as usize)
    }

    pub fn store_reg(&mut self, reg: Register, value: u8) {
        self.store_memory_8(reg as usize, value);
    }

    pub fn store_general_16(&mut self, location: usize, value: u16) {
        self.store_general_8(location, (value & 0xFF) as u8);
        self.store_general_8(location + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub fn read_general_16(&self, location: usize) -> u16 {
        self.read_general_8(location) as u16 |
            ((self.read_general_8(location+1) as u16) << 8)
    }

    pub fn store_memory_8(&mut self, location: usize, value: u8) {
        self.mem[location] = value;
    }
}