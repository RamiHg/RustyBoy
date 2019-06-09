mod test_bg;
mod test_sprites;
mod test_window;
// mod test_timing;

use std::path::Path;

use crate::gpu::sprites::SpriteEntry;
use crate::gpu::{self, *};
use crate::io_registers;
use crate::system::System;
use crate::test::image::*;
use crate::test::*;

pub trait ImageFn = Fn(usize, usize) -> Color;
pub trait TransformFn = Fn(usize, usize) -> (usize, usize);

pub const IDENTITY_TRANSFORM: &TransformFn = &|i, j| (i, j);
pub const WHITE_BG_IMAGE: &ImageFn = &|i, j| Color::White;

pub struct ImageBuilder {
    tile_set: Vec<u8>,
    tile_map: Vec<u8>,
    oam: Vec<SpriteEntry>,
    xscroll: usize,
    yscroll: usize,
    wx: usize,
    wy: usize,
    sprites_enabled: bool,
    window_enabled: bool,

    golden_fn: Option<Box<dyn ImageFn>>,
}

impl ImageBuilder {
    pub fn new() -> ImageBuilder {
        ImageBuilder {
            tile_set: vec![0; 0x1800],
            tile_map: vec![0; 0x800],
            oam: Vec::new(),
            xscroll: 0,
            yscroll: 0,
            wx: 0,
            wy: 0,
            sprites_enabled: false,
            window_enabled: false,

            golden_fn: None,
        }
    }

    pub fn add_sprite(mut self, sprite: SpriteBuilder) -> ImageBuilder {
        assert_lt!(self.oam.len(), 40 - 1);
        let sprite_index = self.oam.len();
        // Give each sprite their own tile.
        let mut entry = sprite.sprite;
        entry.set_tile_index(self.oam.len() as u8);
        // Paint that tile.
        if let Some(mask) = sprite.mask {
            self = self.paint_tile(1, sprite_index, sprite.color, &mask);
        } else {
            self = self.color_tile_solid(1, sprite_index, sprite.color);
        }
        self.oam.push(entry);
        self
    }

    pub fn add_sprites(mut self, sprites: &[SpriteBuilder]) -> ImageBuilder {
        for &sprite in sprites {
            self = self.add_sprite(sprite);
        }
        self
    }

    pub fn xscroll(mut self, xscroll: usize) -> ImageBuilder {
        self.xscroll = xscroll;
        self
    }

    pub fn yscroll(mut self, yscroll: usize) -> ImageBuilder {
        self.yscroll = yscroll;
        self
    }

    pub fn wx(mut self, wx: usize) -> ImageBuilder {
        self.wx = wx;
        self
    }

    pub fn wy(mut self, wy: usize) -> ImageBuilder {
        self.wy = wy;
        self
    }

    pub fn enable_sprites(mut self) -> ImageBuilder {
        self.sprites_enabled = true;
        self
    }

    pub fn enable_window(mut self) -> ImageBuilder {
        self.window_enabled = true;
        self
    }

    pub fn as_test(self) -> TestContext {
        use crate::gpu::registers::LcdControl;
        let mut lcdc = LcdControl(0);
        lcdc.set_enable_bg(true);
        lcdc.set_enable_sprites(self.sprites_enabled);
        lcdc.set_enable_window(self.window_enabled);
        lcdc.set_bg_set_id(0);
        lcdc.set_enable_display(true);
        lcdc.set_window_map_select(0);

        let mut oam = Vec::new();
        for entry in self.oam {
            for i in 0..4 {
                oam.push(((entry.0 >> (i * 8)) & 0xFF) as u8);
            }
        }

        with_default()
            .set_mem_range(0x8000, &self.tile_set)
            .set_mem_range(0x9800, &self.tile_map)
            .set_mem_range(0xFE00, &oam)
            .set_mem_8bit(io_registers::Addresses::LcdControl as i32, lcdc.0)
            .set_mem_8bit(io_registers::Addresses::BgPalette as i32, 0b11_10_01_00)
            .set_mem_8bit(
                io_registers::Addresses::SpritePalette0 as i32,
                0b11_10_01_00,
            )
            .set_mem_8bit(io_registers::Addresses::ScrollX as i32, self.xscroll as i32)
            .set_mem_8bit(io_registers::Addresses::ScrollY as i32, self.yscroll as i32)
            .set_mem_8bit(io_registers::Addresses::WindowXPos as i32, self.wx as i32)
            .set_mem_8bit(io_registers::Addresses::WindowYPos as i32, self.wy as i32)
    }

    /// Image setup controls.

    pub fn build_default_bg(mut self, image_fn: Box<dyn ImageFn>) -> ImageBuilder {
        self = self
            .color_tile_solid(0, 1, Color::LightGray)
            .color_tile_solid(0, 2, Color::DarkGray)
            .color_tile_solid(0, 3, Color::Black);
        // Sample the top-left corner of each tile.
        for j in 0..32 {
            for i in 0..32 {
                self = self.set_bg_map(i, j, image_fn(i * 8, j * 8) as usize);
            }
        }
        self.golden_fn = Some(Box::new(image_fn));
        self
    }

