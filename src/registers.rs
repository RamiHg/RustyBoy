use num_traits::FromPrimitive;

/// Base register trait. Describes registers: their location in memory, etc.
pub trait Register {
    const address: usize;
}

#[derive(FromPrimitive, PartialEq)]
pub enum LcdcModeFlag {
    HBlank,
    VBlank,
    ReadingOAM,
    TransferingToLCD,
}

bitfield! {
    pub struct LcdStatus(u8);
    u8;
    pub into LcdcModeFlag, mode, set_mode: 1, 0;
    is_coincidence_flag, set_is_coincidence_flag: 2;
    pub enable_hblank_int, set_enable_hblank_int: 3;
    pub enable_vblank_int, set_vnable_hblank_int: 4;
    pub enable_oam_int, set_enable_oam_int: 5;
    pub enable_coincident_int, set_enable_coincident_int: 6;
}

bitfield! {
    pub struct LcdControl(u8);
    u8;
    enable_bg, _: 0;
    enable_sprites, _: 1;
    sprite_size_select, _: 2;
    bg_map_select, _: 3;
    bg_set_select, _: 4;
    enable_window, _: 5;
    window_map_select, _: 6;
    // Stopping display must be performed during vblank only.
    pub enable_display, _: 7;
}

/// Implements the Register trait.
macro_rules! declare_register {
    ($x:ident, $address:literal) => {
        // Implement the Register trait.
        impl Register for $x {
            const address: usize = $address;
        }
    };
}

/// Implements the std::Convert::From<u8> trait.
macro_rules! from_u8 {
    ($x:ty) => {
        // Implement conversion from u8.
        impl std::convert::From<u8> for $x {
            fn from(flag: u8) -> $x {
                FromPrimitive::from_u8(flag).unwrap()
            }
        }
    };
}

declare_register!(LcdStatus, 0xFF41);
declare_register!(LcdControl, 0xFF00);

from_u8!(LcdcModeFlag);
