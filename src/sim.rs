use soc::gpu::Color;
use soc::joypad::Key;
use soc::system::System;

pub struct Simulator {
    system: System,

    time_accum: f32,
}

impl Simulator {
    pub fn with_system(system: System) -> Simulator {
        Simulator { system, time_accum: 0. }
    }

    /// Updates the internal simulator state by dt seconds. The state is updated in chunks of
    /// simulated "frames", i.e. one simulated 16.66ms block. Will therefore produce 0 or more of
    /// those chunks. If at least one frame was simulated, will return the latest system screen.
    pub fn update(&mut self, dt: f32) -> Option<Vec<Color>> {
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
            Some(Vec::from(self.system.screen()))
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
