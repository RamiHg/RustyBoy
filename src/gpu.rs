mod fetcher;
mod registers;
mod sprites;

use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;
use registers::*;

use arrayvec::ArrayVec;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::rc::Rc;

use fetcher::*;
use sprites::*;

pub const LCD_WIDTH: i32 = 160;
pub const LCD_HEIGHT: i32 = 144;

#[derive(Clone, Copy)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub fn zero() -> Pixel { Pixel { r: 0, g: 0, b: 0 } }

    pub fn from_values(r: u8, g: u8, b: u8) -> Pixel { Pixel { r, g, b } }
}

#[derive(Clone)]
pub struct Gpu {
    // Registers.
    lcd_control: LcdControl,
    lcd_status: LcdStatus,
    bg_palette: i32,
    scroll_x: i32,
    scroll_y: i32,
    lyc: i32,

    current_y: i32,
    current_x: i32,

    pixels_pushed: i32,
    pixels_scrolled: i32,

    cycle: i32,

    fifo: PixelFifo,
    fetcher: PixelFetcher,
    visible_sprites: ArrayVec<[u8; 10]>,

    // VRAM.
    vram: Rc<RefCell<[u8; 8192 + 160]>>,
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
            scroll_x: 0,
            scroll_y: 0,
            current_y: 0,
            lyc: 0,

            current_x: 0,
            pixels_pushed: 0,
            pixels_scrolled: 0,

            cycle: 0,

            fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            visible_sprites: ArrayVec::new(),

