mod fetcher;
mod fifo;
pub mod registers;
mod sprites;

#[cfg(test)]
mod test;

use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;
use crate::util;
use registers::*;

use arrayvec::ArrayVec;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::rc::Rc;

use fetcher::*;
use fifo::*;
use sprites::*;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;

#[derive(Clone, Copy, PartialEq, FromPrimitive)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub fn zero() -> Pixel { Pixel { r: 0, g: 0, b: 0 } }

    pub fn from_values(r: u8, g: u8, b: u8) -> Pixel { Pixel { r, g, b } }
}

#[derive(Clone, Copy, Debug)]
enum DrawingMode {
    /// Regular drawing mode. I.e., just drawing background.
    Bg,
    /// A sprite is visible in the current pixel, and is currently being fetched.
    FetchingSprite,
    /// A sprite is currently fully fetched and being drawn.
    DrawingSprite,
}

#[derive(Clone)]
pub struct Gpu {
    // Registers.
    lcd_control: LcdControl,
    lcd_status: LcdStatus,
    bg_palette: i32,
    sprite_palette_0: i32,
    sprite_palette_1: i32,
    scroll_x: i32,
    scroll_y: i32,
    lyc: i32,
    window_xpos: i32,
    window_ypos: i32,

    current_y: i32,

    pixels_pushed: i32,
    window_ycount: i32,
    drawing_mode: DrawingMode,

    cycle: i32,

    fifo: PixelFifo,
    fetcher: PixelFetcher,
    visible_sprites: ArrayVec<[u8; 10]>,

    // VRAM.
    vram: Rc<RefCell<[u8; 8192]>>,
    oam: Rc<RefCell<[u8; 160]>>,
}

impl Gpu {
    const OAM_ADDR: usize = 8192;

    pub fn new() -> Gpu {
        let mut lcd_status = LcdStatus(0);
        lcd_status.set_mode(LcdMode::ReadingOAM as u8);

        Gpu {
            lcd_control: LcdControl(0x91),
            lcd_status: lcd_status,
            bg_palette: 0xFC,
            sprite_palette_0: 0xFF,
            sprite_palette_1: 0xFF,
            scroll_x: 0,
            scroll_y: 0,
            current_y: 0,
            lyc: 0,
            window_xpos: 0,
            window_ypos: 0,

            drawing_mode: DrawingMode::Bg,
            pixels_pushed: 0,
            window_ycount: 0,

            cycle: 0,

            fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            visible_sprites: ArrayVec::new(),

            // Store OAM with Vram in order to reduce amount of copying.
            vram: Rc::new(RefCell::new([0; 8192])),
            oam: Rc::new(RefCell::new([0; 160])),
        }
    }

    pub fn vram(&self, address: i32) -> u8 { self.vram.borrow()[(address - 0x8000) as usize] }
    fn set_vram(&mut self, address: i32, value: i32) {
        debug_assert!(util::is_8bit(value));
        self.vram.borrow_mut()[(address - 0x8000) as usize] = value as u8;
    }
    pub fn oam(&self, address: i32) -> u8 { self.oam.borrow()[(address - 0xFE00) as usize] }
    fn set_oam(&mut self, address: i32, value: i32) {
        debug_assert!(util::is_8bit(value));
        self.oam.borrow_mut()[(address - 0xFE00) as usize] = value as u8;
    }

