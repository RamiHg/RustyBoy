mod fetcher;
mod fifo;
pub mod registers;
mod sprites;

#[cfg(test)]
mod test;

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
    lcd_control: LcdControl,
    lcd_status: LcdStatus,
    bg_palette: i32,
    sprite_palette_0: i32,
    sprite_palette_1: i32,
    scroll_x: i32,
    scroll_y: i32,
    lyc: Lyc,
    window_xpos: i32,
    window_ypos: i32,

    current_y: CurrentY,

    pixels_pushed: i32,
    window_ycount: i32,
    drawing_mode: DrawingMode,

    // State related to internal counting state.
    cycle: i32,
    cycles_in_hblank: i32,
    first_frame: bool,

    fifo: PixelFifo,
    fetcher: PixelFetcher,
    visible_sprites: ArrayVec<[u8; 10]>,
    fetched_sprites: [bool; 10],

    // VRAM.
    // vram: Rc<RefCell<[u8; 8192]>>,
    // oam: Rc<RefCell<[u8; 160]>>,
    vram: Rc<RefCell<Vec<u8>>>,
    oam: Rc<RefCell<Vec<u8>>>,
}

impl Gpu {
    pub fn new() -> Gpu {
        let mut lcd_status = LcdStatus(0);
        lcd_status.set_mode(LcdMode::ReadingOAM as u8);

        // This is the state of the GPU after the bootrom completes. The GPU is in the 4th cycle of
        // the vblank mode on line 153 (or 0 during cycle 0).

        Gpu {
            lcd_control: LcdControl(0x91),
            lcd_status,
            bg_palette: 0xFC,
            sprite_palette_0: 0xFF,
            sprite_palette_1: 0xFF,
            scroll_x: 0,
            scroll_y: 0,
            current_y: CurrentY(0),
            lyc: Lyc(0),
            window_xpos: 0,
            window_ypos: 0,

            drawing_mode: DrawingMode::Bg,
            pixels_pushed: 0,
            window_ycount: 0,
            cycles_in_hblank: 0,

            cycle: 0,
            first_frame: false,

            fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            visible_sprites: ArrayVec::new(),
            fetched_sprites: [false; 10],

            // Store OAM with Vram in order to reduce amount of copying.
            vram: Rc::new(RefCell::new(vec![0; 8192])),
            oam: Rc::new(RefCell::new(vec![0; 160])),
        }
    }

    fn enable_display(&mut self) {
        self.first_frame = true;
        self.current_y.0 = 0;
        self.window_ycount = 0;
        // The GPU starts off immediately in HBlank for a couple of cycles.
        self.lcd_status.set_mode(LcdMode::HBlank as u8);
        self.cycles_in_hblank = 0;
        self.cycle = (75 + 0) * 4 - 0;
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
        // HW: This is technically not correct for VVlank The STAT VBlank interrupt will fire at any
        // time within the VBlank duration.
        if (interrupt_type as i32 & self.lcd_status.0) != 0 {
            Interrupts::STAT
        } else {
            Interrupts::empty()
        }
    }