            // Store OAM with Vram in order to reduce amount of copying.
            vram: Rc::new(RefCell::new([0; 8192 + 160])),
        }
    }

    pub fn vram(&self, address: i32) -> i32 {
        self.vram.borrow()[(address - 0x8000) as usize] as i32
    }
    fn set_vram(&mut self, address: i32, value: i32) {
        self.vram.borrow_mut()[(address - 0x8000) as usize] = value as u8;
    }
    pub fn oam(&self, address: i32) -> i32 {
        self.vram.borrow()[(address - 0xFE00 + 8192) as usize] as i32
    }
    fn set_oam(&mut self, address: i32, value: i32) {
        self.vram.borrow_mut()[(address - 0xFE00 + 8192) as usize] = value as u8;
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

    fn maybe_fire_interrupt(&self, interrupt_type: InterruptType) -> Interrupts {
        let mut fired_interrupts = Interrupts::empty();
        // HW: This is technically not correct for VVlank The STAT VBlank interrupt will fire at any
        // time within the VBlank duration.
        if (interrupt_type as u8 & self.lcd_status.0) != 0 {
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
            next_state.current_x = 0;
            next_state.current_y = 0;
            next_state.pixels_scrolled = 0;
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
                    // todo: move me somewhere better
                    next_state.pixels_scrolled = self.scroll_x % 8;
                }
            }
            LcdMode::TransferringToLcd => {
                self.lcd_transfer_cycle(&mut next_state, screen);
                //debug_assert!(self.cycle < (43 + 16) * 4);
                if next_state.pixels_pushed == LCD_WIDTH {
                    // TODO: Must be careful about next state selection in hardware.
                    next_state.pixels_pushed = 0;
                    next_state.pixels_scrolled = 0;
                    next_state.current_x = 0;
                    next_state.cycle = 0;
                    next_state.fetcher.reset();
                    next_state.fifo = PixelFifo::new();
                    next_mode = LcdMode::HBlank;
                    fire_interrupt = self.maybe_fire_interrupt(InterruptType::HBlank);
                }
            }
            LcdMode::HBlank => {
                if self.cycle >= 51 * 4 {
                    next_state.cycle = 0;
                    next_state.current_y += 1;
                    if next_state.current_y >= LCD_HEIGHT {
                        debug_assert_eq!(next_state.current_y, LCD_HEIGHT);
                        next_mode = LcdMode::VBlank;
                    } else {
                        next_mode = LcdMode::ReadingOAM;
                        fire_interrupt = self.maybe_fire_interrupt(InterruptType::Oam);
                    }
                }
            }
            LcdMode::VBlank => {
                if self.cycle == 0 {
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
                        assert_eq!(self.current_x, 0);
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
            // println!("Going to {:?}", next_mode);
        }
        next_state.lcd_status.set_mode(next_mode as u8);
        (next_state, fire_interrupt)
    }

    fn oam_cycle(&self, next_state: &mut Gpu) {
        // Doesn't really matter when we do the oam search - memory is unreadable by CPU.
        if self.cycle == 0 {
            next_state.visible_sprites =
                sprites::find_visible_sprites(&self.vram.borrow()[8192..], self.current_y);
        }
    }

    fn lcd_transfer_cycle(&self, next_state: &mut Gpu, screen: &mut [Pixel]) {
        let fetcher_is_ready = self.fetcher.is_ready();

        let mut next_fifo = self.fifo.clone();
        let mut next_fetcher = self.fetcher.execute_tcycle_mut(&self);
        let mut new_len = self.fifo.len();

        let mut pixel = None;

        let maybe_sprite_index = sprites::get_visible_sprite(
            self.pixels_pushed,
            &self.visible_sprites,
            &self.vram.borrow()[Gpu::OAM_ADDR..],
        );
        if let Some(sprite_index) = maybe_sprite_index {
            if self.fifo.len() < 8 {
                // Keep fetching pixels until we have 8 pixels in the fifo.
                debug_assert!(!self.fifo.is_suspended);
            } else {
                // Check if we've already directed the fetcher to get the sprite.
                if self.fetcher.is_fetching_sprite() && self.fetcher.is_ready() {
                    // Done with the sprite! Mix it in and unsuspend the fifo.
                    next_fifo = next_fifo.combined_with_sprite(self.fetcher.get_row().into_iter());
                    pixel = Some(self.fifo_entry_to_pixel(next_fifo.peek()));
                    new_len -= 1;
                } else if !self.fetcher.is_fetching_sprite() {
                    // Suspend the fifo, switch the fetcher to the sprite.
                    next_fifo.suspend();
                    next_fetcher.reset();
                    next_fetcher.start_sprite(self.get_sprite(sprite_index));
                }
            }
        }
        // TODO: Remove this extra cycle delay.
        else {
            // Easy case: fifo has more than 8 pixels. We can simply pop a pixel off.
            if self.fifo.len() > 8 {
                let entry = self.fifo.peek();
                pixel = Some(self.fifo_entry_to_pixel(entry));
                new_len -= 1;
            } else if self.fifo.len() > 0 && fetcher_is_ready {
                pixel = Some(self.fifo_entry_to_pixel(self.fifo.peek()));
                new_len -= 1;
            }
        }

        // Push from fetcher to fifo.
        if pixel.is_some() {
            next_fifo = self.fifo.popped();
        };
        if new_len <= 8 && self.fetcher.is_ready() {
            next_fifo = next_fifo.pushed(
                self.fetcher
                    .get_row()
                    .into_iter()
                    .skip(self.pixels_scrolled as usize),
            );
            next_state.pixels_scrolled = 0;
        }
        // Start fetching new tile.
        if (new_len <= 8 && self.fetcher.is_ready()) || self.fetcher.is_idle() {
            next_fetcher.reset();

            next_fetcher.start(self.compute_bg_tile_address(next_state.current_x));
            next_state.current_x += 8;
        }
        next_state.fetcher = next_fetcher;
        next_state.fifo = next_fifo;
        // Actually push the pixel on the screen.
        if let Some(pixel) = pixel {
            debug_assert!(self.pixels_pushed < LCD_WIDTH);
            screen[(self.pixels_pushed + self.current_y * LCD_WIDTH) as usize] = pixel;
            next_state.pixels_pushed += 1;
        }
    }

    fn fifo_entry_to_pixel(&self, entry: FifoEntry) -> Pixel {
        match (self.bg_palette >> (entry.pixel_index * 2)) & 0x3 {
            0 => Pixel::from_values(255u8, 255u8, 255u8),
            1 => Pixel::from_values(192u8, 192u8, 192u8),
            2 => Pixel::from_values(96u8, 96u8, 96u8),
            3 | _ => Pixel::from_values(0u8, 0u8, 0u8),
        }
    }

    /// There are 20x18 tiles. Each tile is 16 bytes.
    /// The background map is 32x32 tiles (32 * 32 = 1KB)
    fn compute_bg_tile_address(&self, x: i32) -> i32 {
        let x = ((x + self.scroll_x) / 8) % 32;
        let map_index = x + self.current_y / 8 * 32;
        self.lcd_control.translate_bg_map_index(map_index)
    }

    fn get_sprite(&self, sprite_index: u8) -> SpriteEntry {
        // HW: Might not be possible to do in 1 cycle unless OAM is SRAM
        SpriteEntry::from_slice(&self.vram.borrow()[Gpu::OAM_ADDR + sprite_index as usize * 4..])
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
                Some(Addresses::LcdY) => Some(self.current_y),
                Some(Addresses::LcdYCompare) => Some(self.lyc),
                Some(Addresses::BgPallette) => Some(self.bg_palette),
                _ => None,
            },
            mmu::Location::VRam => {
                if self.can_access_vram() {
                    Some(self.vram(raw))
                } else {
                    Some(0xFF)
                }
            }
            mmu::Location::OAM => {
                if self.can_access_oam() {
                    Some(self.oam(raw))
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
                    self.lcd_control.0 = value as u8;
                    Some(())
                }
                Some(Addresses::LcdStatus) => {
                    let mask = 0b111;
                    self.lcd_status.0 = (self.lcd_status.0 & mask) | (value as u8 & !mask);
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
                Some(Addresses::LcdY) => Some(()),
                Some(Addresses::LcdYCompare) => {
                    self.lyc = value;
                    Some(())
                }
                Some(Addresses::BgPallette) => {
                    self.bg_palette = value;
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

#[derive(Clone, Copy, Debug)]
pub struct FifoEntry {
    pixel_index: u8,
    is_sprite: bool,
    is_s1: bool,
}

impl FifoEntry {
    pub fn new() -> FifoEntry {
        FifoEntry {
            pixel_index: 0,
            is_sprite: false,
            is_s1: false,
        }
    }

    pub fn from_data(pixel_index: u8, is_sprite: bool, is_s1: bool) -> FifoEntry {
        debug_assert_lt!(pixel_index, 4);
        FifoEntry {
            pixel_index,
            is_sprite,
            is_s1,
        }
    }

    pub fn pixel_index(&self) -> u8 { self.pixel_index }
}

// A sad attempt to make a copyable fifo.
#[derive(Clone, Copy, Debug)]
struct PixelFifo {
    fifo: [FifoEntry; 16],
    cursor: i8,
    is_suspended: bool,
}

impl PixelFifo {
    pub fn new() -> PixelFifo {
        PixelFifo {
            fifo: [FifoEntry::new(); 16],
            cursor: 0,
            is_suspended: false,
        }
    }

    pub fn peek(&self) -> FifoEntry { self.fifo[0] }
    pub fn len(&self) -> usize {
        if self.is_suspended {
            16
        } else {
            self.cursor as usize
        }
    }

    pub fn suspend(&mut self) { self.is_suspended = true; }

    pub fn combined_with_sprite(
        mut self,
        sprite_row: impl Iterator<Item = FifoEntry>,
    ) -> PixelFifo {
        for (i, entry) in sprite_row.enumerate() {
            if self.fifo[i].pixel_index() == 0 {
                // Sprite always wins on top of translucent bg.
                self.fifo[i] = entry;
            } else if !entry.is_s1 {
                // S0 sprites always win on top of bg and S1 sprites.
                self.fifo[i] = entry;
            }
        }
        self.is_suspended = false;
        self
    }

    pub fn pushed(mut self, row: impl Iterator<Item = FifoEntry>) -> PixelFifo {
        //self.fifo.extend(row);
        for entry in row {
            self.fifo[self.cursor as usize] = entry;
            self.cursor += 1;
        }
        self
    }

    pub fn popped(&self) -> PixelFifo {
        let mut new_me = PixelFifo::new();
        for i in 1..self.cursor as usize {
            new_me.fifo[i - 1] = self.fifo[i];
        }
        new_me.cursor = self.cursor - 1;
        new_me
    }
}
