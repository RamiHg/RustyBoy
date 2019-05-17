use super::sprites::SpriteEntry;
use super::Gpu;
use crate::util;

use bitfield::bitfield;

bitfield! {
    struct NametableAddress(u16);
    impl Debug;
    u8;
    upper_xscroll, set_upper_xscroll: 4, 0;
    upper_ybase, set_upper_ybase: 9, 5;
    nametable_number, set_nametable_number: 10, 10;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    Invalid,
    // TODO: Refactor this a bit. Currently only needed for sprite mode - but can easily put bg
    // addresses here too.
    ReadTileIndex { address: Option<i32> },
    ReadData0,
    ReadData1,
    Ready,
}

impl Default for Mode {
    fn default() -> Mode { Mode::Invalid }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PixelFetcher {
    pub mode: Mode,
    pub tock: bool,
    sprite_mode: bool,
    window_mode: bool,
    pub is_initial_fetch: bool,

    y_within_tile: i32,

    bg_tiles_read: i32,
    window_tiles_read: i32,

    tile_index: u8,
    data0: u8,
    data1: u8,
}

impl PixelFetcher {
    pub fn new() -> PixelFetcher { PixelFetcher::default() }

    pub fn start_new_scanline(gpu: &Gpu) -> PixelFetcher {
        PixelFetcher {
            mode: Mode::ReadTileIndex { address: None },
            is_initial_fetch: true,
            ..Default::default()
        }
    }

    pub fn start_new_sprite(
        &self,
        gpu: &Gpu,
        sprite_index: i32,
        sprite: &SpriteEntry,
    ) -> PixelFetcher {
        let mut y_within_tile = (gpu.current_y + gpu.scroll_y - sprite.top()) % 16;
        if sprite.flip_y() {
            y_within_tile = if gpu.lcd_control.large_sprites() {
                15
            } else {
                7
            } - y_within_tile;
        }
        PixelFetcher {
            mode: Mode::ReadTileIndex {
                address: Some(0xFE00 + sprite_index * 4 + 2),
            },
            sprite_mode: true,
            // Compute the y-offset now while we still have the sprite.
            y_within_tile,
            // We must preserve the state of the background fetch.
            bg_tiles_read: self.bg_tiles_read,
            ..Default::default()
        }
    }

    pub fn start_continue_scanline(&self) -> PixelFetcher {
        debug_assert!(self.sprite_mode);
        PixelFetcher {
            mode: Mode::ReadTileIndex { address: None },
            sprite_mode: false,
            bg_tiles_read: self.bg_tiles_read,
            ..*self
        }
    }

    pub fn start_window_mode(self) -> PixelFetcher {
        PixelFetcher {
            mode: Mode::ReadTileIndex { address: None },
            window_mode: true,
            bg_tiles_read: self.bg_tiles_read,
            ..Default::default()
        }
    }

    pub fn has_data(&self) -> bool { self.mode == Mode::Ready }

    pub fn execute_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        debug_assert_ne!(self.mode, Mode::Invalid);
        // We only read memory at every 2nd tcycle.
        if !self.tock {
            let mut next_state = self;
            next_state.tock = true;
            return next_state;
        }
        let mut next_state = if self.sprite_mode {
            self.execute_sprite_tcycle(gpu)
        } else {
            self.execute_bg_tcycle(gpu)
        };
        next_state.tock = false;
        next_state
    }

    pub fn next(mut self) -> PixelFetcher {
        self.mode = Mode::ReadTileIndex { address: None };
        self.tock = false;
        if self.window_mode {
            self.window_tiles_read += 1;
        } else if !self.is_initial_fetch {
            self.bg_tiles_read += 1;
        }
        self.is_initial_fetch = false;
        self
    }

