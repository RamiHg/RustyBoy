use crate::gpu;
use crate::gpu::sprites::SpriteEntry;
use crate::gpu::Color;
use crate::system::System;

use crate::test::image::*;
use crate::test::*;

use num_traits::FromPrimitive as _;
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
    xscroll: usize,
    yscroll: usize,
}

impl ImageBuilder {
    pub fn new() -> ImageBuilder {
        ImageBuilder {
            tile_set: vec![0; 0x1800],
            tile_map: vec![0; 0x800],
            oam: Vec::new(),
            xscroll: 0,
            yscroll: 0,
        }
    }

    pub fn add_sprite(mut self, sprite: SpriteBuilder) -> ImageBuilder {
        self.oam.push(sprite.get());
        assert_lt!(self.oam.len(), 40);
        self
    }

    pub fn set_bg_map(mut self, x: usize, y: usize, tile_index: usize) -> ImageBuilder {
        self.tile_map[x + y * 32] = tile_index as u8;
        self
    }

    pub fn color_bg_tile_solid(mut self, tile_index: usize, color: Color) -> ImageBuilder {
        let row = color_to_row(color);
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

    pub fn xscroll(mut self, xscroll: usize) -> ImageBuilder {
        self.xscroll = xscroll;
        self
    }

    pub fn yscroll(mut self, yscroll: usize) -> ImageBuilder {
        self.yscroll = yscroll;
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
            .set_mem_8bit(io_registers::Addresses::BgPalette as i32, 0b11_10_01_00)
            .set_mem_8bit(io_registers::Addresses::ScrollX as i32, self.xscroll as i32)
            .set_mem_8bit(io_registers::Addresses::ScrollY as i32, self.yscroll as i32)
    }
}

fn make_checkerboard_image(
    golden_fner: impl Fn(usize, usize) -> Color,
) -> (ImageBuilder, Vec<gpu::Pixel>) {
    let mut builder = ImageBuilder::new()
        .color_bg_tile_solid(1, Color::LightGray)
        .color_bg_tile_solid(2, Color::DarkGray)
        .color_bg_tile_solid(3, Color::Black);
    for j in 0..32 {
        for i in 0..32 {
            builder = builder.set_bg_map(i, j, simple_checkerboard(i, j) as usize);
        }
    }
    let golden = image::make_fn_image(golden_fner);
    (builder, golden)
}

fn compare_with_golden(test_name: &str, system: &System, golden: Vec<gpu::Pixel>) {
    if system.get_screen() != golden.as_slice() {
        dump_system_image(Path::new("./failed_tests/gpu"), test_name, &system);
        dump_screen(Path::new("./failed_tests/gpus"), test_name, &golden);
        panic!("{} failed.", test_name);
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
fn test_simple_checkerboard() {
    let (builder, golden) = make_checkerboard_image(|i, j| simple_checkerboard(i / 8, j / 8));
    let system = builder
        .as_test()
        .wait_for_vsync()
        .wait_for_vsync()
        .wait_for_vsync()
        .system;
    compare_with_golden("simple_checkerboard", &system, golden);
}

#[test]
fn test_large_xscroll_checkerboard() {
    for xscroll in 0..32 {
        let (builder, golden) = make_checkerboard_image(|i, j| {
            simple_checkerboard(((i + xscroll * 8) / 8) % 32, j / 8)
        });
        let system = builder
            .xscroll(xscroll * 8)
            .as_test()
            .wait_for_vsync()
            .system;
        compare_with_golden(
            format!("large_{}_xscroll", xscroll).as_str(),
            &system,
            golden,
        );
    }
}

#[test]
fn test_fine_yscroll_checkerboard() {
    for yscroll in 0..256 {
        let (builder, golden) = make_checkerboard_image(move |i, j| {
            simple_checkerboard(i / 8, (j + yscroll) % (32 * 8))
        });
        let system = builder.yscroll(yscroll).as_test().wait_for_vsync().system;
        compare_with_golden(
            format!("fine_{}_yscroll", yscroll).as_str(),
            &system,
            golden,
        );
    }
}

fn simple_checkerboard(i: usize, j: usize) -> Color {
    let mut color = (((i + j) % 2) == 0) as usize;
    if i == j {
        color += 1;
    }
    if color > 0 && (j % 2) == 0 {
        color += 1;
    }
    Color::from_usize(color).unwrap()
}