use std::convert::TryFrom as _;

use super::sprites::SpriteEntry;
use super::{FifoEntry, Gpu};
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Invalid,
    InitialTilemapFetch,
    ReadTileIndex,
    ReadData0,
    ReadData1,
    Ready,
}

impl Default for Mode {
    fn default() -> Mode { Mode::Invalid }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PixelFetcher {
    mode: Mode,
    //address: i32,

    tock: bool,

    bg_tiles_read: i32,
    // window_tiles_read: i32,

    tile_index: u8,
    data0: u8,
    data1: u8,
}

impl PixelFetcher {
    pub fn new() -> PixelFetcher { PixelFetcher::default() }

    pub fn start_new_scanline(gpu: &Gpu) -> PixelFetcher {
        PixelFetcher {
            mode: Mode::InitialTilemapFetch,
            ..Default::default()
        }
    }

    pub fn execute_tcycle(self, gpu: &Gpu) -> PixelFetcher {
        debug_assert_ne!(self.mode, Mode::Invalid);
        let mut next_state = self;
        // We only read memory at every 2nd tcycle.
        if !self.tock {
            next_state.tock = true;
            return next_state;
        }
        use Mode::*;
        match self.mode {
            InitialTilemapFetch => {
                // Don't actually read anything..
                next_state.mode = ReadTileIndex;
            }
            ReadTileIndex => {
                let address = self.nametable_address(gpu);
                next_state.tile_index = gpu.vram(address);
                next_state.mode = ReadData0;
            }
            ReadData0 => {
                next_state.data0 = self.read_tile_data(gpu, 0);
                next_state.mode = ReadData1;
            }
            ReadData1 => {
                next_state.data1 = self.read_tile_data(gpu, 1);
                next_state.mode = Ready;
                next_state.bg_tiles_read += 1;
            }
            Ready => (),
            Invalid => panic!(),
        }
        next_state
    }

    pub fn ready(&self) -> bool { self.mode == Mode::Ready }
    pub fn has_data(&self) -> bool { self.mode == Mode::Ready }

    fn nametable_address(&self, gpu: &Gpu) -> i32 {
        let mut addr = NametableAddress(0);
        addr.set_upper_xscroll((util::upper_5_bits(gpu.scroll_x) + self.bg_tiles_read) as u8);
        let ybase = gpu.scroll_y + gpu.current_y;
        addr.set_upper_ybase(util::upper_5_bits(ybase) as u8);
        addr.set_nametable_number(gpu.lcd_control.bg_map_select());
        (addr.0 as i32) | 0x9800
    }

    pub fn get_row(&self) -> u16 {
        debug_assert_eq!(self.mode, Mode::Ready);
        debug_assert_eq!(expand_tile_bits(0b1010_0101), 0b01000100_00010001);
        decode_tile_data(self.data0, self.data1)
    }

    pub fn next(mut self) -> PixelFetcher {
        self.mode = Mode::ReadTileIndex;
        self.tock = false;
        self
    }

    fn read_tile_data(&self, gpu: &Gpu, byte: i32) -> u8 {
        let address = self.bg_tileset_address(gpu);
        // If address is -1, it means we are rows 8-16 of a sprite in 8x8 mode.
        gpu.vram(address + byte)
    }

    // fn tileset_address(&self, gpu: &Gpu) -> i32 {
    //     if self.sprite.is_some() {
    //         self.sprite_tileset_address(gpu)
    //     } else {
    //         self.bg_tileset_address(gpu)
    //     }
    // }

    fn bg_tileset_address(&self, gpu: &Gpu) -> i32 {
        let y_within_tile = (gpu.current_y + gpu.scroll_y) % 8;
        let base = gpu.lcd_control.translate_bg_set_index(self.tile_index);
        base + y_within_tile * 2
        // // tile_index * 16 + y_within_tile * 2
        // let tile_address = if gpu.lcd_control.bg_set_select() {
        //     ((self.tile_index as i32) << 4) | (y_within_tile << 1)
        // } else {
        //     (0x1000 - ((self.tile_index as i32) << 4)) | (y_within_tile << 1)
        // };
        // 0x8000 + tile_address
    }

    // fn sprite_tileset_address(&self, gpu: &Gpu) -> i32 {
    //     let y_within_tile = gpu.current_y - self.sprite.unwrap().top();
    //     debug_assert_lt!(y_within_tile, 16);
    //     debug_assert_ge!(y_within_tile, 0);
    //     if y_within_tile >= 16 {
    //         return -1;
    //     }
    //     0x8000 + self.sprite.unwrap().tile_index() as i32 * 16 + y_within_tile * 2
    // }
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