    pub fn execute_t_cycle(&self, tstate: i32, screen: &mut [Pixel]) -> (Gpu, Interrupts) {
        let mut next_state = self.clone();
        //dbg!(self.cycle);
        // println!("Enabled: {}", self.lcd_control.enable_display());
        if !self.lcd_control.enable_display() {
            next_state.lcd_status.set_mode(LcdMode::ReadingOAM as u8);
            next_state.current_y.0 = 0;
            return (next_state, Interrupts::empty());
        }
        //debug_assert_eq!(self.cycle % 4, tstate - 1);

        let mut next_mode = self.lcd_status.mode();
        next_state.cycle += 1;

        // Remaining minor points: LY=LYC needs to be forced to 0 in certain situations.
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM => {
                if self.cycle == 4 {
                    self.start_new_scanline(&mut next_state);
                }
                if next_state.cycle == 20 * 4 {
                    // Switch to LCD transfer.
                    next_state.cycle = 0;
                    next_mode = LcdMode::TransferringToLcd;
                    // TODO: This is currently not exactly right, since we haven't reset self.cycle.
                    // Refactor this entire function into a proper Mealy machine.
                    self.lcd_transfer_cycle(&mut next_state, screen);
                }
            }
            LcdMode::TransferringToLcd => {
                if self.pixels_pushed < LCD_WIDTH as i32 {
                    self.lcd_transfer_cycle(&mut next_state, screen);
                }
                // Moving at tstate == 4 is a hack. But hey, all the acceptance tests pass.
                if next_state.pixels_pushed == LCD_WIDTH as i32 {
                    // Base length of 172 cycles.
                    let mut cycles = 172;
                    // + SCX % 8
                    cycles += self.scroll_x % 8;
                    // At least 6 cycles per sprite.
                    cycles += 6 * self.visible_sprites.len() as i32;
                    // debug_assert_ge!(self.cycle, cycles);
                    if self.visible_sprites.is_empty() {
                        //debug_assert_eq!(self.cycle, cycles + 1);
                    }

                    // debug_assert_ge!(next_state.cycle, 43 * 4);
                    // debug_assert_lt!(next_state.cycle, (43 + 25) * 4);
                    next_mode = LcdMode::HBlank;
                    next_state.cycles_in_hblank = 0;
                }
            }
            LcdMode::HBlank => {
                // This is basically a signal to do the interrupt.
                next_state.cycles_in_hblank += 1;
                // This is a hack so that the CPU can see the new value on this clock.
                if next_state.cycle == 94 * 4 - 1 {
                    next_state.current_y += 1;
                }
                if next_state.cycle == 94 * 4 {
                    debug_assert_le!(next_state.cycles_in_hblank, 51 * 4);

                    next_state.cycle = 0;
                    next_state.pixels_pushed = 0;
                    //next_state.current_y += 1;
                    if next_state.current_y == LCD_HEIGHT as i32 {
                        next_mode = LcdMode::VBlank;
                    } else {
                        next_mode = LcdMode::ReadingOAM;
                    }
                    // If the LCD was just turned on, start on line 0 - and go straight to Mode3.
                    if self.first_frame {
                        next_state.current_y.0 = 0;
                        next_mode = LcdMode::TransferringToLcd;
                        next_state.first_frame = false;
                        self.start_new_scanline(&mut next_state);
                    }
                }
            }
            LcdMode::VBlank => {
                // This is super odd behavior with line 153 - which actually lasts for one mcycle,
                // and switches to line 0.
                if self.current_y == 153 && next_state.cycle == 4 {
                    // debug_assert_eq!(tstate, 4);
                    next_state.current_y.0 = 0;
                }
                // Figure out how to relay LY to the CPU without resorting to this hack (maybe
                // update all registers at 3rd tcycle?)
                if next_state.cycle == 114 * 4 - 1 && self.current_y != 0 {
                    next_state.current_y += 1;
                }
                if next_state.cycle == 114 * 4 {
                    next_state.cycle = 0;
                    if self.current_y == 0 {
                        next_mode = LcdMode::ReadingOAM;
                    } else {
                        // next_state.current_y += 1;
                    }
                }
            }
        }

        if next_mode != self.lcd_status.mode() {
            // trace!(target: "gpu", "Going to {:?}", next_mode);
            // println!(
            //     "Going to {:?}. Y {}. Cycle {}",
            //     next_mode, self.current_y.0, self.cycle
            // );
        }
        next_state.lcd_status.set_mode(next_mode as u8);
        let fire_interrupt = next_state.set_output(tstate % 4);
        if fire_interrupt != Interrupts::empty() {
            trace!(target:"gpu",
                "Firing interrupts {:?}. Mode is {:X?} cycle is {} line is {}",
                fire_interrupt,
                next_state.lcd_status.mode(),
                next_state.cycle,
                next_state.current_y.0
            );
        }

