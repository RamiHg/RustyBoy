use gpu::Gpu;
use cpu::Cpu;
use memory::Memory;
use cart::*;

use std::rc::Rc;

pub struct System {
    pub gpu: Gpu,
    cpu: Cpu,
}

impl System {

    // Due to a stupid decision to make Cpu own Memory early on in the project
    // Now I have to use cpu.memory everywhere. Someday I will refactor this
    pub fn new() -> System {
        System {
            gpu: Gpu::new(),
            cpu: Cpu::new(),
        }
    }

    pub fn start_system(&mut self, cart_location: &str) {
        self.cpu.memory.cart.read_file(cart_location); // terrible. just terrible
        self.cpu.memory.set_starting_sequence();
        self.cpu.pc = 0x100;
    }

    pub fn update_frame(&mut self) {

    }

    pub fn execute_instruction(&mut self) {
        if !self.cpu.is_stopped {
            let opcode = self.cpu.memory.read_general_8(self.cpu.pc as usize);
            let num_cycles = self.cpu.execute_instruction(opcode);
            self.gpu.update(num_cycles as u32, &mut self.cpu.memory);
        }
        
        self.cpu.handle_interrupts();

        //println!("Ran in {} cycles", num_cycles);
    }
}