    fn execute_bg_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        let mut next_state = self;
        use Mode::*;
        match self.mode {
            ReadTileIndex { .. } => {
                let address = self.nametable_address(gpu);
                next_state.tile_index = gpu.vram(address);
                // Latch the y-offset into the tile data now.
                next_state.y_within_tile = self.y_within_tile(gpu);
                next_state.mode = ReadData0;
            }
            ReadData0 => {
                next_state.data0 = self.read_tile_data(gpu, 0);
                next_state.mode = ReadData1;
            }
            ReadData1 => {
                next_state.data1 = self.read_tile_data(gpu, 1);
                next_state.mode = Ready;
                if self.is_initial_fetch {
                    next_state.mode = ReadData0;
                    next_state.is_initial_fetch = false;
                }
            }
            Ready => (),
            Invalid => panic!(),
        }
        next_state
    }

    fn execute_sprite_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        let mut next_state = self;
        use Mode::*;
        match self.mode {
            ReadTileIndex {
                address: Some(address),
            } => {
                // The tile index is the 3rd byte of its entry.
                // let address = 0xFE00 + sprite_index as i32 * 4 + 2;
                next_state.tile_index = gpu.oam(address);
                next_state.mode = ReadData0;
            }
            ReadData0 => {
                next_state.data0 = gpu.vram(self.sprite_tiledata_address());
                // HW: I'm not actually sure what the timing is like here. Do we skip the sprite
                // entirely if we know we're past its 8-pixel vertical bounds?
                if !gpu.lcd_control.large_sprites() && self.y_within_tile >= 8 {
                    next_state.data0 = 0;
                    panic!();
                }
                next_state.mode = ReadData1;
            }
            ReadData1 => {
                next_state.data1 = gpu.vram(self.sprite_tiledata_address() + 1);
                if !gpu.lcd_control.large_sprites() && self.y_within_tile >= 8 {
                    next_state.data1 = 0;
                    panic!();
                }
                next_state.mode = Ready;
            }
            Ready => (),
            _ => panic!(),
        }
        next_state
    }

    fn nametable_address(&self, gpu: &Gpu) -> i32 {
        if self.window_mode {
            self.window_nametable_address(gpu)
        } else {
            self.bg_nametable_address(gpu)
        }
    }

    fn window_nametable_address(&self, gpu: &Gpu) -> i32 {
        let mut addr = NametableAddress(0);
        addr.set_upper_xscroll(self.window_tiles_read as u8);
        let base_w = gpu.window_ycount - gpu.scroll_y;
        addr.set_upper_ybase(util::upper_5_bits(base_w) as u8);
        addr.set_nametable_number(gpu.lcd_control.window_map_select());
        (addr.0 as i32) | 0x9800
    }

    fn bg_nametable_address(&self, gpu: &Gpu) -> i32 {
        let mut addr = NametableAddress(0);
        addr.set_upper_xscroll((util::upper_5_bits(gpu.scroll_x) + self.bg_tiles_read) as u8);
        let ybase = gpu.scroll_y + gpu.current_y.0;
        addr.set_upper_ybase(util::upper_5_bits(ybase) as u8);
        addr.set_nametable_number(gpu.lcd_control.bg_map_select());
        (addr.0 as i32) | 0x9800
    }

    fn y_within_tile(&self, gpu: &Gpu) -> i32 {
        if self.window_mode {
            (gpu.window_ycount - gpu.scroll_y) % 8
        } else {
            (gpu.scroll_y + gpu.current_y.0) % 8
        }
    }

    pub fn get_row(&self) -> u16 {
        debug_assert_eq!(self.mode, Mode::Ready);
        debug_assert_eq!(expand_tile_bits(0b1010_0101), 0b01000100_00010001);
        decode_tile_data(self.data0, self.data1)
    }

    fn read_tile_data(&self, gpu: &Gpu, byte: i32) -> u8 {
        let address = if self.window_mode {
            0x8800 + self.tile_index as i32 * 16
        } else {
            self.bg_tiledata_address(gpu)
        };
        // If address is -1, it means we are rows 8-16 of a sprite in 8x8 mode.
        gpu.vram(address + byte)
    }

    fn sprite_tiledata_address(&self) -> i32 {
        0x8000 + self.tile_index as i32 * 16 + self.y_within_tile * 2
    }

    fn bg_tiledata_address(&self, gpu: &Gpu) -> i32 {
        let base = gpu.lcd_control.translate_bg_set_index(self.tile_index);
        base + self.y_within_tile * 2
        // // tile_index * 16 + y_within_tile * 2
        // let tile_address = if gpu.lcd_control.bg_set_select() {
        //     ((self.tile_index as i32) << 4) | (y_within_tile << 1)
        // } else {
        //     (0x1000 - ((self.tile_index as i32) << 4)) | (y_within_tile << 1)
        // };
        // 0x8000 + tile_address
    }
}

fn expand_tile_bits(bits_u8: u8) -> u16 {
    let bits = bits_u8 as u16;
    (bits & 0x1)
        | ((bits & 0x2) << 1)
        | ((bits & 0x4) << 2)
        | ((bits & 0x8) << 3)
        | ((bits & 0x10) << 4)
        | ((bits & 0x20) << 5)
        | ((bits & 0x40) << 6)
        | ((bits & 0x80) << 7)
}
fn decode_tile_data(data0: u8, data1: u8) -> u16 {
    expand_tile_bits(data0) | (expand_tile_bits(data1) << 1)
}
