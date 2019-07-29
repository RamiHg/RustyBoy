use super::registers::*;
use super::InternalState;

const NUM_LINE_TCYCLES: i32 = 456;

impl InternalState {
    pub fn tick(&mut self) {
        self.counter += 1;

        if self.counter == NUM_LINE_TCYCLES {
            self.counter = 0;
            self.current_y += 1;
            if self.current_y == 154 {
                self.current_y = 0;
                self.is_first_frame = false;
            }
        }
    }

    /// Loosely based on Metroboy's mode change logic.
    pub fn update_mode(&mut self) {
        if self.counter == 0 {
            debug_assert_ne!(
                self.mode,
                LcdMode::TransferringToLcd,
                "A line took too long to render. Possible hang in GPU."
            );
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
}
