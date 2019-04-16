mod registers;

use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;

use num_traits::FromPrimitive;
use registers::*;

const LCD_WIDTH: i32 = 160;
const LCD_HEIGHT: i32 = 144;

#[derive(Clone, Copy)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    fn zero() -> Pixel { Pixel { r: 0, g: 0, b: 0 } }

    fn from_values(r: u8, g: u8, b: u8) -> Pixel { Pixel { r, g, b } }
}

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

    screen: [Pixel; LCD_WIDTH as usize * LCD_HEIGHT as usize],

    fifo: PixelFifo,
    fetcher: PixelFetcher,

    // VRAM.
    vram: [u8; 8192],
}

impl Gpu {
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
            screen: [Pixel::zero(); (LCD_WIDTH * LCD_HEIGHT) as usize],

            fifo: PixelFifo::new(),
            fetcher: PixelFetcher::new(),
            vram: [0; 8192],
        }
    }

    pub fn get_pixel(&self, i: i32, j: i32) -> Pixel { self.screen[(i + j * LCD_WIDTH) as usize] }

    pub fn vram(&self, address: i32) -> i32 { self.vram[(address - 0x8000) as usize] as i32 }
    pub fn set_vram(&mut self, address: i32, value: i32) {
        self.vram[(address - 0x8000) as usize] = value as u8;
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

    pub fn execute_t_cycle(&mut self) -> Interrupts {
        // dbg!(self.current_x);
        // dbg!(self.current_y);
        // dbg!(self.pixels_pushed);
        let mut fire_interrupt = Interrupts::empty();
        let mut next_mode = self.lcd_status.mode();

        if !self.lcd_control.enable_display() {
            self.pixels_pushed = 0;
            self.current_x = 0;
            self.current_y = 0;
            self.pixels_scrolled = 0;
            self.cycle = 0;
            self.fetcher = PixelFetcher::new();
            self.fifo = PixelFifo::new();
            return Interrupts::empty();
        }

        self.lcd_status.set_ly_is_lyc(self.current_y == self.lyc);
        // Remaining minor points: LY=LYC needs to be forced to 0 in certain situations.
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM => {
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
                    self.cycle = 0;
                    next_mode = LcdMode::TransferringToLcd;
                }
            }
            LcdMode::TransferringToLcd => {
                self.lcd_transfer_cycle();
                // dbg!(self.pixels_pushed);
                // dbg!(self.pixels_scrolled);
                // dbg!(self.current_x);
                assert!(self.cycle < (43 + 16) * 4);
                if self.pixels_pushed == LCD_WIDTH {
                    // TODO: Must be careful about next state selection in hardware.
                    self.pixels_pushed = 0;
                    self.pixels_scrolled = 0;
                    self.current_x = 0;
                    self.cycle = 0;
                    self.fetcher.reset();
                    next_mode = LcdMode::HBlank;
                    fire_interrupt = self.maybe_fire_interrupt(InterruptType::HBlank);
                }
            }
            LcdMode::HBlank => {
                if self.cycle == 51 * 4 {
                    self.cycle = 0;
                    self.current_y += 1;
                    if self.current_y == LCD_HEIGHT {
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
                    self.current_y = 0;
                }
                if self.cycle == (114 - 1) * 4 {
                    self.cycle = 0;
                    self.current_y += 1;
                    if self.current_y == 1 {
                        // Switch over to Oam of line 1! Weird timing, I know!
                        self.cycle = 0;
                        self.current_y = 0;
                        assert_eq!(self.current_x, 0);
                        assert_eq!(self.pixels_pushed, 0);
                        next_mode = LcdMode::ReadingOAM;
                    }
                }
            }
        }
        if next_mode == self.lcd_status.mode() {
            self.cycle += 1;
        } else {
            // println!("Going to {:?}", next_mode);
        }
        self.lcd_status.set_mode(next_mode as u8);
        fire_interrupt
    }

    fn lcd_transfer_cycle(&mut self) {
        let pop_fifo = self.fifo.enough_pixels();
        // Check if the fifo has at least 8 pixels. If so, push a pixel onto the screen.
        if self.fifo.enough_pixels() {
            assert!(self.pixels_pushed < LCD_WIDTH);
            let entry = self.fifo.peek();
            let pixel = self.fifo_entry_to_pixel(entry);
            if self.pixels_scrolled == 0 {
                self.screen[(self.pixels_pushed + self.current_y * LCD_WIDTH) as usize] = pixel;
                self.pixels_pushed += 1;
            } else {
                self.pixels_scrolled += 1;
            }
        }

        // Can we push a new tile into the fifo?
        if self.fifo.can_push_tile() {
            if self.fetcher.is_done() {
                self.current_x += 8;
                self.fifo.push(&self.fetcher.get_tile());
            }
        }

        // Move forward.
        if self.fetcher.is_idle() {
            self.fetcher.start(self.compute_bg_tile_address());
            self.pixels_scrolled = -(self.scroll_x % 8);
        }

        self.fetcher = self.fetcher.execute_tcycle_mut(&self);
        if pop_fifo {
            self.fifo.pop();
        }
    }

    fn fifo_entry_to_pixel(&self, entry: FifoEntry) -> Pixel {
        match (self.bg_palette >> entry.pixel_index) & 0x3 {
            0 => Pixel::from_values(255u8, 255u8, 255u8),
            1 => Pixel::from_values(192u8, 192u8, 192u8),
            2 => Pixel::from_values(96u8, 96u8, 96u8),
            3 | _ => Pixel::from_values(0u8, 0u8, 0u8),
        }
    }

    /// There are 20x18 tiles. Each tile is 16 bytes.
    /// The background map is 32x32 tiles (32 * 32 = 1KB)
    fn compute_bg_tile_address(&self) -> i32 {
        let base_bg_map_address = self.lcd_control.bg_map_address();
        let x = ((self.current_x + self.scroll_x) / 8) % 32;
        let tile_index = x + self.current_y / 8 * 32;
        base_bg_map_address + tile_index
    }
}

