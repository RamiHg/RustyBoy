use crate::gpu::Pixel;
use crate::gpu::{LCD_HEIGHT, LCD_WIDTH};
use crate::joypad::Key;
use crate::system::System;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Simulator {
    // #[wasm_bindgen(skip)]
    system: System,

    time_accum: f32,
}

impl Simulator {
    pub fn with_system(system: System) -> Simulator {
        Simulator { system, time_accum: 0.0 }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl Simulator {
    pub fn from_cart_bytes(cart_bytes: &[u8]) -> Simulator {
        console_error_panic_hook::set_once();
        let cart = crate::cart::from_file_contents(cart_bytes);
        let mut system = System::new_complete();
        system.set_cart(cart);
        Simulator { system, time_accum: 0. }
    }

    /// Updates the internal simulator state by dt seconds. The state is updated in chunks of
    /// simulated "frames", i.e. one simulated 16.66ms block. Will therefore produce 0 or more of
    /// those chunks. If at least one frame was simulated, will return the latest system screen.
    pub fn update(&mut self, dt: f32) -> Option<Box<[u8]>> {
        // Accumulate passed time. Make sure not to fall back behind by more than one second.
        self.time_accum += dt.min(1.);
        // Simulate the system in discrete 16.66ms chunks.
        const STEP_SIZE: f32 = 1. / 60.;
        // Either return a screen, or early out (saves having to copy multiple screens if stepping
        // for more than one frame).
        if self.time_accum >= STEP_SIZE {
            while self.time_accum >= STEP_SIZE {
                self.simulate_frame();
                self.time_accum -= STEP_SIZE;
            }
            let mut arr = [0; LCD_WIDTH * LCD_HEIGHT * 4];

            for (i, pixel) in self.system.screen().iter().map(Pixel::from).enumerate() {
                if cfg!(target_arch = "wasm32") {
                    arr[i * 4] = pixel.r;
                    arr[i * 4 + 1] = pixel.g;
                    arr[i * 4 + 2] = pixel.b;
                    arr[i * 4 + 3] = pixel.a;
                } else {
                    arr[i * 4] = pixel.b;
                    arr[i * 4 + 1] = pixel.g;
                    arr[i * 4 + 2] = pixel.r;
                    arr[i * 4 + 3] = pixel.a;
                }
            }
            Some(Box::from(arr))
        } else {
            None
        }
    }

    pub fn press_key(&mut self, key: Key) {
        self.system.joypad_mut().press(key);
    }
    pub fn release_key(&mut self, key: Key) {
        self.system.joypad_mut().release(key);
    }

    fn simulate_frame(&mut self) {
        let mut is_vsyncing = self.system.is_vsyncing();
        // Equivalent to while !(!is_vsyncing && system.is_vsyncing()). Aka edge detection.
        while is_vsyncing || !self.system.is_vsyncing() {
            is_vsyncing = self.system.is_vsyncing();
            self.system.execute_machine_cycle().unwrap();
        }
    }
}