    fn can_access_oam(&self) -> bool {
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM | LcdMode::TransferringToLcd
                if self.lcd_control.enable_display() =>
            {
                false
            }
            _ => true,
        }
    }

    fn can_access_vram(&self) -> bool {
        match self.lcd_status.mode() {
            LcdMode::TransferringToLcd if self.lcd_control.enable_display() => false,
            _ => true,
        }
    }

    pub fn is_vsyncing(&self) -> bool { self.lcd_status.mode() == LcdMode::VBlank }

    fn maybe_fire_interrupt(&self, interrupt_type: InterruptType) -> Interrupts {
        let mut fired_interrupts = Interrupts::empty();
        // HW: This is technically not correct for VVlank The STAT VBlank interrupt will fire at any
        // time within the VBlank duration.
        if (interrupt_type as i32 & self.lcd_status.0) != 0 {
            fired_interrupts = Interrupts::STAT;
        }
        if let InterruptType::VBlank = interrupt_type {
            fired_interrupts |= Interrupts::VBLANK;
        }
        fired_interrupts
    }

    pub fn execute_t_cycle(&self, screen: &mut [Pixel]) -> (Gpu, Interrupts) {
        let mut next_state = self.clone();

        if !self.lcd_control.enable_display() {
            next_state.lcd_status.set_mode(LcdMode::ReadingOAM as u8);
            next_state.pixels_pushed = 0;
            next_state.current_y = 0;
            next_state.cycle = 0;
            next_state.fetcher = PixelFetcher::new();
            next_state.fifo = PixelFifo::new();
            return (next_state, Interrupts::empty());
        }
        let mut fire_interrupt = Interrupts::empty();
        let mut next_mode = self.lcd_status.mode();

        next_state.cycle += 1;
        // Remaining minor points: LY=LYC needs to be forced to 0 in certain situations.
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM => {
                self.oam_cycle(&mut next_state);

                if self.cycle == 0 {
                    if self.current_y == self.lyc && self.lyc != 0 {
                        // HW: The LYC interrupt must fire on the cycle the mode becomes OAM.
                        fire_interrupt = self.maybe_fire_interrupt(InterruptType::LyIsLyc);
                    }
                    if self.current_y == 0 {
                        fire_interrupt |= self.maybe_fire_interrupt(InterruptType::Oam);
                    }
                }
                if self.cycle == 20 * 4 {
                    // Switch to LCD transfer.
                    next_state.cycle = 0;
                    next_mode = LcdMode::TransferringToLcd;
                }
            }
            LcdMode::TransferringToLcd => {
                self.lcd_transfer_cycle(&mut next_state, screen);
                //debug_assert!(self.cycle < (43 + 16) * 4);
                if next_state.pixels_pushed == LCD_WIDTH as i32 {
                    // TODO: Must be careful about next state selection in hardware.
                    next_state.pixels_pushed = 0;
                    next_state.cycle = 0;
                    next_mode = LcdMode::HBlank;
                    fire_interrupt = self.maybe_fire_interrupt(InterruptType::HBlank);
                }
            }
            LcdMode::HBlank => {
                if self.cycle >= 51 * 4 {
                    next_state.cycle = 0;
                    next_state.current_y += 1;
                    if next_state.current_y >= LCD_HEIGHT as i32 {
                        debug_assert_eq!(next_state.current_y, LCD_HEIGHT as i32);
                        next_mode = LcdMode::VBlank;
                    } else {
                        next_mode = LcdMode::ReadingOAM;
                        fire_interrupt = self.maybe_fire_interrupt(InterruptType::Oam);
                    }
                }
            }
            LcdMode::VBlank => {
                if self.cycle == 0 {
                    println!("Whatup {}", self.current_y);
                    fire_interrupt = self.maybe_fire_interrupt(InterruptType::Oam);
                    // HW: The VBlank interrupt must fire on the cycle that mode becomes VBlank.
                    if self.current_y == 144 {
                        fire_interrupt |= self.maybe_fire_interrupt(InterruptType::VBlank);
                    }
                    if self.current_y == self.lyc {
                        // HW: The LYC interrupt must fire on the same cycle that mode becomes VB.
                        fire_interrupt |= self.maybe_fire_interrupt(InterruptType::LyIsLyc);
                    }
                }
                if self.current_y == 153 {
                    // TODO: This timing isn't correct. The interrupt actually has to be delayed by
                    // one cycle.
                    // TODO: Also, bit2 of STAT is only true for one cycle.
                    // TODO: This (lyc == 0) also isn't correct. Has to be delayed by another cycle.
                    if self.lyc == 153 || self.lyc == 0 {
                        fire_interrupt = self.maybe_fire_interrupt(InterruptType::LyIsLyc);
                    }
                    next_state.current_y = 0;
                    assert_ne!(self.cycle, (114 - 1) * 4);
                }
                if self.cycle == (114 - 1) * 4 {
                    next_state.cycle = 0;
                    next_state.current_y += 1;
                    if next_state.current_y == 1 {
                        // Switch over to Oam of line 1! Weird timing, I know!
                        next_state.cycle = 0;
                        next_state.current_y = 0;
                        assert_eq!(self.pixels_pushed, 0);
                        next_mode = LcdMode::ReadingOAM;
                        //next_state.scroll_x = (next_state.scroll_x + 1) % 255;
                    }
                }
            }
        }
        next_state
            .lcd_status
            .set_ly_is_lyc(next_state.current_y == self.lyc);

        if next_mode != self.lcd_status.mode() {
            println!("Going to {:?}", next_mode);
        }
        next_state.lcd_status.set_mode(next_mode as u8);
        (next_state, fire_interrupt)
    }

    fn oam_cycle(&self, next_state: &mut Gpu) {
        // Doesn't really matter when we do the oam search - memory is unreadable by CPU.
        if self.cycle == 0 {
            self.start_new_scanline(next_state);
        }
    }

    fn start_new_scanline(&self, next_state: &mut Gpu) {
        next_state.window_ycount = 0;

        next_state.fetcher = PixelFetcher::start_new_scanline(&self);
        next_state.fifo = PixelFifo::start_new_scanline(self.scroll_x);

        if self.lcd_control.enable_sprites() {
            next_state.visible_sprites = sprites::find_visible_sprites(
                &*self.oam.borrow(),
                (self.current_y + self.scroll_y) % 256,
            );
        } else {
            next_state.visible_sprites.clear();
        }
    }

    fn lcd_transfer_cycle(&self, next_state: &mut Gpu, screen: &mut [Pixel]) {
        let mut next_fetcher = self.fetcher.execute_tcycle(&self);
        let mut next_fifo = self.fifo.clone();

        // Handle sprites now. State will be valid regardless of what state sprite-handling is in.
        self.handle_sprites(&mut next_fetcher, &mut next_fifo, next_state);

        // Handle window.
        self.handle_window(next_state);

        if next_fifo.has_pixels() {
            if next_fifo.is_good_pixel() {
                // Push a pixel into the screen.
                let entry = next_fifo.peek();
                if entry.is_sprite || self.lcd_control.enable_bg() {
                    screen[(self.pixels_pushed + self.current_y * LCD_WIDTH as i32) as usize] =
                        self.fifo_entry_to_pixel(next_fifo.peek());
                }

                next_state.pixels_pushed += 1;
            }
            // Pop the pixel regardless if we drew it or not.
            next_fifo = next_fifo.popped();
        }

        if next_fifo.has_room() && next_fetcher.has_data() {
            let row = next_fetcher.get_row();
            next_fifo = next_fifo.pushed(FifoEntry::from_row(row).into_iter());
            next_fetcher = next_fetcher.next()
        }

        next_state.fetcher = next_fetcher;
        next_state.fifo = next_fifo;
    }

    fn handle_window(&self, next_state: &mut Gpu) {
        let next_fetcher = &mut next_state.fetcher;
        let next_fifo = &mut next_state.fifo;

        if self.lcd_control.enable_window()
            && self.window_xpos <= 166
            && self.window_xpos + 7 == self.pixels_pushed
        {
            // Triggered window! Switch to window mode until the end of the line.
            *next_fifo = next_fifo.clone().cleared();
            *next_fetcher = next_fetcher.start_window_mode();
        }
    }

    fn handle_sprites(
        &self,
        next_fetcher: &mut PixelFetcher,
        next_fifo: &mut PixelFifo,
        next_state: &mut Gpu,
    ) {
        let maybe_sprite_index = sprites::get_visible_sprite(
            (self.pixels_pushed + self.scroll_x) % 256,
            &self.visible_sprites,
            self.oam.borrow().as_ref(),
        );
        match self.drawing_mode {
            DrawingMode::Bg => {
                if maybe_sprite_index.is_some() {
                    // Suspend the fifo and fetch the sprite, but only if we have enough pixels in
                    // the first place! Also, if we need to fine x-scroll, do it before any sprite
                    // work.
                    debug_assert!(!next_fifo.is_suspended);
                    let sprite_index = maybe_sprite_index.unwrap();
                    if next_fifo.has_pixels() && next_fifo.is_good_pixel() {
                        next_fifo.is_suspended = true;
                        *next_fetcher = next_fetcher.start_new_sprite(
                            &self,
                            sprite_index as i32,
                            &self.get_sprite(sprite_index),
                        );
                        next_state.drawing_mode = DrawingMode::FetchingSprite;
                    }
                }
            }
            DrawingMode::FetchingSprite => {
                debug_assert!(maybe_sprite_index.is_some());
                // Check if the fetcher is ready.
                if next_fetcher.has_data() {
                    // If so, composite the sprite pixels ontop of the pixels currently in the
                    // fifo.
                    let row = FifoEntry::from_row(next_fetcher.get_row());
                    *next_fetcher = next_fetcher.start_continue_scanline();
                    *next_fifo = next_fifo.clone().combined_with_sprite(row.into_iter(), 0);
                    // Go back to drawing as usual.
                    next_state.drawing_mode = DrawingMode::DrawingSprite;
                }
            }
            DrawingMode::DrawingSprite => {
                // If the sprite is no longer visible, throw away the rest of the sprite-blending
                // pixels and immediately go back to bg.
                if maybe_sprite_index.is_none() {
                    *next_fifo = next_fifo.clear_sprite();
                    next_state.drawing_mode = DrawingMode::Bg;
                }
            }
        }
    }

    fn fifo_entry_to_pixel(&self, entry: FifoEntry) -> Pixel {
        let palette = if entry.is_sprite {
            self.sprite_palette_0
        } else {
            self.bg_palette
        };

        match (palette >> (entry.pixel_index() * 2)) & 0x3 {
            0 => Pixel::from_values(255u8, 255u8, 255u8),
            1 => Pixel::from_values(192u8, 192u8, 192u8),
            2 => Pixel::from_values(96u8, 96u8, 96u8),
            3 | _ => Pixel::from_values(0u8, 0u8, 0u8),
        }
    }

    fn get_sprite(&self, sprite_index: u8) -> SpriteEntry {
        // HW: Might not be possible to do in 1 cycle unless OAM is SRAM
        SpriteEntry::from_slice(&self.oam.borrow()[sprite_index as usize * 4..])
    }
}

