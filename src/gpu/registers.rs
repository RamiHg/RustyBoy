//use crate::io_registers::{self, declare_register, declare_register_u8};

use crate::io_registers;
use bitfield::bitfield;
use num_derive::FromPrimitive;

// Column viewports wrap around

/*
 * Starts at FE00
 * OAM Entry (4 bytes)
 *  Pos X (1 byte)
 *  Pos Y (1 byte)
 *  Tile Number (1 byte)
 *  Priority
 *  Flip X
 *  FLip Y
 *  Palette
 */

/*
 * Clock: 4 cycles
 *
 * OAM Search: 20 clocks
 * Pixel transfer: 43+ clocks
 * HBlank: 51- clocks
 * VBlank = 10 lines
 *
 * 114 * 154 = 17,556 clocks per screen
 */

/* Visible Sprites:
 *  oam.x != 0
 *  not LY + 16 >= oam.y
 *  not LY + 16 < oam.y + h
 */

/* Pixel FIFO (4Mhz)
 * Every 4mhz step, shift out pixel, send to LCD
 * 8 pixels
 * Pauses unless it contains more than 8 pixels
 *
 * WHen FIFO empty: Fetch (2Mhz)
 *  Read Tile Number (2 cycle)
 *  Read Data 0: (2 cycle)
 *  Read Data 1: (2 cycle)
 *  Put 8 pixels in upper FIFO half
 * From that, can construct 8 new pixels
 *
 * Scrolling: Simply discard SCX pixels
 *
 * Window:
 *  When X = WX, Completely clear FIFO
 * Sprites:
 *  Temporarily suspend FIFO, Switch to fetching sprite data
 *  Overlay it with first 8 pixels
 *
 *
 * Pixel fifo stores original combination of pixel information and source
 * Applying palette only done at very end when pixel is shifted out
 */

/// LCD Control Register (LCDC). 0xFF00.
bitfield! {
    pub struct LcdControl(u8);
    u8;
    enable_bg, _: 0;
    enable_sprites, _: 1;
    sprite_size_select, _: 2;
    pub bg_map_select, _: 3;
    pub bg_set_select, _: 4;
    enable_window, _: 5;
    window_map_select, _: 6;
    // Stopping display must be performed during vblank only.
    pub enable_display, _: 7;
}

impl LcdControl {
    pub fn bg_map_address(&self) -> i32 {
        if self.bg_map_select() {
            0x9C00
        } else {
            0x9800
        }
    }

    pub fn bg_set_address(&self) -> i32 {
        if self.bg_set_select() {
            0x8000
        } else {
            0x8800
        }
    }
}

/// LCD Status Register (STAT). 0xFF41.
#[derive(FromPrimitive, PartialEq, Debug)]
pub enum LcdMode {
    HBlank,
    VBlank,
    ReadingOAM,
    TransferringToLcd,
}

bitfield! {
    pub struct LcdStatus(u8);
    u8;
    pub into LcdMode, mode, set_mode: 1, 0;
    is_coincidence_flag, set_is_coincidence_flag: 2;
    pub enable_hblank_int, set_enable_hblank_int: 3;
    pub enable_vblank_int, set_vnable_hblank_int: 4;
    pub enable_oam_int, set_enable_oam_int: 5;
    pub enable_coincident_int, set_enable_coincident_int: 6;
}

// BgPalette Register. 0xFF47.
bitfield! {
    pub struct BgPalette(u8);
    u8;
}

//declare_register!(LcdStatus, io_registers::Addresses::LcdStatus);
declare_register_u8!(LcdControl, io_registers::Addresses::LcdControl);

from_u8!(LcdMode);
