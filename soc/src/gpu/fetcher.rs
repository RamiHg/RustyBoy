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
    ReadTileIndex,
    ReadData0,
    ReadData1,
    Ready,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Invalid
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PixelFetcher {
    pub mode: Mode,
    pub tock: bool,
    sprite_mode: bool,
    pub window_mode: bool,

    bg_tiles: i32,
    pub window_tiles_read: i32,
    // Mostly (i.e. just) needed for sprites. Can easily just pass this in the execute functions.
    y_within_tile: i32,

    tile_index: u8,
    data0: u8,
    data1: u8,
}

impl PixelFetcher {
    pub fn new() -> PixelFetcher {
        PixelFetcher::default()
    }

    pub fn start_new_scanline(gpu: &Gpu) -> PixelFetcher {
        PixelFetcher {
            mode: Mode::ReadTileIndex,
            bg_tiles: util::upper_5_bits(gpu.scroll_x),
            ..Default::default()
        }
    }

    pub fn start_new_sprite(
        self,
        gpu: &Gpu,
        sprite_index: i32,
        sprite: &SpriteEntry,
    ) -> PixelFetcher {
        let mut y_within_tile = (gpu.current_y() - sprite.top()) % 16;
        if sprite.flip_y() {
            y_within_tile = if gpu.lcd_control().large_sprites() {
                15
            } else {
                7
            } - y_within_tile;
        }
        PixelFetcher {
            mode: Mode::ReadTileIndex,
            tock: false,
            sprite_mode: true,
            tile_index: sprite.tile_index(),
            // Compute the y-offset now while we still have the sprite.
            y_within_tile,
            ..self
        }
    }

    pub fn continue_scanline(self) -> PixelFetcher {
        debug_assert!(self.sprite_mode);
        PixelFetcher {
            mode: Mode::ReadTileIndex,
            tock: false,
            sprite_mode: false,
            ..self
        }
    }

    pub fn start_window_mode(&mut self) {
        self.mode = Mode::ReadTileIndex;
        self.tock = false;
        self.window_mode = true;
    }

    pub fn has_data(&self) -> bool {
        self.mode == Mode::Ready
    }

    pub fn execute_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        //    debug_assert_ne!(self.mode, Mode::Invalid);
        // We only read memory at every 2nd tcycle.
        if !self.tock {
            let mut next_state = self;
            next_state.tock = true;
            return next_state;
        }
        let mut next_state = self.execute_bg_tcycle(gpu);
        next_state.tock = false;
        next_state
    }

    pub fn next(mut self) -> PixelFetcher {
        self.mode = Mode::ReadTileIndex;
        self.tock = false;
        if self.window_mode {
            self.window_tiles_read += 1;
        } else {
            self.bg_tiles += 1;
        }
        self
    }

    fn execute_bg_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        let mut next_state = self;
        use Mode::*;
        match self.mode {
            ReadTileIndex if self.sprite_mode => {
                // Do nothing - just go to the next state.
                next_state.mode = ReadData0;
            }
            ReadTileIndex /* if !self.sprite_mode */ => {
                let address = self.nametable_address(gpu);
                next_state.tile_index = gpu.vram(address);
                // Latch the y-offset into the tile data now.
                next_state.y_within_tile = self.bg_y_within_tile(gpu);
                next_state.mode = ReadData0;
            }
            ReadData0 => {
                next_state.data0 = self.read_tile_data(gpu, 0);
                next_state.mode = ReadData1;
            }
            ReadData1 => {
                next_state.data1 = self.read_tile_data(gpu, 1);
                next_state.mode = Ready;
            }
            Ready => (),
            Invalid => (),
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
        let base_w = gpu.window_ycount;
        addr.set_upper_ybase(util::upper_5_bits(base_w) as u8);
        addr.set_nametable_number(gpu.lcd_control().window_map_select());
        (addr.0 as i32) | 0x9800
    }

    fn bg_nametable_address(&self, gpu: &Gpu) -> i32 {
        let mut addr = NametableAddress(0);
        addr.set_upper_xscroll(self.bg_tiles as u8);
        let ybase = gpu.scroll_y + gpu.current_y();
        addr.set_upper_ybase(util::upper_5_bits(ybase) as u8);
        addr.set_nametable_number(gpu.lcd_control().bg_map_select());
        (addr.0 as i32) | 0x9800
    }

    fn bg_y_within_tile(&self, gpu: &Gpu) -> i32 {
        if self.window_mode {
            gpu.window_ycount % 8
        } else {
            (gpu.scroll_y + gpu.current_y()) % 8
        }
    }

    pub fn get_row(&mut self) -> u16 {
        debug_assert_eq!(self.mode, Mode::Ready);
        debug_assert_eq!(expand_tile_bits(0b1010_0101), 0b01000100_00010001); // move this to unit test!
        let data = decode_tile_data(self.data0, self.data1);
        self.mode = Mode::Invalid;
        self.data0 = 0;
        self.data1 = 0;
        self.tile_index = 255;
        data
    }

    fn read_tile_data(&self, gpu: &Gpu, byte: i32) -> u8 {
        let tileset_id = if self.sprite_mode {
            1
        } else {
            gpu.lcd_control().bg_set_id() as i32
        };
        let address =
            PixelFetcher::tileset_address(tileset_id, self.tile_index) + self.y_within_tile * 2;
        gpu.vram(address + byte)
    }

    fn tileset_address(tileset_id: i32, tile_index: u8) -> i32 {
        if tileset_id == 0 {
            PixelFetcher::tileset_0_address(tile_index)
        } else {
            PixelFetcher::tileset_1_address(tile_index)
        }
    }
    fn tileset_0_address(tile_index: u8) -> i32 {
        0x9000 + (tile_index as i8) as i32 * 16
    }
    fn tileset_1_address(tile_index: u8) -> i32 {
        0x8000 + tile_index as i32 * 16
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
