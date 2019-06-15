use arrayvec::ArrayVec;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::rc::Rc;

mod fetcher;
mod fifo;
pub mod options;
pub mod registers;
mod sprites;

#[cfg(test)]
mod test;

use crate::io_registers::Register as _;
use crate::mmu;
use crate::system::{self, TState};
use crate::util;

use fetcher::*;
use fifo::*;
use registers::*;
use sprites::*;

pub use options::*;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;

/// TODO: Refactor this entire file.

#[derive(Clone, Copy, PartialEq, FromPrimitive)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub fn zero() -> Pixel { Pixel { r: 0, g: 0, b: 0 } }

    pub fn from_values(r: u8, g: u8, b: u8) -> Pixel { Pixel { r, g, b } }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum DrawingMode {
    /// Regular drawing mode. I.e., just drawing background.
    Bg,
    /// A sprite is visible in the current pixel, and is currently being fetched.
    FetchingSprite,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Gpu {
    // Registers.
    bg_palette: i32,
    sprite_palette_0: i32,
    sprite_palette_1: i32,
    scroll_x: i32,
    scroll_y: i32,
    window_xpos: i32,
    window_ypos: i32,

    window_ycount: i32,
    drawing_mode: DrawingMode,

    fifo: PixelFifo,
    fetcher: PixelFetcher,
    visible_sprites: ArrayVec<[u8; 10]>,
    fetched_sprites: [bool; 10],

    // VRAM.
    vram: Rc<RefCell<Vec<u8>>>,
    oam: Rc<RefCell<Vec<u8>>>,

    pub options: Options,

    state: InternalState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct InternalState {
    // Registers.
    pub lcd_control: LcdControl,
    pub lcd_status: LcdStatus,
    /// The line number that is visible from outside the PPU. Can often be different than the
    /// true current line (e.g. 153). Maybe fix that.
    pub external_y: CurrentY,
    pub lyc: Lyc,
    // Rendering state.
    /// The actual internal accurate line number.
    pub current_y: i32,
    pub counter: i32,
    pub pixels_pushed: i32,
    pub entered_oam: bool,
    pub mode: LcdMode,
    pub oam_lock: bool,
    pub vram_lock: bool,
    // Interrupts.
    pub interrupts: Interrupts,
    pub fire_interrupt: bool,
    stat_asserted: bool,
    old_stat_asserted: bool,
    fire_interrupt_oam_hack: bool,
    // Misc.
    pub is_first_frame: bool,
    pub hblank_delay_tcycles: i32,

    options: Options,
}

impl InternalState {
    fn with_options(options: &Options) -> InternalState {
        InternalState {
            lcd_control: LcdControl(0x91),
            lcd_status: LcdStatus(0x80),
            external_y: CurrentY(0),
            lyc: Lyc(0),

            current_y: 153,
            counter: 403,
            pixels_pushed: 160,
            entered_oam: false,
            mode: LcdMode::HBlank,
            oam_lock: false,
            vram_lock: false,

            interrupts: Interrupts::empty(),
            fire_interrupt: false,
            stat_asserted: false,
            old_stat_asserted: false,
            fire_interrupt_oam_hack: false,

            is_first_frame: false,
            hblank_delay_tcycles: 8,

            options: *options,
        }
    }

    pub fn update_tick(&mut self, t_state: TState) {
        self.counter += 1;

        if self.counter == self.options.num_tcycles_in_line {
            self.counter = 0;
            self.current_y += 1;
            if self.current_y == 154 {
                self.current_y = 0;
                self.is_first_frame = false;
            }
        }

        if !self.lcd_control.enable_display() {
            return;
        }

        if let TState::T2 | TState::T4 = t_state {
            return;
        }

        self.entered_oam = (self.current_y == 0 && self.counter == 4)
            || (self.current_y > 0 && self.current_y <= 144 && self.counter == 0);

        if self.entered_oam {
            debug_assert_eq!(t_state, TState::T1);
        }

        if self.counter == 0 {
            self.hblank_delay_tcycles = self.options.num_hblank_delay_tcycles;
        }

        if self.counter == 0 {
            self.oam_lock = true;
        } else if self.counter == 80 {
            self.oam_lock = false;
        } else if self.counter == 82 {
            self.oam_lock = true;
            self.vram_lock = true;
        }
        if self.hblank_delay_tcycles < 8 || self.current_y >= 144 {
            self.oam_lock = false;
            self.vram_lock = false;
        }

        // Do operations that should only happen at every PPU tick.
        // Update the external LY.
        self.update_external_y();

        // Update interrupts.
        self.update_interrupts(t_state);

        // if (self.current_y == 144 && self.counter >= 4) || self.current_y >= 145 {
        //     self.mode = LcdMode::VBlank;
        // } else if self.hblank_delay_tcycles < self.options.num_hblank_delay_tcycles {
        //     self.mode = LcdMode::HBlank;
        // } else if self.counter == 0 {
        //     // Initial mcycle is always HBlank.
        //     self.mode = LcdMode::HBlank;
        // } else if self.counter == 4 {
        //     if !self.is_first_frame || self.current_y != 0 {
        //         self.mode = LcdMode::ReadingOAM;
        //     }
        // } else if self.counter == self.options.transfer_mode_start_tcycle {
        //     self.mode = LcdMode::TransferringToLcd;
        // }
        if self.counter == 0 {
            self.mode = LcdMode::HBlank;
        }
        if self.counter == 4 && (!self.is_first_frame || self.current_y != 0) {
            self.mode = LcdMode::ReadingOAM;
        }
        if self.counter == 84 {
            self.mode = LcdMode::TransferringToLcd;
        }
        if self.counter > 84 && self.pixels_pushed == 160 {
            self.mode = LcdMode::HBlank;
        }
        if (self.current_y == 144 && self.counter >= 4) || self.current_y >= 145 {
            self.mode = LcdMode::VBlank;
        }
    }

    pub fn update_tock(&mut self, t_state: TState, bus: &mut mmu::MemoryBus) {
        if self.counter == 0 {
            self.pixels_pushed = 0;
        }
        if let TState::T1 | TState::T3 = t_state {
            self.lcd_status.set_mode(self.mode as u8);
            self.lcd_status
                .set_ly_is_lyc(self.lyc == self.required_lyc_for_interrupt());
        }
        let stat_asserted = (self.interrupts.bits() & self.lcd_status.0) != 0;
        self.fire_interrupt = stat_asserted && !self.old_stat_asserted;
        if let TState::T1 = t_state {
            self.old_stat_asserted = stat_asserted;
        }
        // Handle bus requests now.
        self.handle_bus_reads(bus);
        self.handle_bus_writes(bus);
    }

    pub fn update_tock_after_render(&mut self, bus: &mut mmu::MemoryBus) {
        // TODO: Delete this method.
        if self.pixels_pushed == LCD_WIDTH as i32 && self.hblank_delay_tcycles > 0 {
            self.hblank_delay_tcycles -= 1;
        }

        // TODO: Try to remove the late reads.
        self.handle_bus_reads(bus);
        self.handle_bus_writes(bus);
    }

    pub fn update_interrupts(&mut self, t_state: TState) {
        self.interrupts
            .remove(Interrupts::HBLANK | Interrupts::VBLANK | Interrupts::LYC);
        if self.hblank_delay_tcycles < 7 {
            self.interrupts |= Interrupts::HBLANK;
        }
        if (self.current_y == 144 && self.counter >= 4) || self.current_y >= 145 {
            self.interrupts |= Interrupts::VBLANK;
        }
        if self.lyc == self.required_lyc_for_interrupt() && self.current_y > 0 {
            // TODO: To fix the ly_lyc_write wilbert tests, I have to recheck for LYC interrupts
            // after a CPU write has happened.
            self.interrupts |= Interrupts::LYC;
        }
        if let TState::T1 = t_state {
            self.fire_interrupt_oam_hack = (self.interrupts.bits() & self.lcd_status.0) != 0;
            self.interrupts.remove(Interrupts::OAM);
            if self.entered_oam {
                self.interrupts |= Interrupts::OAM;
            }
        }
    }

    /// Sets the LY register that is visible from outside the PPU.
    /// Prerequisites: Only called on a PPU cycle (i.e. T1 and T3).
    fn update_external_y(&mut self) {
        self.external_y.0 = if self.current_y == 0 {
            0
        } else if self.current_y == 153 && self.counter >= 4 {
            0
        } else {
            self.current_y
        };
    }

    /// Returns the necessary LYC value for the LY=LYC interrupt to fire in this cycle. Will return
    /// 256 if it is impossible for LY=LYC to fire.
    fn required_lyc_for_interrupt(&self) -> i32 {
        if self.current_y == 0 {
            // This is useless.
            match self.counter {
                0..=3 => 0,
                _ => self.external_y.0,
            }
        } else if self.current_y == 153 {
            match self.counter {
                0..=3 => 256, // Impossible.
                4..=7 => 153,
                8..=11 => 256, // Impossible.
                _ => 0,
            }
        } else {
            match self.counter {
                0..=3 => 256, // Impossible.
                _ => self.current_y,
            }
        }
    }

    fn handle_bus_reads(&self, bus: &mut mmu::MemoryBus) {
        bus.maybe_read(self.lcd_control);
        bus.maybe_read(self.external_y);
        bus.maybe_read(self.lyc);
        if bus.reads_from(self.lcd_status) {
            let enable_mask = if self.lcd_control.enable_display() {
                0xFF
            } else {
                !0b111
            };
            bus.data_latch = (self.lcd_status.0 & enable_mask) | 0x80;
        }
    }

    fn handle_bus_writes(&mut self, bus: &mut mmu::MemoryBus) {
        self.lcd_control.set_from_bus(bus);
        self.lcd_status.set_from_bus(bus);
        self.lyc.set_from_bus(bus);
    }

    pub fn update_tock_disabled(&mut self, bus: &mut mmu::MemoryBus) {
        self.counter = 7;
        self.current_y = 0;
        self.external_y.0 = 0;
        self.is_first_frame = true;
        self.hblank_delay_tcycles = self.options.num_hblank_delay_tcycles;
        self.pixels_pushed = 0;
        self.mode = LcdMode::HBlank;
        self.lcd_status.set_mode(self.mode as u8);
        self.oam_lock = false;
        self.vram_lock = false;
        self.handle_bus_reads(bus);
        self.handle_bus_writes(bus);
    }
}

impl Gpu {
    pub fn new() -> Gpu {
        // This is the state of the GPU after the bootrom completes. The GPU is in the 4th cycle of
        // the vblank mode on line 153 (or 0 during cycle 0).

        Gpu {
            bg_palette: 0xFC,
            sprite_palette_0: 0xFF,
            sprite_palette_1: 0xFF,
            scroll_x: 0,
            scroll_y: 0,
            window_xpos: 0,
            window_ypos: 0,

            drawing_mode: DrawingMode::Bg,
            window_ycount: 0,
            fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            visible_sprites: ArrayVec::new(),
            fetched_sprites: [false; 10],

            // Store OAM with Vram in order to reduce amount of copying.
            vram: Rc::new(RefCell::new(vec![0; 8192])),
            oam: Rc::new(RefCell::new(vec![0; 160])),

            options: Options::new(),

            state: InternalState::with_options(&Options::new()),
        }
    }

    pub fn hack(&self) -> bool { self.state.fire_interrupt_oam_hack }

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

    pub fn at_vblank(&self) -> bool { self.state.counter == 4 && self.state.current_y == 144 }

    fn can_access_oam(&self) -> bool { !self.state.oam_lock }
    fn can_access_vram(&self) -> bool { !self.state.vram_lock }

    pub fn is_vsyncing(&self) -> bool { self.lcd_status().mode() == LcdMode::VBlank }

    pub fn execute_tcycle_tick(&mut self, t_state: TState, bus: &mut mmu::MemoryBus) {
        self.state.update_tick(t_state);
    }

    pub fn execute_tcycle_tock(
        &mut self,
        t_state: TState,
        bus: &mut mmu::MemoryBus,
        screen: &mut [Pixel],
    ) -> system::Interrupts {
        if !self.state.lcd_control.enable_display() {
            self.state.update_tock_disabled(bus);
            return system::Interrupts::empty();
        }

        self.state.update_tock(t_state, bus);
        if self.state.counter == 80 {
            self.start_new_scanline();
        }

        if self.state.counter >= 82 //self.options.transfer_start_tcycle
            && self.state.hblank_delay_tcycles > 7
        //&& self.state.mode == LcdMode::TransferringToLcd
        {
            self.lcd_transfer_cycle(screen);
        }

        self.state.update_tock_after_render(bus);

        if self.state.fire_interrupt {
            system::Interrupts::STAT
        } else {
            system::Interrupts::empty()
        }
    }

    fn start_new_scanline(&mut self) {
        self.state.pixels_pushed = 0;

        if self.fetcher.window_mode {
            // debug_assert_gt!(self.current_y(), 0);
            self.window_ycount += 1;
        }

        self.fetcher = PixelFetcher::start_new_scanline(&self);
        self.fifo = PixelFifo::start_new_scanline(self.scroll_x);

        self.fetched_sprites = [false; 10];

        if self.lcd_control().enable_sprites() {
            self.visible_sprites = sprites::find_visible_sprites(
                &*self.oam.borrow(),
                self.state.current_y,
                self.lcd_control().large_sprites(),
            );
        } else {
            self.visible_sprites.clear();
        }
    }

    fn lcd_transfer_cycle(&mut self, screen: &mut [Pixel]) {
        self.fetcher = self.fetcher.execute_tcycle(&self);

        // Handle sprites now. State will be valid regardless of what state sprite-handling is in.
        self.handle_sprites();

        // // Handle window.
        self.handle_window();

        if self.fifo.has_room() && self.fetcher.has_data() {
            let row = self.fetcher.get_row();
            self.fifo
                .push(FifoEntry::from_row(row, self.fetcher.window_mode));
            self.fetcher = self.fetcher.next();
        }

        if self.fifo.has_pixels() && self.state.counter >= self.options.transfer_start_tcycle {
            if self.fifo.is_good_pixel() {
                // Push a pixel into the screen.
                let entry = self.fifo.peek();
                if (entry.is_sprite() || self.lcd_control().enable_bg())
                    && self.current_y() < LCD_HEIGHT as i32
                //&& !self.state.is_first_frame
                {
                    debug_assert_ge!(self.state.hblank_delay_tcycles, 7);
                    debug_assert_lt!(self.current_y(), LCD_HEIGHT as i32);
                    debug_assert_lt!(self.pixels_pushed(), LCD_WIDTH as i32);
                    screen[(self.pixels_pushed() + self.current_y() * LCD_WIDTH as i32) as usize] =
                        self.fifo_entry_to_pixel(self.fifo.peek());
                }

                self.state.pixels_pushed += 1;
            }
            // Pop the pixel regardless if we drew it or not.
            self.fifo.pop();
        }
    }

    fn handle_window(&mut self) {
        if self.lcd_control().enable_window()
            && self.window_xpos <= 166
            && self.window_xpos - 7 == self.pixels_pushed()
            && self.current_y() >= self.window_ypos
            && self.fifo.is_good_pixel()
            && !self.fetcher.window_mode
        {
            // Triggered window! Switch to window mode until the end of the line.
            self.fifo.clear();
            self.fetcher.start_window_mode();
        }
    }

    fn handle_sprites(&mut self) {
        let maybe_visible_sprite_array_index = sprites::get_visible_sprite(
            self.pixels_pushed(),
            &self.visible_sprites,
            &self.fetched_sprites,
            self.oam.borrow().as_ref(),
        );
        let has_visible_sprite = maybe_visible_sprite_array_index.is_some();

        let sprite_index = if let Some(index) = maybe_visible_sprite_array_index {
            self.visible_sprites[index]
        } else {
            0
        };
        match self.drawing_mode {
            DrawingMode::Bg => {
                if has_visible_sprite {
                    debug_assert!(self.lcd_control().enable_sprites());
                    // Suspend the fifo and fetch the sprite, but only if we have enough pixels in
                    // the first place! Also, if we need to fine x-scroll, do it before any sprite
                    // work.
                    if self.fifo.enough_for_sprite() && self.fifo.is_good_pixel() {
                        self.fifo.is_suspended = true;
                        self.fetcher = self.fetcher.start_new_sprite(
                            &self,
                            sprite_index as i32,
                            &self.get_sprite(sprite_index),
                        );
                        self.drawing_mode = DrawingMode::FetchingSprite;
                    }
                } else {
                    self.fifo.is_suspended = false;
                }
            }
            DrawingMode::FetchingSprite => {
                let sprite_array_index = maybe_visible_sprite_array_index.unwrap();
                debug_assert!(self.lcd_control().enable_sprites());
                debug_assert!(has_visible_sprite);
                debug_assert!(!self.fetched_sprites[sprite_array_index]);
                // Check if the fetcher is ready.
                if self.fetcher.has_data() {
                    // If so, composite the sprite pixels ontop of the pixels currently in the fifo.
                    let sprite = self.get_sprite(sprite_index);
                    let row = FifoEntry::from_sprite_row(
                        self.fetcher.get_row(),
                        true,
                        sprite.priority(),
                        sprite.palette(),
                        sprite.flip_x(),
                    )
                    .take(8)
                    .skip(sprites::pixels_behind(self.pixels_pushed(), &sprite));
                    // Only keep enough pixels to
                    self.fetcher = self.fetcher.continue_scanline();
                    self.fifo = self.fifo.clone().combined_with_sprite(row);

                    // Go back to drawing as usual.
                    self.drawing_mode = DrawingMode::Bg;
                    self.fetched_sprites[sprite_array_index] = true;
                }
            }
        }
    }

    fn fifo_entry_to_pixel(&self, entry: FifoEntry) -> Pixel {
        let palette = if entry.is_sprite() {
            if entry.palette() == 0 {
                self.sprite_palette_0
            } else {
                self.sprite_palette_1
            }
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

    fn current_y(&self) -> i32 { self.state.current_y }
    fn lcd_control(&self) -> &LcdControl { &self.state.lcd_control }
    fn lcd_status(&self) -> LcdStatus { self.state.lcd_status }
    fn pixels_pushed(&self) -> i32 { self.state.pixels_pushed }
}

impl mmu::MemoryMapped for Gpu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::ScrollX) => Some(self.scroll_x),
                Some(Addresses::ScrollY) => Some(self.scroll_y),
                Some(Addresses::WindowXPos) => Some(self.window_xpos),
                Some(Addresses::WindowYPos) => Some(self.window_ypos),
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
                // if raw == 0x8000 && self.vram(0x8000) as i32 != value {
                //     println!("Setting VRAM to {:X?}", value);
                // }
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

#[cfg(test)]
impl Gpu {
    pub fn stat(&self) -> i32 { self.state.lcd_status.0 }
    pub fn ctrl(&self) -> i32 { self.state.lcd_control.0 }
    pub fn y(&self) -> i32 { self.state.external_y.0 }
    pub fn lyc(&self) -> i32 { self.state.lyc.0 }

    pub fn ctrl_mut(&mut self) -> &mut LcdControl { &mut self.state.lcd_control }
    pub fn stat_mut(&mut self) -> &mut LcdStatus { &mut self.state.lcd_status }
}