        (next_state, fire_interrupt)
    }

    fn fire_if_lyc_is(&self, y: i32) -> Interrupts {
        if self.lyc.0 == y {
            self.maybe_fire_interrupt(InterruptType::LyIsLyc)
        } else {
            Interrupts::empty()
        }
    }

    // Emulating Moore machine. One day I will refactor this.
    fn set_output(&mut self, t_state: i32) -> Interrupts {
        let mut fire_interrupt = Interrupts::empty();
        let mode = self.lcd_status.mode();
        let is_doing_line = mode == LcdMode::ReadingOAM || mode == LcdMode::VBlank;
        let in_line_first_mcycle = is_doing_line && self.cycle < 4;
        let ly_is_lyc = self.current_y == self.lyc.0;

        // LY=LYC Interrupt.
        if t_state == 0 && self.current_y != 0 {
            self.lcd_status
                .set_ly_is_lyc(ly_is_lyc && !in_line_first_mcycle);
            if is_doing_line && self.cycle == 4 {
                fire_interrupt |= self.fire_if_lyc_is(self.current_y.0);
            }
        } else if t_state == 0 && self.current_y == 0 {
            if self.lcd_status.mode() == LcdMode::VBlank {
                debug_assert_ne!(self.cycle, 0);
                if self.cycle == 4 {
                    self.lcd_status.set_ly_is_lyc(self.lyc == 153);
                    fire_interrupt |= self.fire_if_lyc_is(153);
                } else if self.cycle == 8 {
                    self.lcd_status.set_ly_is_lyc(false);
                } else {
                    self.lcd_status.set_ly_is_lyc(ly_is_lyc);
                    fire_interrupt |= self.fire_if_lyc_is(self.current_y.0);
                }
            } else {
                self.lcd_status
                    .set_ly_is_lyc(ly_is_lyc && !in_line_first_mcycle);
            }
        }
        // VBL Interrupt.
        if self.current_y == 144 && self.cycle == 4 {
            debug_assert_eq!(self.lcd_status.mode(), LcdMode::VBlank);
            fire_interrupt |= Interrupts::VBLANK;
            fire_interrupt |= self.maybe_fire_interrupt(InterruptType::VBlank);
        }
        // OAM Interrupt.
        if is_doing_line {
            let should_fire_oam = match self.current_y.0 {
                1...143 => self.cycle == 0,
                144 | 0 if self.cycle == 4 => true,
                _ if self.cycle == 12 => true,
                // 0 if mode == LcdMode::VBlank && self.cycle == 12 => true,
                _ => false,
            };
            if should_fire_oam {
                fire_interrupt |= self.maybe_fire_interrupt(InterruptType::Oam);
            }
        }
        // // HBlank interrupt.
        // if mode == LcdMode::HBlank && self.cycles_in_hblank == 0 {
        //     fire_interrupt |= self.maybe_fire_interrupt(InterruptType::HBlank);
        // }
        if mode == LcdMode::HBlank && self.cycles_in_hblank == 0 {
            fire_interrupt |= self.maybe_fire_interrupt(InterruptType::HBlank);
        }

        fire_interrupt
    }

    fn start_new_scanline(&self, next_state: &mut Gpu) {
        next_state.window_ycount = 0;

        next_state.fetcher = PixelFetcher::start_new_scanline(&self);
        next_state.fifo = PixelFifo::start_new_scanline(self.scroll_x);

        next_state.fetched_sprites = [false; 10];

        if self.lcd_control.enable_sprites() {
            next_state.visible_sprites = sprites::find_visible_sprites(
                &*self.oam.borrow(),
                (self.current_y + self.scroll_y) % 256,
                self.lcd_control.large_sprites(),
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

        if next_fifo.has_room() && next_fetcher.has_data() {
            if !next_fetcher.is_initial_fetch {
                let row = next_fetcher.get_row();
                next_fifo = next_fifo.pushed(FifoEntry::from_row(row));
            }
            next_fetcher = next_fetcher.next()
        }

        if next_fifo.has_pixels() {
            if next_fifo.is_good_pixel() {
                // Push a pixel into the screen.
                let entry = next_fifo.peek();
                if entry.is_sprite() || self.lcd_control.enable_bg() {
                    screen[(self.pixels_pushed + self.current_y * LCD_WIDTH as i32) as usize] =
                        self.fifo_entry_to_pixel(next_fifo.peek());
                }

                next_state.pixels_pushed += 1;
            }
            // Pop the pixel regardless if we drew it or not.
            next_fifo = next_fifo.popped();
        }
        // if self.current_y == 0 {
        //     println!("Cycle {}. Pushed {}", self.cycle, self.pixels_pushed);
        //     dbg!(self.fetcher.mode);
        // }
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
        let true_x = (self.pixels_pushed + self.scroll_x) % 256;
        let maybe_visible_sprite_array_index = sprites::get_visible_sprite(
            true_x,
            &self.visible_sprites,
            &self.fetched_sprites,
            self.oam.borrow().as_ref(),
        );
        let has_visible_sprite = maybe_visible_sprite_array_index.is_some();
        let sprite_array_index = maybe_visible_sprite_array_index.unwrap_or(0);
        let sprite_index = if let Some(index) = maybe_visible_sprite_array_index {
            self.visible_sprites[index]
        } else {
            0
        };
        // if has_visible_sprite && self.current_y == 0 {
        //     println!(
        //         "Am in {:?} pixel {}. Maybe is {:?}.",
        //         self.drawing_mode, self.pixels_pushed, sprite_index
        //     );
        // }
        match self.drawing_mode {
            DrawingMode::Bg => {
                if has_visible_sprite {
                    debug_assert!(self.lcd_control.enable_sprites());
                    // Suspend the fifo and fetch the sprite, but only if we have enough pixels in
                    // the first place! Also, if we need to fine x-scroll, do it before any sprite
                    // work.
                    if next_fifo.has_pixels_or_suspended() && next_fifo.is_good_pixel() {
                        next_fifo.is_suspended = true;
                        *next_fetcher = next_fetcher.start_new_sprite(
                            &self,
                            sprite_index as i32,
                            &self.get_sprite(sprite_index),
                        );
                        next_state.drawing_mode = DrawingMode::FetchingSprite;
                    }
                } else {
                    next_fifo.is_suspended = false;
                }
            }
            DrawingMode::FetchingSprite => {
                debug_assert!(self.lcd_control.enable_sprites());
                debug_assert!(has_visible_sprite);
                debug_assert!(!self.fetched_sprites[sprite_array_index]);
                // Check if the fetcher is ready.
                if next_fetcher.has_data() {
                    // If so, composite the sprite pixels ontop of the pixels currently in the
                    // fifo.
                    let sprite = self.get_sprite(sprite_index);
                    let row = FifoEntry::from_sprite_row(
                        next_fetcher.get_row(),
                        true,
                        sprite.priority(),
                        sprite.palette(),
                        sprite.flip_x(),
                    )
                    .take(8)
                    .skip(sprites::pixels_behind(true_x, &sprite));
                    // println!("Skipping {}", sprites::pixels_behind(true_x, &sprite));
                    // Only keep enough pixels to
                    *next_fetcher = next_fetcher.start_continue_scanline();
                    // for i in 0..8 {
                    //     if self.current_y == 0 {
                    //         dbg!(next_fifo.fifo[i]);
                    //     }
                    // }
                    *next_fifo = next_fifo.clone().combined_with_sprite(row);

                    // Go back to drawing as usual.
                    next_state.drawing_mode = DrawingMode::Bg;
                    next_state.fetched_sprites[sprite_array_index] = true;
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
}

impl mmu::MemoryMapped for Gpu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::LcdControl) => Some(self.lcd_control.0 as i32),
                Some(Addresses::LcdStatus) => {
                    let stat = self.lcd_status;
                    // This is really weird behavior. But alas, replicate.
                    if stat.mode() == LcdMode::ReadingOAM && self.cycle < 4 {
                        //  stat.set_mode(LcdMode::HBlank as u8);
                    } else if stat.mode() == LcdMode::VBlank
                        && self.cycle < 4
                        && self.current_y == 144
                    {
                        //  stat.set_mode(LcdMode::HBlank as u8);
                    }
                    let enable_mask = if self.lcd_control.enable_display() {
                        0xFF
                    } else {
                        !0b111
                    };
                    Some((stat.0 as i32 & enable_mask) | 0x80)
                }
                Some(Addresses::ScrollX) => Some(self.scroll_x),
                Some(Addresses::ScrollY) => Some(self.scroll_y),
                Some(Addresses::WindowXPos) => Some(self.window_xpos),
                Some(Addresses::WindowYPos) => Some(self.window_ypos),
                Some(Addresses::LcdY) => Some(self.current_y.0),
                Some(Addresses::LcdYCompare) => Some(self.lyc.0),
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
                    let was_enabled = self.lcd_control.enable_display();
                    self.lcd_control.0 = value;
                    if !was_enabled && self.lcd_control.enable_display() {
                        self.enable_display();
                    }
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
                    self.lyc.0 = value;
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

#[cfg(test)]
impl Gpu {
    pub fn stat_mut(&mut self) -> &mut LcdStatus { &mut self.lcd_status }
}