impl mmu::MemoryMapped for Gpu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::LcdControl) => Some(self.lcd_control.0 as i32),
                Some(Addresses::LcdStatus) => Some(self.lcd_status.0 as i32),
                Some(Addresses::ScrollX) => Some(self.scroll_x),
                Some(Addresses::ScrollY) => Some(self.scroll_y),
                Some(Addresses::LcdY) => Some(self.current_y),
                Some(Addresses::LcdYCompare) => Some(self.lyc),
                Some(Addresses::BgPallette) => Some(self.bg_palette),
                _ => None,
            },
            mmu::Location::VRam => Some(self.vram(raw)),
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
                    self.lcd_status.0 = value as u8;
                    Some(())
                }
                Some(Addresses::ScrollX) => {
                    dbg!(value);
                    self.scroll_x = value;
                    Some(())
                }
                Some(Addresses::ScrollY) => {
                    dbg!(value);
                    self.scroll_y = value;
                    Some(())
                }
                Some(Addresses::LcdY) => Some(()),
                Some(Addresses::LcdYCompare) => {
                    dbg!(value);
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
                if self.lcd_status.mode() != LcdMode::TransferringToLcd {
                    self.set_vram(raw, value);
                }
                Some(())
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FifoEntry {
    pub pixel_index: u8,
}

impl FifoEntry {
    pub fn new(data: u8) -> FifoEntry {
        assert!(data < 4);
        FifoEntry { pixel_index: data }
    }
}

#[derive(Debug)]
struct PixelFifo {
    pub fifo: Vec<FifoEntry>,
}

impl PixelFifo {
    pub fn new() -> PixelFifo { PixelFifo { fifo: Vec::new() } }

    pub fn peek(&self) -> FifoEntry { self.fifo[0] }

    pub fn enough_pixels(&self) -> bool { self.fifo.len() > 8 }
    pub fn can_push_tile(&self) -> bool { self.fifo.len() <= 8 }

    pub fn push(&mut self, tile: &[FifoEntry]) {
        for i in 0..8 {
            self.fifo.push(tile[i]);
        }
    }

    pub fn pop(&mut self) { self.fifo.remove(0); }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FetcherMode {
    ReadTileIndex,
    ReadData0,
    ReadData1,
    Ready,
    Idle,
}

#[derive(Clone, Copy, Debug)]
struct PixelFetcher {
    mode: FetcherMode,
    address: i32,
    counter: i32,

    tile_index: u8,
    data0: u8,
    data1: u8,
}

impl PixelFetcher {
    pub fn new() -> PixelFetcher {
        PixelFetcher {
            mode: FetcherMode::Idle,
            address: -1,
            counter: -1,
            tile_index: 0,
            data0: 0,
            data1: 0,
        }
    }

    pub fn execute_tcycle_mut(mut self, gpu: &Gpu) -> PixelFetcher {
        if (self.counter % 2) == 0 {
            match self.mode {
                FetcherMode::ReadTileIndex => {
                    let tile_unsigned_index = gpu.vram(self.address);
                    self.tile_index = if !gpu.lcd_control.bg_set_select() {
                        (i32::from(tile_unsigned_index) + 128) as u8
                    } else {
                        tile_unsigned_index as u8
                    };
                    self.mode = FetcherMode::ReadData0;
                }
                FetcherMode::ReadData0 => {
                    self.data0 = gpu.vram(self.tileset_address(gpu)) as u8;
                    self.mode = FetcherMode::ReadData1;
                }
                FetcherMode::ReadData1 => {
                    self.data1 = gpu.vram(self.tileset_address(gpu) + 1) as u8;
                    self.mode = FetcherMode::Ready;
                }
                FetcherMode::Ready => {
                    assert!(self.counter <= 16);
                }
                FetcherMode::Idle => {
                    assert!(self.counter < 16);
                }
            }
        }
        self.counter += 1;
        self
    }

    pub fn start(&mut self, address: i32) {
        assert_eq!(self.mode, FetcherMode::Idle);
        self.mode = FetcherMode::ReadTileIndex;
        self.address = address;
        self.counter = 0;
        self.tile_index = 0;
        self.data0 = 0;
        self.data1 = 0;
    }

    pub fn reset(&mut self) { *self = PixelFetcher::new(); }

    pub fn is_idle(&self) -> bool { self.mode == FetcherMode::Idle }
    pub fn is_done(&self) -> bool { self.mode == FetcherMode::Ready }

    pub fn get_tile(&mut self) -> Vec<FifoEntry> {
        assert_eq!(self.mode, FetcherMode::Ready);
        let mut result = Vec::new();
        // We want to start with the left-most pixel first.
        for i in (0..8).rev() {
            result.push(FifoEntry::new(
                ((self.data0 >> i) & 0x01) | (((self.data1 >> i) & 0x01) << 1),
            ));
        }
        self.reset();
        result
    }

    fn tileset_address(&self, gpu: &Gpu) -> i32 {
        let y_within_tile = gpu.current_y % 8;

        gpu.lcd_control.bg_set_address() + self.tile_index as i32 * 16 + y_within_tile * 2
    }
}
