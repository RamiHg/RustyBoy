use super::sprites::SpriteEntry;
use super::{FifoEntry, Gpu};

#[derive(Clone, Copy, Debug, PartialEq)]
enum FetcherMode {
    ReadTileIndex,
    ReadData0,
    ReadData1,
    Ready,
    Idle,
}

#[derive(Clone, Copy, Debug)]
pub struct PixelFetcher {
    mode: FetcherMode,
    address: i32,
    counter: i32,
    sprite: Option<SpriteEntry>,

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
            sprite: None,
            tile_index: 0,
            data0: 0,
            data1: 0,
        }
    }

    pub fn is_fetching_sprite(&self) -> bool { self.sprite.is_some() }

    pub fn execute_tcycle_mut(mut self, gpu: &Gpu) -> PixelFetcher {
        if (self.counter % 2) == 0 {
            match self.mode {
                FetcherMode::ReadTileIndex => {
                    // TODO: Actually, let's support sprites here too! Right now we're skipping the
                    // first 2 cycles.
                    if self.sprite.is_none() {
                        let tile_unsigned_index = gpu.vram(self.address);
                        self.tile_index = tile_unsigned_index as u8;
                    }
                    self.mode = FetcherMode::ReadData0;
                }
                FetcherMode::ReadData0 => {
                    self.data0 = self.read_tile_data(gpu, 0) as u8;
                    self.mode = FetcherMode::ReadData1;
                }
                FetcherMode::ReadData1 => {
                    self.data1 = self.read_tile_data(gpu, 1) as u8;
                    self.mode = FetcherMode::Ready;
                }
                FetcherMode::Ready => {
                    debug_assert!(self.counter <= 16);
                }
                FetcherMode::Idle => {
                    debug_assert!(self.counter < 16);
                }
            }
        }
        self.counter += 1;
        self
    }

    pub fn start(&mut self, address: i32) {
        debug_assert_eq!(self.mode, FetcherMode::Idle);
        self.mode = FetcherMode::ReadTileIndex;
        self.address = address;
        self.counter = 0;
        self.tile_index = 0;
        self.data0 = 0;
        self.data1 = 0;
    }

    pub fn start_sprite(&mut self, sprite: SpriteEntry) {
        debug_assert_eq!(self.mode, FetcherMode::Idle);
        self.start(0);
        self.sprite = Some(sprite);
    }

    pub fn reset(&mut self) { *self = PixelFetcher::new(); }

    pub fn is_idle(&self) -> bool { self.mode == FetcherMode::Idle }
    pub fn is_ready(&self) -> bool { self.mode == FetcherMode::Ready }

    pub fn get_row(&self) -> Vec<FifoEntry> {
        debug_assert_eq!(self.mode, FetcherMode::Ready);
        let mut result = Vec::new();
        let is_sprite = self.sprite.is_some();
        let is_s1 = self.sprite.is_some() && self.sprite.unwrap().priority() == 1;
        // We want to start with the left-most pixel first.
        for i in (0..8).rev() {
            // Blend in the low and high bits from the two bytes representing the row.
            let pixel_data = ((self.data0 >> i) & 0x01) | (((self.data1 >> i) & 0x01) << 1);
            result.push(FifoEntry::from_data(pixel_data, is_sprite, is_s1));
        }
        result
    }

    pub fn peek(&self) -> FifoEntry {
        debug_assert_eq!(self.mode, FetcherMode::Ready);
        self.get_row()[0] // Bad performance but who's watching.
    }

    fn read_tile_data(&self, gpu: &Gpu, byte: i32) -> i32 {
        let address = self.tileset_address(gpu);
        // If address is -1, it means we are rows 8-16 of a sprite in 8x8 mode.
        if address == -1 {
            0
        } else {
            gpu.vram(address + byte)
        }
    }

    fn tileset_address(&self, gpu: &Gpu) -> i32 {
        if self.sprite.is_some() {
            self.sprite_tileset_address(gpu)
        } else {
            self.bg_tileset_address(gpu)
        }
    }

    fn bg_tileset_address(&self, gpu: &Gpu) -> i32 {
        let y_within_tile = gpu.current_y % 8;

        gpu.lcd_control.translate_bg_set_index(self.tile_index) + y_within_tile * 2
    }

    fn sprite_tileset_address(&self, gpu: &Gpu) -> i32 {
        let y_within_tile = gpu.current_y - self.sprite.unwrap().top();
        debug_assert_lt!(y_within_tile, 16);
        debug_assert_ge!(y_within_tile, 0);
        if y_within_tile >= 8 {
            return -1;
        }
        0x8000 + self.sprite.unwrap().tile_index() as i32 * 16 + y_within_tile * 2
    }
}
