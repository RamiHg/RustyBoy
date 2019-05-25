mod fetcher;
mod fifo;
pub mod registers;
mod sprites;

#[cfg(test)]
mod test;

use crate::mmu;
use crate::system::{self, TState};
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

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Options {
    pub cycle_after_enable: i32,
    pub vblank_cycle: i32,
    pub hblank_cycle: i32,
    pub oam_1_143_cycle: i32,
    pub oam_144_cycle: i32,
    pub oam_145_152_cycle: i32,
    pub oam_0_cycle: i32,
    pub oam_0_vblank_cycle_first: i32,
    pub oam_0_vblank_cycle_second: i32,
    pub oam_cycles: i32,

    pub use_fetcher_initial_fetch: bool,

    pub num_tcycles_in_line: i32,

    pub hblank_delay_tcycles: i32,
}

impl Options {
    fn new() -> Options {
        // Options {
        //     cycle_after_enable: 73 * 4 + 4,
        //     vblank_cycle: 0,
        //     hblank_cycle: 0,
        //     oam_1_143_cycle: 0, //-4,
        //     oam_144_cycle: 0,
        //     oam_145_152_cycle: 0,
        //     oam_0_cycle: 0,
        //     oam_0_vblank_cycle_first: 0,
        //     oam_0_vblank_cycle_second: 8,

        //     use_fetcher_initial_fetch: false,
        // }
        Options {
            cycle_after_enable: 74 * 4 + 3,
            vblank_cycle: 0,
            hblank_cycle: 0,
            oam_1_143_cycle: 0,
            oam_144_cycle: 0,
            oam_145_152_cycle: 8,
            oam_0_cycle: 0,
            oam_0_vblank_cycle_first: 0,
            oam_0_vblank_cycle_second: 8,
            oam_cycles: 21,
            use_fetcher_initial_fetch: true,

            num_tcycles_in_line: 456,
            hblank_delay_tcycles: 16,
            ..Default::default()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Gpu {
    // Registers.
    bg_palette: i32,
    sprite_palette_0: i32,
    sprite_palette_1: i32,
    scroll_x: i32,
    scroll_y: i32,
    lyc: Lyc,
    window_xpos: i32,
    window_ypos: i32,

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

    pub options: Options,

    state: InternalState,
}

#[derive(Clone, Serialize, Deserialize)]
struct InternalState {
    // Registers.
    pub lcd_control: LcdControl,
    pub lcd_status: LcdStatus,
    pub current_y: CurrentY,
    // Rendering state.
    pub counter: i32,
    pub pixels_pushed: i32,
    pub entered_oam: bool,
    pub mode: LcdMode,
    // Interrupts.
    pub interrupts: Interrupts,
    pub fire_interrupt: bool,
    // Misc.
    pub is_first_frame: i32,
    pub hblank_delay_tcycles: i32,
}

impl InternalState {
    pub fn update_tick(&mut self, t_state: TState) {
        self.counter += 1;

        if self.counter == self.options.num_tcycles_in_line {
            self.counter = 0;
            self.current_y += 1;
            if self.current_y == 154 {
                self.current_y.0 = 0;
                self.is_first_frame = false;
            }
        }

        if !self.lcd_control.enabled() {
            return;
        }

        self.entered_oam =
            self.current_y == 0 && self.counter == 4 || self.current_y <= 144 && self.counter == 0;
        let in_vblank = self.current_y >= 144;

        // Everything is a state machine..
        let mode = match self.counter {
            _ if (self.current_y == 144 && self.counter >= 4) || self.current_y >= 145 => {
                LcdMode::VBlank
            }
            _ if self.hblank_delay_tcycles < 8 => LcdMode::HBlank,
            0 => LcdMode::HBlank,
            4 if !self.is_first_frame && self.current_y != 0 => LcdMode::ReadingOAM,
            84 => LcdMode::TransferringToLcd,
            _ => self.lcd_status.mode(),
        };

        if self.counter == 0 {
            self.hblank_delay_tcycles = 8;
        }

        // Update interrupts.
        self.update_interrupts(t_state);
    }

    pub fn update_tock(&mut self, t_state: TState, bus: &mut mmu::MemoryBus) {
        if self.counter == 0 {
            self.pixels_pushed = 0;
        }

        if let TState::T1 | TState::T3 = t_state {
            self.lcd_status.set_mode(self.mode);
        }

        // Handle bus requests now.
        self.handle_bus_reads(bus);

        if self.pixels_pushed == LCD_WIDTH as i32 && self.hblank_delay_tcycles > 0 {
            self.hblank_delay_tcycles -= 1;
        }

        // TODO: Try to remove the late reads.
        self.handle_bus_reads(bus);
    }

    pub fn update_interrupts(&mut self, t_state: TState) {
        let mut interrupts = Interrupts::empty();
        self.interrupts
            .remove(Interrupts::HBLANK | Interrupts::VBLANK | Interrupts::LYC);
        if self.hblank_delay_tcycles < 7 {
            self.interrupts |= Interrupts::HBLANK;
        }
        if self.current_y == 144 && self.counter >= 4 || self.counter >= 145 {
            self.interrupts |= Interrupts::VBLANK;
        }
        // todo: ly=lyc
        if t_state == TState::T1 {
            self.fire_interrupt = (self.interrupts as i32 & self.lcd_status.0) != 0;
            self.interrupts.remove(Interrupts::OAM);
            if self.entered_oam {
                self.interrupts |= Interrupts::OAM;
            }
        }
    }

    fn handle_bus_reads(&self, bus: &mut mmu::MemoryBus) {
        bus.maybe_read(self.lcd_control);
        bus.maybe_read(self.current_y);
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
    }
}

impl Default for InternalState {
    fn default() -> InternalState {
        InternalState {
            lcd_control: LcdControl(0x91),
            lcd_status: LcdStatus(0x80),
            current_y: CurrentY(0),

            counter: 403,
            pixels_pushed: 160,
            entered_oam: false,
            mode: LcdMode::HBlank,

            interrupts: Interrupts::empty(),
            fire_interrupt: false,

            is_first_frame: true,
            hblank_delay_tcycles: 8,
        }
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

            options: Options::default(),

            state: InternalState::default(),
        }
    }

    pub fn enable_display(&mut self) {
        self.first_frame = true;
        self.window_ycount = 0;
        self.cycles_in_hblank = 1;
        self.cycle = self.options.cycle_after_enable;
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

    pub fn execute_tcycle_tick(
        &mut self,
        t_state: i32,
        screen: &mut [Pixel],
        bus: &mut mmu::MemoryBus,
    ) {
        let state = &mut self.state;
        debug_assert_gt!(state.current_y, 0);
    }

    pub fn execute_t_cycle(&self, tstate: i32, screen: &mut [Pixel]) -> (Gpu, Interrupts) {
        let mut next_state = self.clone();
        if !self.lcd_control.enable_display() {
            next_state.lcd_status.set_mode(LcdMode::ReadingOAM as u8);
            next_state.current_y.0 = 0;
            return (next_state, Interrupts::empty());
        }
        // debug_assert_eq!(self.cycle % 4, tstate % 4);

        let mut next_mode = self.lcd_status.mode();
        next_state.cycle += 1;

        // Remaining minor points: LY=LYC needs to be forced to 0 in certain situations.
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM => {
                if self.cycle == 4 {
                    next_state.start_new_scanline();
                }
                if next_state.cycle == self.options.oam_cycles * 4 {
                    // Switch to LCD transfer.
                    next_state.cycle = 0;
                    next_mode = LcdMode::TransferringToLcd;
                    // TODO: This is currently not exactly right, since we haven't reset self.cycle.
                    // Refactor this entire function into a proper Mealy machine.
                    next_state.lcd_transfer_cycle(screen);
                }
            }
            LcdMode::TransferringToLcd => {
                if self.pixels_pushed < LCD_WIDTH as i32 {
                    next_state.lcd_transfer_cycle(screen);
                }
                if next_state.pixels_pushed == LCD_WIDTH as i32 {
                    next_mode = LcdMode::HBlank;
                    next_state.cycles_in_hblank = 0;
                }
            }
            LcdMode::HBlank => {
                // This is basically a signal to do the interrupt.
                next_state.cycles_in_hblank += 1;

                if next_state.cycle == 93 * 4 && !self.first_frame {
                    next_state.current_y += 1;
                }

                if next_state.cycle == 94 * 4 {
                    next_state.cycle = 0;
                    next_state.pixels_pushed = 0;
                    // next_state.current_y += 1;
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
                        next_state.start_new_scanline();
                        next_state.lcd_transfer_cycle(screen);
                    }
                }
            }
            LcdMode::VBlank => {
                if next_state.cycle == 113 * 4 && self.current_y() != 0 {
                    next_state.current_y += 1;
                }
                // This is super odd behavior with line 153 - which actually lasts for one mcycle,
                // and switches to line 0.
                if self.current_y() == 153 && self.cycle == 0 {
                    next_state.current_y.0 = 0;
                }
                if next_state.cycle == 114 * 4 {
                    next_state.cycle = 0;
                    if next_state.current_y == 0 {
                        next_mode = LcdMode::ReadingOAM;
                    }
                }
            }
        }

        if next_mode != self.lcd_status.mode() {
            // println!(
            //     "Going to {:?}. Y {}. Cycle {}. Tstate {}.",
            //     next_mode, self.current_y(), next_state.cycle, tstate
            // );
        }
        next_state.lcd_status.set_mode(next_mode as u8);
        let fire_interrupt = next_state.poll_interrupts();
        if fire_interrupt != Interrupts::empty() {
            trace!(target: "gpu",
                "Firing interrupts {:?}. Mode is {:X?} cycle is {} line is {}. TState is {}",
                fire_interrupt,
                next_state.lcd_status.mode(),
                next_state.cycle,
                next_state.current_y.0,
                tstate
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
    fn poll_interrupts(&mut self) -> Interrupts {
        use LcdMode::*;
        let mut fire_interrupt = Interrupts::empty();
        let mode = self.lcd_status.mode();
        let is_doing_line = mode == LcdMode::ReadingOAM || mode == LcdMode::VBlank;
        let in_line_first_mcycle = is_doing_line && self.cycle < 4;
        let ly_is_lyc = self.current_y() == self.lyc.0;

        let in_last_hblank =
            mode == HBlank && self.cycle >= 93 * 4 || mode == VBlank && self.cycle < 4;

        // LY=LYC Interrupt.
        if self.current_y() == 0 {
            if self.lcd_status.mode() == LcdMode::VBlank {
                debug_assert_ne!(self.cycle, 0);
                if self.cycle == 4 {
                    self.lcd_status.set_ly_is_lyc(self.lyc == 153);
                    fire_interrupt |= self.fire_if_lyc_is(153);
                } else if self.cycle == 8 {
                    self.lcd_status.set_ly_is_lyc(false);
                } else if self.cycle == 12 {
                    self.lcd_status.set_ly_is_lyc(ly_is_lyc);
                    fire_interrupt |= self.fire_if_lyc_is(0);
                }
            } else {
                self.lcd_status.set_ly_is_lyc(ly_is_lyc);
            }
        } else if self.current_y() != 0 {
            self.lcd_status.set_ly_is_lyc(ly_is_lyc && !in_last_hblank);
            if is_doing_line && self.cycle == 4 {
                debug_assert_ne!(self.current_y(), 153);
                fire_interrupt |= self.fire_if_lyc_is(self.current_y());
            }
        }
        // VBL Interrupt.
        if self.current_y() == 144 && self.cycle == self.options.vblank_cycle {
            debug_assert_eq!(self.lcd_status.mode(), LcdMode::VBlank);
            fire_interrupt |= Interrupts::VBLANK;
            fire_interrupt |= self.maybe_fire_interrupt(InterruptType::VBlank);
        }
        // OAM Interrupt.
        let should_fire_oam = match self.current_y() {
            1...143 => {
                if self.options.oam_1_143_cycle < 0 {
                    match mode {
                        HBlank => self.cycle == 94 * 4 + self.options.oam_1_143_cycle,
                        _ => false,
                    }
                } else {
                    mode == ReadingOAM && self.cycle == self.options.oam_1_143_cycle
                }
            }
            144 => {
                if self.options.oam_144_cycle < 0 {
                    mode == HBlank && self.cycle == 94 * 4 + self.options.oam_144_cycle
                } else {
                    mode == VBlank && self.cycle == self.options.oam_144_cycle
                }
            }
            145...152 => {
                if self.options.oam_145_152_cycle < 0 {
                    self.cycle == 114 * 4 + self.options.oam_145_152_cycle
                } else {
                    self.cycle == self.options.oam_145_152_cycle
                }
            }
            0 if mode == LcdMode::VBlank => {
                self.cycle == self.options.oam_0_vblank_cycle_first
                    || self.cycle == self.options.oam_0_vblank_cycle_second
            }
            0 if mode == LcdMode::ReadingOAM => self.cycle == self.options.oam_0_cycle,
            0 => false,
            153 => false,
            _ => panic!(),
        };
        if should_fire_oam {
            fire_interrupt |= self.maybe_fire_interrupt(InterruptType::Oam);
        }
        // HBlank interrupt.
        let should_fire_hblank = if self.options.hblank_cycle < 0 {
            mode == LcdMode::TransferringToLcd && self.will_hblank()
        } else {
            mode == LcdMode::HBlank
                && self.cycles_in_hblank == self.options.hblank_cycle
                && !self.first_frame
        };
        if should_fire_hblank {
            fire_interrupt |= self.maybe_fire_interrupt(InterruptType::HBlank);
        }

        fire_interrupt
    }

    fn start_new_scanline(&mut self) {
        self.window_ycount = 0;
        self.pixels_pushed = 0;

        self.fetcher = PixelFetcher::start_new_scanline(&self);
        self.fifo = PixelFifo::start_new_scanline(self.scroll_x);

        self.fetched_sprites = [false; 10];

        if self.lcd_control.enable_sprites() {
            self.visible_sprites = sprites::find_visible_sprites(
                &*self.oam.borrow(),
                self.current_y(),
                self.lcd_control.large_sprites(),
            );
        } else {
            self.visible_sprites.clear();
        }
    }

    fn will_hblank(&self) -> bool {
        self.pixels_pushed == LCD_WIDTH as i32 - 1
            && self.fifo.has_pixels()
            && self.fifo.is_good_pixel()
    }

    fn lcd_transfer_cycle(&mut self, screen: &mut [Pixel]) {
        let mut next_fetcher = self.fetcher.execute_tcycle(&self);
        let mut next_fifo = self.fifo.clone();

        // Handle sprites now. State will be valid regardless of what state sprite-handling is in.
        self.handle_sprites(&mut next_fetcher, &mut next_fifo);

        // Handle window.
        self.handle_window();

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
                    screen[(self.pixels_pushed + self.current_y() * LCD_WIDTH as i32) as usize] =
                        self.fifo_entry_to_pixel(next_fifo.peek());
                }

                self.pixels_pushed += 1;
            }
            // Pop the pixel regardless if we drew it or not.
            next_fifo = next_fifo.popped();
        }
        self.fetcher = next_fetcher;
        self.fifo = next_fifo;
    }

    fn handle_window(&mut self) {
        let next_fetcher = &mut self.fetcher;
        let next_fifo = &mut self.fifo;

        if self.lcd_control.enable_window()
            && self.window_xpos <= 166
            && self.window_xpos + 7 == self.pixels_pushed
        {
            // Triggered window! Switch to window mode until the end of the line.
            *next_fifo = next_fifo.clone().cleared();
            *next_fetcher = next_fetcher.start_window_mode();
        }
    }

    fn handle_sprites(&mut self, next_fetcher: &mut PixelFetcher, next_fifo: &mut PixelFifo) {
        let true_x = self.pixels_pushed;
        let maybe_visible_sprite_array_index = sprites::get_visible_sprite(
            true_x,
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
                    debug_assert!(self.lcd_control.enable_sprites());
                    // Suspend the fifo and fetch the sprite, but only if we have enough pixels in
                    // the first place! Also, if we need to fine x-scroll, do it before any sprite
                    // work.
                    if next_fifo.enough_for_sprite() && next_fifo.is_good_pixel() {
                        next_fifo.is_suspended = true;
                        *next_fetcher = next_fetcher.start_new_sprite(
                            &self,
                            sprite_index as i32,
                            &self.get_sprite(sprite_index),
                        );
                        self.drawing_mode = DrawingMode::FetchingSprite;
                    }
                } else {
                    next_fifo.is_suspended = false;
                }
            }
            DrawingMode::FetchingSprite => {
                let sprite_array_index = maybe_visible_sprite_array_index.unwrap();
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
                    //     if self.current_y() == 0 {
                    //         dbg!(next_fifo.fifo[i]);
                    //     }
                    // }
                    *next_fifo = next_fifo.clone().combined_with_sprite(row);

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

    fn current_y(&self) -> i32 { self.state.current_y.0 }
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
