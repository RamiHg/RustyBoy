use memory::*;

use image;

struct Lcdc {
    enable_bg: bool,
    enable_sprites: bool,
    sprite_size: u8,
    bg_map: u8,
    bg_set: u8,
    enable_window: bool,
    window_map: u8,
    enable_display: bool,
}

impl Lcdc {
    pub fn new(val: u8) -> Lcdc {
        Lcdc {
            enable_bg: (val & 0x1) != 0,
            enable_sprites: (val & 0x2) != 0,
            sprite_size: (val & 0x4) >> 2,
            bg_map: (val & 0x8) >> 3,
            bg_set: (val & 0x10) >> 4,
            enable_window: (val & 0x20) != 0,
            window_map: (val & 0x40) >> 6,
            enable_display: (val & 0x80) != 0
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum GpuMode {
    SclnOAM,
    SclnVram,
    HBlank,
    VBlank,
}

const LCD_WIDTH: u32 = 160;
const LCD_HEIGHT: u32 = 144;

#[derive(Clone, Copy)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct Gpu {
    pub mode: GpuMode,
    clock: u32,
    line: u32,
    
    image: [Pixel; LCD_WIDTH as usize * LCD_HEIGHT as usize],
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            mode: GpuMode::SclnOAM,
            clock: 0,
            line: 0,
            image: [Pixel {r:0, g:0, b:0}; (LCD_WIDTH*LCD_HEIGHT) as usize]
        }
    }

    pub fn get_pixel(&self, i: u32, j:u32) -> Pixel {
        self.image[(i + j*LCD_WIDTH) as usize]
    }
    
    pub fn update(&mut self, delta_cycles: u32, memory: &mut Memory) {
        /*
         * Various statuses: 
            HBlank (00): CPU can access display RAM (0x8000 - 0x9FFF)
            VBlank (01): CPU can access display RAM
            0AM Used (10): OAM is being used (0xFE00 - 0xFE9F)
        */

        let lcdc = Lcdc::new(memory.read_reg(Register::Lcdc));
        let mut lcdstatus = memory.read_reg(Register::LcdStatus);
        
        // Technically this can only work during vblank
        if !lcdc.enable_display {
            //assert!(self.mode == GpuMode::VBlank);
            //return;
        }
        
        self.clock += delta_cycles;
        
        match self.mode.clone() {
            GpuMode::SclnOAM => {
                lcdstatus = (lcdstatus & 0xFC) | 0b10;

                if self.clock >= 80 {
                    // Enter mode 3 (accessing vram)
                    self.clock = 0;
                    self.mode = GpuMode::SclnVram;
                }
            },
            
            GpuMode::SclnVram => {
                lcdstatus = (lcdstatus & 0xFC) | 0b11;
                
                if self.clock >= 172 {
                    // Enter HBlank
                    self.mode = GpuMode::HBlank;
                    self.clock = 0;
                    
                    // Render a scanline
                    self.render_scanline(memory);
                }
            },
            
            GpuMode::HBlank => {
                // Signify HBlank status
                lcdstatus = (lcdstatus & 0xFC) | 0b00;

                if self.clock >= 204 {
                    self.clock = 0;
                    self.line += 1;
                    
                    if self.line == 144 {
                        // Enter VBlank
                        self.mode = GpuMode::VBlank;

                        // TODO: When is the interrupt fired? This frame or next?
                        let old_if = memory.read_reg(Register::InterruptFlag);
                        memory.store_reg(Register::InterruptFlag, old_if | 0x1);

                        lcdstatus = (lcdstatus & 0xFC) | 0b01;
                        
                        // Upload image to window
                    } else {
                        // Render a new line
                        self.mode = GpuMode::SclnOAM;  
                    }
                }
            },
            
            GpuMode::VBlank => {
                // Signify VBlank status
                lcdstatus = (lcdstatus & 0xFC) | 0b01;

                if self.clock >= 456 {
                    self.clock = 0;
                    self.line += 1;
                    
                    // It takes 10 lines for VBlank
                    if self.line > 153 {
                        self.mode = GpuMode::SclnOAM;
                        self.line = 0;
                    }
                }
            }
        }

        memory.store_reg(Register::LcdStatus, lcdstatus);
    }

    fn render_scanline(&mut self, memory: &Memory) {
        let scroll_x = memory.read_reg(Register::ScrollX) as u32;
        let scroll_y = memory.read_reg(Register::ScrollY) as u32;

        let lcdc = Lcdc::new(memory.read_reg(Register::Lcdc));
        let palette = memory.read_reg(Register::BgPalette);

        let tilemap_location: usize = if lcdc.bg_map == 0 { 0x9800 } else { 0x9C00 };
        let tileset_location: usize = if lcdc.bg_set == 0 { 0x8800 } else { 0x8000 };

        // Loop over every pixel in the scan line
        for i in 0..LCD_WIDTH {
            // TODO: Rewrite this loop in terms of tiles instead of pixels for a significant optimization
            // Find out which tile we're in
            let tilemap_index: usize = ((scroll_x + i) / 8 + ((scroll_y + self.line) / 8) * 8) as usize;
            assert!(tilemap_index < 1024, "Unexpected tilemap index");

            let tile_unsigned_index = memory.read_general_8(tilemap_location + tilemap_index);
            println!("{}", tile_unsigned_index);
            
            let tile_index = if lcdc.bg_set == 1 {
                ((tile_unsigned_index as i8) as i32 + 128) as usize
            } else {
                tile_unsigned_index as usize
            };
            
            let tile_j = (scroll_y + self.line) % 8;

            // Each row of the tile is 2 bytes, for a total of 16 bytes
            let tile_row_value = memory.read_general_16(
                tileset_location + tile_index * 16 + tile_j as usize * 2);

            let tile_i = (scroll_x + i) % 8;

            let pixel_value = (tile_row_value >> tile_i) & 0x1 |
                (tile_row_value >> (tile_i + 7) & 0x2);

            let mut color = match (palette >> pixel_value) & 0x3 {
                0 => [255u8, 255u8, 255u8],
                1 => [192u8, 192u8, 192u8],
                2 => [96u8, 96u8, 96u8],
                3 => [0u8, 0u8, 0u8],
                _ => panic!()
            };

            self.image[(i + self.line*LCD_WIDTH) as usize] = Pixel {r: color[0],
                g: color[1], b: color[2]};
        }
    }
}
