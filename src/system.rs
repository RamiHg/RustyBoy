use gpu::Gpu;
use cpu::Cpu;
use memory::Memory;
use cart::*;

use std::rc::Rc;

pub struct System<'a> {
    pub gpu: Gpu,
    memory: Memory,
    cpu: Cpu<'a>,
    cart: Cart
}

impl<'a> System<'a> {
    pub fn new<'b>() -> System<'b> {
        let mut memory = Memory::new();

        System {
            memory: memory,
            gpu: Gpu::new(),
            cpu: Cpu::new(memory),
            cart: Cart::new()
        }
    }

    pub fn start_system(&mut self, cart_location: &str) {
        self.cart.read_file(cart_location);
        self.cpu.pc = 0x100;
    }

    pub fn update_frame(&mut self) {

    }

    pub fn execute_instruction(&mut self) {
        let opcode = self.cpu.memory.read_general_8(self.cpu.pc as usize);
        let num_cycles = self.cpu.execute_instruction(opcode);
        self.gpu.update(num_cycles as u32, &mut self.cpu.memory);

        //println!("Ran in {} cycles", num_cycles);
    }
}