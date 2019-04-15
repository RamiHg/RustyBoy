mod registers;

use crate::io_registers;
use crate::mmu;
use crate::system::FireInterrupt;

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

    current_y: i32,
    current_x: i32,

    pixels_pushed: i32,

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
            current_y: 0,

            current_x: 0,
            pixels_pushed: 0,
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

    pub fn execute_t_cycle(&mut self) -> Option<FireInterrupt> {
        // dbg!(self.current_x);
        // dbg!(self.current_y);
        // dbg!(self.pixels_pushed);
        let mut fire_interrupt = None;
        let mut next_mode = self.lcd_status.mode();
        match self.lcd_status.mode() {
            LcdMode::ReadingOAM => {
                assert!(self.cycle <= 20 * 24);
                if self.cycle == 20 * 4 {
                    // Switch to LCD transfer.
                    next_mode = LcdMode::TransferringToLcd;
                    fire_interrupt = Some(FireInterrupt::lcdc());
                }
            }
            LcdMode::TransferringToLcd => {
                self.lcd_transfer_cycle();

                if self.pixels_pushed == LCD_WIDTH {
                    // TODO: Must be careful about next state selection in hardware.
                    self.pixels_pushed = 0;
                    self.current_y += 1;
                    self.current_x = 0;
                    self.cycle = 0;
                    next_mode = LcdMode::ReadingOAM;
                    if self.current_y == LCD_HEIGHT {
                        self.current_y = 0;
                        next_mode = LcdMode::HBlank;
                        fire_interrupt = Some(FireInterrupt::lcdc());
                    }
                }
            }
            LcdMode::HBlank => {
                if self.cycle == 51 * 4 {
                    self.cycle = 0;
                    next_mode = LcdMode::VBlank;
                    fire_interrupt = Some(FireInterrupt::lcdc());
                }
            }
            LcdMode::VBlank => {
                if self.cycle == 114 * 10 * 4 {
                    self.cycle = 0;
                    next_mode = LcdMode::ReadingOAM;
                    assert_eq!(self.current_x, 0);
                    assert_eq!(self.current_y, 0);
                }
            }
        }

        self.cycle += 1;
        self.lcd_status.set_mode(next_mode as u8);
        fire_interrupt
    }

    fn lcd_transfer_cycle(&mut self) {
        // dbg!(self.fifo.fifo.len());
        let pop_fifo = self.fifo.enough_pixels();
        // Check if the fifo has at least 8 pixels. If so, push a pixel onto the screen.
        if self.fifo.enough_pixels() {
            assert!(self.pixels_pushed < LCD_WIDTH);
            let entry = self.fifo.peek();
            let pixel = self.fifo_entry_to_pixel(entry);

            self.screen[(self.pixels_pushed + self.current_y * LCD_WIDTH) as usize] = pixel;
            self.pixels_pushed += 1;
        }

        // Can we push a new tile into the fifo?
        if self.fifo.can_push_tile() {
            if self.fetcher.is_done() {
                self.current_x += 8;
                self.fifo.push(&self.fetcher.get_tile());
                self.fetcher.invalidate();
            }
        }

        // Move forward.
        if self.fetcher.is_invalidated() {
            self.fetcher.start(self.compute_bg_tile_address());
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
        assert_eq!(self.current_x % 8, 0);
        let base_bg_map_address = self.lcd_control.bg_map_address();
        let tile_index = self.current_x / 8 + self.current_y / 8 * 32;
        base_bg_map_address + tile_index
    }
}

impl mmu::MemoryMapped for Gpu {
    fn read(&self, address: mmu::Address) -> Option<i32> {
        assert!(self.current_y < LCD_HEIGHT);
        let mmu::Address(location, raw) = address;
        use crate::io_registers::Addresses;
        match location {
            mmu::Location::Registers => match Addresses::from_i32(raw) {
                Some(Addresses::LcdControl) => Some(self.lcd_control.0 as i32),
                Some(Addresses::LcdStatus) => Some(self.lcd_status.0 as i32),
                Some(Addresses::LcdY) => Some(self.current_y),
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
                Some(Addresses::LcdY) => Some(()),
                Some(Addresses::BgPallette) => {
                    self.bg_palette = value;
                    Some(())
                }
                _ => None,
            },
            mmu::Location::VRam => {
                assert_ne!(self.lcd_status.mode(), LcdMode::TransferringToLcd);
                self.set_vram(raw, value);
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
    Invalid,
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
            mode: FetcherMode::Invalid,
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
                    self.data0 = gpu.vram(self.tileset_address(gpu.lcd_control)) as u8;
                    self.mode = FetcherMode::ReadData1;
                }
                FetcherMode::ReadData1 => {
                    self.data1 = gpu.vram(self.tileset_address(gpu.lcd_control) + 1) as u8;
                    self.mode = FetcherMode::Ready;
                }
                FetcherMode::Ready => {
                    assert!(self.counter < 10);
                }
                FetcherMode::Invalid => {
                    panic!("Invalid fetcher mode!.");
                }
            }
        }
        self.counter += 1;
        self
    }

    pub fn start(&mut self, address: i32) {
        assert!(self.mode == FetcherMode::Invalid || self.mode == FetcherMode::Ready);
        self.mode = FetcherMode::ReadTileIndex;
        self.address = address;
        self.counter = 0;
        self.tile_index = 0;
        self.data0 = 0;
        self.data1 = 0;
    }

    pub fn invalidate(&mut self) { *self = PixelFetcher::new(); }

    pub fn is_invalidated(&self) -> bool { self.mode == FetcherMode::Invalid }
    pub fn is_done(&self) -> bool { self.mode == FetcherMode::Ready && self.counter >= 8 }

    pub fn get_tile(&mut self) -> Vec<FifoEntry> {
        assert_eq!(self.mode, FetcherMode::Ready);
        //assert_eq!(self.counter, 8);
        let mut result = Vec::new();
        for i in 0..8 {
            result.push(FifoEntry::new(
                ((self.data0 >> i) & 0x01) | (((self.data1 >> i) & 0x01) << 1),
            ));
        }
        self.mode = FetcherMode::Invalid;
        result
    }

    fn tileset_address(&self, lcd_control: LcdControl) -> i32 {
        lcd_control.bg_set_address() + self.tile_index as i32 * 16
    }
}
