use arrayvec::ArrayVec;
use bitfield::bitfield;

bitfield! {
    pub struct FifoEntry(u8);
    impl Debug;
    u8;
    pub pixel_index, set_index: 1, 0;
    pub is_sprite, set_is_sprite: 2;
    pub priority, set_priority: 3, 3;
    pub palette, set_palette: 4, 4;
}

impl_bitfield_helpful_traits!(FifoEntry);

impl FifoEntry {
    pub fn from_sprite_row(
        mut row: u16,
        is_sprite: bool,
        priority: u8,
        palette: u8,
        flip_x: bool,
    ) -> impl Iterator<Item = FifoEntry> {
        if flip_x {
            row = row.reverse_bits();
        }

        std::iter::from_fn(move || {
            let mut entry = FifoEntry(0);
            entry.set_index(((row & 0xC000) >> 14) as u8);
            entry.set_is_sprite(is_sprite);
            entry.set_priority(priority);
            entry.set_palette(palette);
            row <<= 2;
            Some(entry)
        })
    }

    pub fn from_row(row: u16) -> impl Iterator<Item = FifoEntry> {
        FifoEntry::from_sprite_row(row, false, 0, 0, false)
    }
}

// A sad attempt to make a copyable fifo.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PixelFifo {
    pub is_suspended: bool,

    pub fifo: ArrayVec<[FifoEntry; 16]>,

    pixels_to_scroll: i32,
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

    pub fn enough_for_sprite(&self) -> bool { self.fifo.len() >= 8 }

    pub fn has_pixels(&self) -> bool { !self.is_suspended && self.fifo.len() > 8 }
    pub fn has_room(&self) -> bool { !self.is_suspended && self.fifo.len() <= 8 }

    /// Will return false if this pixel should be skipped due to fine x-scroll.
    pub fn is_good_pixel(&self) -> bool { self.pixels_to_scroll == 0 }

    pub fn peek(&self) -> FifoEntry { self.fifo[0] }

    pub fn combined_with_sprite(
        mut self,
        sprite_row: impl Iterator<Item = FifoEntry>,
    ) -> PixelFifo {
        for (i, entry) in sprite_row.collect::<Vec<_>>().into_iter().enumerate() {
            self.fifo[i] = PixelFifo::blend_sprite(self.fifo[i], entry);
        }
        self
    }

    pub fn pushed(&self, row: impl Iterator<Item = FifoEntry>) -> PixelFifo {
        let mut new_me = self.clone();
        new_me.fifo.extend(row.take(8));
        new_me
    }

    pub fn popped(&self) -> PixelFifo {
        let mut new_me = self.clone();
        new_me.fifo.remove(0);
        if new_me.pixels_to_scroll > 0 {
            new_me.pixels_to_scroll -= 1;
        }
        new_me
    }

    pub fn cleared(self) -> PixelFifo {
        PixelFifo {
            pixels_to_scroll: self.pixels_to_scroll,
            ..PixelFifo::new()
        }
    }

    fn blend_sprite(behind: FifoEntry, sprite: FifoEntry) -> FifoEntry {
        debug_assert!(sprite.is_sprite());
        // We do not draw over existing sprites, or if the sprite is transparent.
        if behind.is_sprite() || sprite.pixel_index() == 0 {
            return behind;
        }
        // Sprite will win over translucent bg, or if it is solid and priority 0.
        if behind.pixel_index() == 0 || sprite.priority() == 0 {
            sprite
        } else {
            behind
        }
    }
}
