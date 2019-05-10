use arrayvec::ArrayVec;

#[derive(Clone, Copy, Debug, Default)]
pub struct FifoEntry {
    pixel_index: u8,
    pub is_sprite: bool,
}

impl FifoEntry {
    pub fn new() -> FifoEntry {
        FifoEntry {
            pixel_index: 0,
            is_sprite: false,
        }
    }

    pub fn from_row(mut row: u16) -> Vec<FifoEntry> {
        let mut result = Vec::new();
        for i in 0..8 {
            result.push(FifoEntry {
                pixel_index: ((row & 0xC000) >> 14) as u8,
                is_sprite: false,
            });
            row <<= 2;
        }
        result
    }

    pub fn pixel_index(&self) -> u8 { self.pixel_index }
}

// A sad attempt to make a copyable fifo.
#[derive(Clone, Debug, Default)]
pub struct PixelFifo {
    pub is_suspended: bool,

    fifo: ArrayVec<[FifoEntry; 16]>,

    pixels_to_scroll: i32,

    sprites_to_blend: ArrayVec<[FifoEntry; 8]>,
}

impl PixelFifo {
    pub fn new() -> PixelFifo { PixelFifo::default() }

    pub fn start_new_scanline(scroll_x: i32) -> PixelFifo {
        let pixels_to_scroll = scroll_x % 8;
        PixelFifo {
            pixels_to_scroll,
            ..PixelFifo::new()
        }
    }

    pub fn has_pixels(&self) -> bool { !self.is_suspended && self.fifo.len() > 8 }
    pub fn has_room(&self) -> bool { !self.is_suspended && self.fifo.len() <= 8 }

    /// Will return false if this pixel should be skipped due to fine x-scroll.
    pub fn is_good_pixel(&self) -> bool { self.pixels_to_scroll == 0 }

    pub fn peek(&self) -> FifoEntry {
        if self.sprites_to_blend.is_empty() {
            self.fifo[0]
        } else {
            PixelFifo::blend_sprite(self.fifo[0], self.sprites_to_blend[0])
        }
    }

    pub fn combined_with_sprite(
        mut self,
        sprite_row: impl Iterator<Item = FifoEntry>,
        priority: i32,
    ) -> PixelFifo {
        self.sprites_to_blend = sprite_row.collect();
        self.is_suspended = false;
        self
    }

    pub fn pushed(&self, row: impl Iterator<Item = FifoEntry>) -> PixelFifo {
        let mut new_me = self.clone();
        // Skip any pixels we need to skip for fine-scrolling now.
        new_me.fifo.extend(row);
        new_me
    }

    pub fn popped(&self) -> PixelFifo {
        let mut new_me = self.clone();
        new_me.fifo.remove(0);
        if !new_me.sprites_to_blend.is_empty() {
            new_me.sprites_to_blend.remove(0);
        }
        if new_me.pixels_to_scroll > 0 {
            new_me.pixels_to_scroll -= 1;
        }
        new_me
    }

    pub fn clear_sprite(&self) -> PixelFifo {
        let mut new_me = self.clone();
        new_me.sprites_to_blend.clear();
        new_me
    }

    pub fn cleared(self) -> PixelFifo {
        PixelFifo {
            pixels_to_scroll: self.pixels_to_scroll,
            ..PixelFifo::new()
        }
    }

    fn blend_sprite(behind: FifoEntry, mut sprite: FifoEntry) -> FifoEntry {
        sprite.is_sprite = true;
        // We do not draw over existing sprites.
        if behind.is_sprite {
            return behind;
        }
        // Sprite will win over translucent bg, or if it is solid and priority 0.
        if behind.pixel_index() == 0 || (sprite.pixel_index() != 0 && 0 == 0) {
            sprite
        } else {
            behind
        }
    }
}
