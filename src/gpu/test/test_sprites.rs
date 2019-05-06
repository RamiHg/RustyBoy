use crate::gpu::sprites::SpriteEntry;
use crate::gpu::Color;

use crate::test::image::*;
use crate::test::*;

use std::path::Path;

fn color_to_row(color: Color) -> u16 {
    let (low, high) = match color {
        Color::White => (0 as u16, 0),
        Color::LightGray => (0xFF, 0),
        Color::DarkGray => (0, 0xFF),
        Color::Black => (0xFF, 0xFF),
    };
    low | (high << 8)
}

struct SpriteBuilder {
    sprite: SpriteEntry,

}

impl SpriteBuilder {
    pub fn new() -> SpriteBuilder {
        SpriteBuilder {
            sprite: SpriteEntry(0),
        }
    }

    pub fn get(self) -> SpriteEntry { self.sprite }

    pub fn pos(mut self, x: i32, y: i32) -> SpriteBuilder {
        self.sprite.set_pos_x(x as u8 + 8);
        self.sprite.set_pos_y(y as u8 + 16);
        self
    }
}

struct ImageBuilder {
    tile_set: Vec<u8>,
    tile_map: Vec<u8>,
    oam: Vec<SpriteEntry>,
}

impl ImageBuilder {
    pub fn new() -> ImageBuilder {
        ImageBuilder {
            tile_set: vec![0; 0x1800],
            tile_map: vec![0; 0x800],
            oam: Vec::new(),
        }
    }

    pub fn add_sprite(mut self, sprite: SpriteBuilder) -> ImageBuilder {
        self.oam.push(sprite.get());
        assert_lt!(self.oam.len(), 40);
        self
    }

    pub fn set_bg_map(mut self, x: usize, y: usize, tile_index: i32) -> ImageBuilder {
        self.tile_map[x + y * 32] = tile_index as u8;
        self
    }

    pub fn color_bg_tile_solid(mut self, tile_index: usize, color: Color) -> ImageBuilder {
        let row = color_to_row(color);
        dbg!(row);
        for i in 0..8 {
            self.tile_set[0x1000 + tile_index * 16 + i * 2 + 0] = (row & 0xFF) as u8;
            self.tile_set[0x1000 + tile_index * 16 + i * 2 + 1] = (row >> 8) as u8;
        }
        self
    }

    pub fn fill_sprite_tile(mut self, index: usize) -> ImageBuilder {
        for i in 0..8 {
            self.tile_set[index * 16 + i * 2 + 0] = 0xFF;
            self.tile_set[index * 16 + i * 2 + 1] = 0xFF;
        }
        self
    }

    pub fn as_test(self) -> TestContext {
        use crate::gpu::registers::LcdControl;
        let mut lcdc = LcdControl(0);
        lcdc.set_enable_bg(true);
        lcdc.set_enable_sprites(true);
        lcdc.set_bg_set(false);
        lcdc.set_enable_display(true);

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
            .set_mem_8bit(io_registers::Addresses::BgPalette as i32, 0b00_10_01_00)
            .set_mem_8bit(io_registers::Addresses::ScrollX as i32, 128)
            .set_mem_8bit(io_registers::Addresses::ScrollY as i32, 0)
    }
}

// /// Tests an empty background with a single square sprite on the top-left corner.
// #[test]
// fn test_simple_sprite_topleft_corner() {
//     let system = ImageBuilder::new()
//         .add_sprite(SpriteBuilder::new().pos(0, 0))
//         //.fill_sprite_tile(0)
//         .set_bg_tiles(0)
//         // .fill_tile(0)
//         .as_test()
//         .wait_for_vsync()
//         .system;
//     dump_system_image(Path::new("./dumps"), "topleft", &system);
// }

/// Tests an empty background with a single square sprite on the top-left corner.
#[test]
fn test_checkerboard() {
    let system = ImageBuilder::new()
        .color_bg_tile_solid(1, Color::LightGray)
        .color_bg_tile_solid(2, Color::DarkGray)
        .set_bg_map(1, 0, 1)
        .set_bg_map(2, 0, 2)
        .as_test()
        .wait_for_vsync()
        .system;
    dump_system_image(Path::new("./dumps"), "topleft", &system);
}