    pub fn golden_fn(mut self, image_fn: Box<dyn ImageFn>) -> ImageBuilder {
        self.golden_fn = Some(Box::new(image_fn));
        self
    }

    /// Assertions.

    pub fn run_and_assert_is_golden_fn(
        self,
        title: impl AsRef<str>,
        transform_fn: impl TransformFn,
    ) {
        let golden = build_golden(self.golden_fn.as_ref().unwrap(), transform_fn);
        let system = self.as_test().wait_for_vsync().wait_for_vsync().system;
        compare_with_golden(title.as_ref(), &system, &golden);
    }

    /// Internal utility functions.

    fn set_bg_map(mut self, x: usize, y: usize, tile_index: usize) -> ImageBuilder {
        self.tile_map[x + y * 32] = tile_index as u8;
        self
    }

    fn color_tile_solid(mut self, tile_set: i32, tile_index: usize, color: Color) -> ImageBuilder {
        let base = if tile_set == 0 { 0x1000 } else { 0 };
        let (low, high) = color_to_row(color);
        for i in 0..8 {
            self.tile_set[base + tile_index * 16 + i * 2 + 0] = low;
            self.tile_set[base + tile_index * 16 + i * 2 + 1] = high;
        }
        self
    }

    // Paints one row.
    fn paint_tile(
        mut self,
        tile_set: i32,
        tile_index: usize,
        color: Color,
        row_mask: &[bool],
    ) -> ImageBuilder {
        let base = if tile_set == 0 { 0x1000 } else { 0 };
        let (low, high) = color_to_row(color);
        let row_bitmask = row_mask
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &x)| if x { acc | (0x80 >> i) } else { acc });
        let (low_masked, high_masked) = (low & row_bitmask, high & row_bitmask);
        for i in 0..1 {
            self.tile_set[base + tile_index * 16 + i * 2 + 0] = low_masked;
            self.tile_set[base + tile_index * 16 + i * 2 + 1] = high_masked;
        }
        self
    }
}

#[derive(Clone, Copy)]
pub struct SpriteBuilder {
    sprite: SpriteEntry,
    color: Color,
    mask: Option<[bool; 8]>,
}

impl SpriteBuilder {
    pub fn new() -> SpriteBuilder {
        SpriteBuilder {
            sprite: SpriteEntry(0),
            color: Color::Black,
            mask: None,
        }
    }

    pub fn with_pos(x: i32, y: i32) -> SpriteBuilder { SpriteBuilder::new().pos(x, y) }

    pub fn get(self) -> SpriteEntry { self.sprite }

    pub fn pos(mut self, x: i32, y: i32) -> SpriteBuilder {
        self.sprite.set_pos_x(x as u8 + 8);
        self.sprite.set_pos_y(y as u8 + 16);
        self
    }

    pub fn color(mut self, color: Color) -> SpriteBuilder {
        self.color = color;
        self
    }

    pub fn mask_row(mut self, _: i32, mask: [i32; 8]) -> SpriteBuilder {
        let mut bool_mask = [false; 8];
        for i in 0..8 {
            bool_mask[i] = mask[i] == 1;
        }
        self.mask = Some(bool_mask);
        self
    }
}

/// Creates a golden image from a base BG fn, and a transformer fn.
pub fn build_golden(image_fn: impl ImageFn, transform_fn: impl TransformFn) -> Vec<gpu::Pixel> {
    use gpu::{Pixel, LCD_HEIGHT, LCD_WIDTH};

    let mut result = Vec::with_capacity(LCD_WIDTH * LCD_HEIGHT);
    for j in 0..LCD_HEIGHT {
        for i in 0..LCD_WIDTH {
            // First, transform the coordinates.
            let (new_i, new_j) = transform_fn(i, j);
            use Color::*;
            result.push(match image_fn(new_i % 256, new_j % 256) {
                White => Pixel::from_values(255u8, 255u8, 255u8),
                LightGray => Pixel::from_values(192u8, 192u8, 192u8),
                DarkGray => Pixel::from_values(96u8, 96u8, 96u8),
                Black => Pixel::from_values(0u8, 0u8, 0u8),
            });
        }
    }
    result
}

/// Tests the current system screen vs a golden image.
pub fn compare_with_golden(test_name: &str, system: &System, golden: &[gpu::Pixel]) {
    if system.get_screen() != golden {
        dump_system_image(Path::new("./failed_tests/gpu"), test_name, system);
        dump_image(
            Path::new("./failed_tests/gpu"),
            format!("{}_golden", test_name).as_str(),
            golden,
        );
        panic!("{} failed.", test_name);
    }
}

fn color_to_row(color: Color) -> (u8, u8) {
    let (low, high) = match color {
        Color::White => (0, 0),
        Color::LightGray => (0xFF, 0),
        Color::DarkGray => (0, 0xFF),
        Color::Black => (0xFF, 0xFF),
    };
    (low, high)
}