impl mmu::MemoryMapped for Gpu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::LcdControl) => Some(self.lcd_control.0 as i32),
                Some(Addresses::LcdStatus) => {
                    let enable_mask = if self.lcd_control.enable_display() {
                        0xFF
                    } else {
                        !0b111
                    };
                    Some(self.lcd_status.0 as i32 & enable_mask)
                }
                Some(Addresses::ScrollX) => Some(self.scroll_x),
                Some(Addresses::ScrollY) => Some(self.scroll_y),
                Some(Addresses::WindowXPos) => Some(self.window_xpos),
                Some(Addresses::WindowYPos) => Some(self.window_ypos),
                Some(Addresses::LcdY) => Some(self.current_y),
                Some(Addresses::LcdYCompare) => Some(self.lyc),
                Some(Addresses::BgPalette) => Some(self.bg_palette),
                Some(Addresses::SpritePalette0) => Some(self.sprite_palette_0),
                Some(Addresses::SpritePalette1) => Some(self.sprite_palette_1),
                _ => None,
            },
            mmu::Location::VRam => {
                if self.can_access_vram() {
                    Some(self.vram(raw) as i32)
                } else {
                    Some(0xFF)
                }
            }
            mmu::Location::OAM => {
                if self.can_access_oam() {
                    Some(self.oam(raw) as i32)
                } else {
                    Some(0xFF)
                }
            }
            _ => None,
        }
    }

    fn write(&mut self, address: mmu::Address, value: i32) -> Option<()> {
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::LcdControl) => {
                    self.lcd_control.0 = value;
                    Some(())
                }
                Some(Addresses::LcdStatus) => {
                    let mask = 0b111;
                    self.lcd_status.0 = (self.lcd_status.0 & mask) | (value & !mask);
                    Some(())
                }
                Some(Addresses::ScrollX) => {
                    self.scroll_x = value;
                    Some(())
                }
                Some(Addresses::ScrollY) => {
                    self.scroll_y = value;
                    Some(())
                }
                Some(Addresses::WindowXPos) => {
                    self.window_xpos = value;
                    Some(())
                }
                Some(Addresses::WindowYPos) => {
                    self.window_ypos = value;
                    Some(())
                }
                Some(Addresses::LcdY) => Some(()),
                Some(Addresses::LcdYCompare) => {
                    self.lyc = value;
                    Some(())
                }
                Some(Addresses::BgPalette) => {
                    self.bg_palette = value;
                    Some(())
                }
                Some(Addresses::SpritePalette0) => {
                    self.sprite_palette_0 = value;
                    Some(())
                }
                Some(Addresses::SpritePalette1) => {
                    self.sprite_palette_1 = value;
                    Some(())
                }
                _ => None,
            },
            mmu::Location::VRam => {
                if self.can_access_vram() {
                    self.set_vram(raw, value);
                }
                Some(())
            }
            mmu::Location::OAM => {
                if self.can_access_oam() {
                    self.set_oam(raw, value);
                }
                Some(())
            }
            _ => None,
        }
    }
}
