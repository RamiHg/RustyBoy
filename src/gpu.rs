mod registers;
mod sprites;

use crate::io_registers;
use crate::mmu;
use crate::system::Interrupts;
use registers::*;

use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::rc::Rc;

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

    // VRAM.
    vram: Rc<RefCell<[u8; 8192]>>,
}

impl Gpu {
    pub fn new() -> Gpu {
        let mut lcd_status = LcdStatus(0);
        lcd_status.set_mode(LcdMode::ReadingOAM as u8);

        // let mut vram = BytesMut::with_capacity(8192);
        // vram.extend_from_slice(&[0; 8192]);

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
            vram: Rc::new(RefCell::new([0; 8192])),
        }
    }

    //pub fn get_pixel(&self, i: i32, j: i32) -> Pixel { self.screen[(i + j * LCD_WIDTH) as usize]
    // }

    pub fn vram(&self, address: i32) -> i32 {
        self.vram.borrow()[(address - 0x8000) as usize] as i32
    }
    pub fn set_vram(&mut self, address: i32, value: i32) {
        self.vram.borrow_mut()[(address - 0x8000) as usize] = value as u8;
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

        next_state
            .lcd_status
            .set_ly_is_lyc(self.current_y == self.lyc);
        next_state.cycle += 1;

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
                    next_state.cycle = 0;
                    next_mode = LcdMode::TransferringToLcd;
                    // todo: move me somewhere better
                    next_state.pixels_scrolled = self.scroll_x % 8;
                }
            }
            LcdMode::TransferringToLcd => {
                self.lcd_transfer_cycle(&mut next_state, screen);
                //assert!(self.cycle < (43 + 16) * 4);
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
                if self.cycle == 51 * 4 {
                    next_state.cycle = 0;
                    next_state.current_y += 1;
                    if next_state.current_y == LCD_HEIGHT {
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
                        next_state.scroll_x = (next_state.scroll_x + 1) % 255;
                    }
                }
            }
        }
        if next_mode != self.lcd_status.mode() {
            //println!("Going to {:?}", next_mode);
        }
        next_state.lcd_status.set_mode(next_mode as u8);
        (next_state, fire_interrupt)
    }

    fn lcd_transfer_cycle(&self, next_state: &mut Gpu, screen: &mut [Pixel]) {
        // dbg!(self.fetcher);
        // dbg!(self.pixels_pushed);
        // dbg!(self.fifo.len());
        let fetcher_is_ready = self.fetcher.is_ready();

        let mut next_fifo = self.fifo.clone();
        let mut next_fetcher = self.fetcher.execute_tcycle_mut(&self);
        //let mut borrowed_pixel = false;
        let mut new_len = self.fifo.len();

        let mut pixel = None;
        // Easy case: fifo has more than 8 pixels. We can simply pop a pixel off.
        if self.fifo.len() > 8 {
            let entry = self.fifo.peek();
            pixel = Some(self.fifo_entry_to_pixel(entry));
            new_len -= 1;
        } else if self.fifo.len() > 0 && fetcher_is_ready {
            pixel = Some(self.fifo_entry_to_pixel(self.fifo.peek()));
            //borrowed_pixel = true;
            new_len -= 1;
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
            next_state.current_x += 8;
            next_fetcher.start(self.compute_bg_tile_address(next_state.current_x));
        }
        next_state.fetcher = next_fetcher;
        next_state.fifo = next_fifo;
        // Actually push the pixel on the screen.
        if let Some(pixel) = pixel {
            assert!(self.pixels_pushed < LCD_WIDTH);
            screen[(self.pixels_pushed + self.current_y * LCD_WIDTH) as usize] = pixel;
            next_state.pixels_pushed += 1;
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
    fn compute_bg_tile_address(&self, x: i32) -> i32 {
        let base_bg_map_address = self.lcd_control.bg_map_address();
        let x = ((x + self.scroll_x) / 8) % 32;
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

// A sad attempt to make a copyable fifo.
#[derive(Clone, Copy, Debug)]
struct PixelFifo {
    pub fifo: [FifoEntry; 16],
    cursor: i8,
}

impl PixelFifo {
    pub fn new() -> PixelFifo {
        PixelFifo {
            fifo: [FifoEntry { pixel_index: 0 }; 16],
            cursor: 0,
        }
    }

    pub fn peek(&self) -> FifoEntry { self.fifo[0] }
    pub fn len(&self) -> usize { self.cursor as usize }

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
    pub fn is_ready(&self) -> bool { self.mode == FetcherMode::Ready }

    pub fn get_row(&self) -> Vec<FifoEntry> {
        assert_eq!(self.mode, FetcherMode::Ready);
        let mut result = Vec::new();
        // We want to start with the left-most pixel first.
        for i in (0..8).rev() {
            result.push(FifoEntry::new(
                ((self.data0 >> i) & 0x01) | (((self.data1 >> i) & 0x01) << 1),
            ));
        }
        result
    }

    pub fn peek(&self) -> FifoEntry {
        assert_eq!(self.mode, FetcherMode::Ready);
        self.get_row()[0] // Bad performance but who's watching.
    }

    fn tileset_address(&self, gpu: &Gpu) -> i32 {
        let y_within_tile = gpu.current_y % 8;

        gpu.lcd_control.bg_set_address() + self.tile_index as i32 * 16 + y_within_tile * 2
    }
}
