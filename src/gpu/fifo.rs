#[derive(Clone, Copy, Debug)]
pub struct FifoEntry {
    pixel_index: u8,
    is_sprite: bool,
    is_s1: bool,
}

impl FifoEntry {
    pub fn new() -> FifoEntry {
        FifoEntry {
            pixel_index: 0,
            is_sprite: false,
            is_s1: false,
        }
    }

    pub fn from_row(mut row: u16) -> Vec<FifoEntry> {
        let mut result = Vec::new();
        for i in 0..8 {
            result.push(FifoEntry {
                pixel_index: ((row & 0xC000) >> 14) as u8,
                is_sprite: false,
                is_s1: false,
            });
            row <<= 2;
        }
        result
    }

    pub fn from_data(pixel_index: u8, is_sprite: bool, is_s1: bool) -> FifoEntry {
        debug_assert_lt!(pixel_index, 4);
        FifoEntry {
            pixel_index,
            is_sprite,
            is_s1,
        }
    }

    pub fn pixel_index(&self) -> u8 { self.pixel_index }
    pub fn is_sprite(&self) -> bool { self.is_sprite }
}

// A sad attempt to make a copyable fifo.
#[derive(Clone, Copy, Debug)]
pub struct PixelFifo {
    fifo: [FifoEntry; 16],
    cursor: i8,
    pub is_suspended: bool,
}

impl PixelFifo {
    pub fn new() -> PixelFifo {
        PixelFifo {
            fifo: [FifoEntry::new(); 16],
            cursor: 0,
            is_suspended: false,
        }
    }

    pub fn has_pixels(&self) -> bool { self.cursor > 8 }
    pub fn has_room(&self) -> bool { self.cursor <= 8 }

    pub fn peek(&self) -> FifoEntry { self.fifo[0] }
    pub fn len(&self) -> usize {
        if self.is_suspended {
            16
        } else {
            self.cursor as usize
        }
    }

    pub fn suspend(&mut self) { self.is_suspended = true; }

    pub fn combined_with_sprite(
        mut self,
        sprite_row: impl Iterator<Item = FifoEntry>,
    ) -> PixelFifo {
        for (i, entry) in sprite_row.enumerate() {
            if self.fifo[i].pixel_index() == 0 {
                // Sprite always wins on top of translucent bg.
                self.fifo[i] = entry;
            } else if !entry.is_s1 {
                // S0 sprites always win on top of bg and S1 sprites.
                self.fifo[i] = entry;
            }
        }
        self.is_suspended = false;
        self
    }

    pub fn pushed(mut self, row: impl Iterator<Item = FifoEntry>) -> PixelFifo {
        //self.fifo.extend(row);
        for entry in row {
            self.fifo[self.cursor as usize] = entry;
            self.cursor += 1;
        }
        self
    }

    pub fn popped(&self) -> PixelFifo {
        let mut new_me = PixelFifo::new();
        for i in 1..self.cursor as usize {
            new_me.fifo[i - 1] = self.fifo[i];
        }
        new_me.cursor = self.cursor - 1;
        new_me
    }
}
