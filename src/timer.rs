use crate::cpu::SideEffect;
use crate::io_registers;
use crate::memory::Memory;
use crate::util::is_bit_set;

#[derive(Copy, Clone)]
pub struct Timer {
    counter: i32,
    is_tima_overflow: bool,
}

impl Timer {
    pub fn execute_cycle(&mut self, memory: &Memory) -> Option<Vec<SideEffect>> {
        let mut side_effects = Vec::new();
        // Gather the register values.
        let tac = memory.read_register(io_registers::TimerControl);
        if !tac.enabled() {
            return None;
        }
        let mut new_state = *self;
        new_state.counter += 1;
        let old_bit = self.edge_detector_input(memory);
        let new_bit = new_state.edge_detector_input(memory);
        if old_bit && !new_bit {
            // Increment timer register!
            let tima_address = io_registers::Addresses::TimerCounter as i32;
            side_effects.push(SideEffect {
                raw_address: time_address,
                valu,
            })
        }
    }

    /// Tries to emulate the internal behavior of the timer as much possible (mostly to accurately
    /// implement unintended consequences and glitches!).
    fn edge_detector_input(&self, memory: &Memory) -> bool {
        let tac = memory.read(io_registers::Addresses::TimerControl as i32);
        assert!(tac < 8);

        let freq_0 = is_bit_set(tac, 0);
        let freq_1 = is_bit_set(tac, 1);
        let freq_1_a = if freq_0 {
            is_bit_set(self.counter, 7)
        } else {
            is_bit_set(self.counter, 5)
        };
        let freq_1_b = if freq_0 {
            is_bit_set(self.counter, 3)
        } else {
            is_bit_set(self.counter, 9)
        };
        if freq_1 {
            freq_1_a
        } else {
            freq_1_b
        }
    }
}
