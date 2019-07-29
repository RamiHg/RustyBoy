//use crate::io_registers::{self, declare_register, declare_register_u8};

use crate::io_registers;
use bitfield::bitfield;
use bitflags::bitflags;
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
    pub struct LcdControl(i32);
    no default BitRange;
    impl Debug;
    u8;
    pub enable_bg, set_enable_bg: 0;
    pub enable_sprites, set_enable_sprites: 1;
    pub large_sprites, set_large_sprites: 2;
    pub bg_map_select, _: 3, 3;
    pub bg_set_id, set_bg_set_id: 4, 4;
    pub enable_window, set_enable_window: 5;
    pub window_map_select, set_window_map_select: 6, 6;
    // Stopping display must be performed during vblank only.
    pub enable_display, set_enable_display: 7;
}

/// LCD Status Register (STAT). 0xFF41.
#[derive(Clone, Copy, FromPrimitive, PartialEq, Debug, Serialize, Deserialize)]
pub enum LcdMode {
    HBlank,
    VBlank,
    ReadingOAM,
    TransferringToLcd,
}

#[derive(Clone, Copy)]
pub enum InterruptType {
    HBlank = 0b1000,
    VBlank = 0b1_0000,
    Oam = 0b10_0000,
    LyIsLyc = 0b100_0000,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct Interrupts: i32 {
        const HBLANK = 0b0000_1000;
        const VBLANK = 0b0001_0000;
        const OAM    = 0b0010_0000;
        const LYC    = 0b0100_0000;
    }
}

bitfield! {
    pub struct LcdStatus(i32);
    no default BitRange;
    impl Debug;
    u8;
    pub into LcdMode, mode, set_mode: 1, 0;
    pub ly_is_lyc, set_ly_is_lyc: 2;
    pub enable_hblank_int, set_enable_hblank_int: 3;
    pub enable_vblank_int, set_enable_vblank_int: 4;
    pub enable_oam_int, set_enable_oam_int: 5;
    pub enable_coincident_int, set_enable_coincident_int: 6;
}

impl io_registers::Register for LcdStatus {
    const ADDRESS: i32 = io_registers::Addresses::LcdStatus as i32;
    fn address(&self) -> i32 {
        Self::ADDRESS
    }

    fn value(&self) -> i32 {
        self.0
    }
    fn set(&mut self, value: i32) {
        let mask = 0b111;
        self.0 = (self.0 as i32 & mask) | (value & !mask);
    }
}

impl_bitfield_bitrange!(LcdStatus);

// BgPalette Register. 0xFF47.
bitfield! {
    pub struct BgPalette(u8);
    u8;
}

impl_bitfield_helpful_traits!(LcdStatus);
impl_serde_bitfield_traits!(LcdStatus);
define_typed_register!(LcdControl, io_registers::Addresses::LcdControl);

define_int_register!(CurrentY, io_registers::Addresses::LcdY);
define_int_register!(Lyc, io_registers::Addresses::LcdYCompare);
define_int_register!(ScrollX, io_registers::Addresses::ScrollX);
define_int_register!(ScrollY, io_registers::Addresses::ScrollY);

from_u8!(LcdMode);
