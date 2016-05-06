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
enum GpuMode {
    SclnOAM,
    SclnVram,
    HBlank,
    VBlank,
}

pub struct Gpu {
    mode: GpuMode,
    clock: u32,
    line: u32,
    
    image: image::RgbImage,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            mode: GpuMode::SclnOAM,
            clock: 0,
            line: 0,
            image: image::ImageBuffer::new(160, 144),
        }
    }
    
    pub fn update(&mut self, delta_cycles: u32, memory: &mut Memory) {
        let lcdc = Lcdc::new(memory.read_reg(Register::Lcdc));
        
        // Technically this can only work during vblank
        if !lcdc.enable_display {
            assert!(self.mode == GpuMode::VBlank);
            return;
        }
        
        self.clock += delta_cycles;
        
        match self.mode.clone() {
            GpuMode::SclnOAM => {
                if self.clock >= 80 {
                    // Enter mode 3 (accessing vram)
                    self.clock = 0;
                    self.mode = GpuMode::SclnVram;
                }
            },
            
            GpuMode::SclnVram => {
                if self.clock >= 172 {
                    // Enter HBlank
                    self.mode = GpuMode::HBlank;
                    self.clock = 0;
                    
                    // Render a scanline
                }
            },
            
            GpuMode::HBlank => {
                if self.clock >= 204 {
                    self.clock = 0;
                    self.line += 1;
                    
                    if self.line == 143 {
                        // Enter VBlank
                        self.mode = GpuMode::VBlank;
                        
                        // Upload image to window
                    } else {
                        // Render a new line
                        self.mode = GpuMode::SclnOAM;  
                    }
                }
            },
            
            GpuMode::VBlank => {
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
    }
}
