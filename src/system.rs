use crate::error::{self, Result};
use crate::gpu;
use crate::io_registers;
use crate::serial;
use crate::timer;
use crate::{cpu, mmu, util};

use gpu::Pixel;

use bitflags::bitflags;

bitflags! {
    pub struct Interrupts: i32 {
        const VBLANK = 0b0001;
        const STAT   = 0b0010;
        const TIMER  = 0b0100;
        const SERIAL = 0b1000;
    }
}

pub struct System {
    cpu: cpu::Cpu,
    gpu: gpu::Gpu,

    memory: mmu::Memory,
    timer: timer::Timer,
    serial: serial::Controller,
    cart: Box<mmu::MemoryMapped>,

    screen: Vec<Pixel>,
}

impl System {
    pub fn new_with_cart(cart: Box<mmu::MemoryMapped>) -> System {
        System {
            cpu: cpu::Cpu::new(),
            gpu: gpu::Gpu::new(),
            memory: mmu::Memory::new(),
            timer: timer::Timer::new(),
            serial: serial::Controller::new(),
            cart,

            screen: vec![Pixel::zero(); (gpu::LCD_WIDTH * gpu::LCD_HEIGHT) as usize],
        }
    }

    pub fn gpu(&self) -> &gpu::Gpu { &self.gpu }
    pub fn get_screen(&self) -> &[Pixel] { &self.screen }

    fn read_request(&self, raw_address: i32) -> Result<i32> {
        let modules = [
            &self.timer,
            self.cart.as_ref(),
            &self.serial,
            &self.gpu,
            &self.memory,
        ];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in &modules {
            if let Some(result) = module.read(address) {
                return Ok(result);
            }
        }
        return Err(error::Type::InvalidOperation(format!(
            "Could not find any memory module accepting {:X?}",
            raw_address
        )));
    }

    fn write_request(&mut self, raw_address: i32, value: i32) -> Result<()> {
        let mut modules = [
            &mut self.timer,
            self.cart.as_mut(),
            &mut self.serial,
            &mut self.gpu,
            &mut self.memory,
        ];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in &mut modules {
            if let Some(()) = module.write(address, value) {
                return Ok(());
            }
        }
        return Err(error::Type::InvalidOperation(format!(
            "Could not find any memory module accepting {:X?}",
            raw_address
        )));
    }

    fn handle_cpu_memory_reads(&mut self) -> Result<()> {
        debug_assert!(!(self.cpu.state.read_latch && self.cpu.state.write_latch));
        let t_state = self.cpu.t_state.get();
        if self.cpu.state.read_latch {
            if t_state == 3 {
                self.cpu.state.data_latch = self.read_request(self.cpu.state.address_latch)?;
            // println!(
            //     "Reading {:X?} from {:X?}",
            //     self.cpu.state.data_latch, self.cpu.state.address_latch
            // );
            } else {
                // Write garbage in data latch to catch bad reads.
                self.cpu.state.data_latch = -1;
            }
        }
        Ok(())
    }

    fn handle_cpu_memory_writes(&mut self) -> Result<()> {
        // Service write requests at T=4's rising edge.
        if self.cpu.state.write_latch {
            // println!(
            //     "Writing {:X?} to {:X}",
            //     self.cpu.state.data_latch, self.cpu.state.address_latch
            // );
            debug_assert!(util::is_16bit(self.cpu.state.address_latch));
            debug_assert!(util::is_8bit(self.cpu.state.data_latch));
            if self.cpu.t_state.get() == 4 {
                self.write_request(self.cpu.state.address_latch, self.cpu.state.data_latch)?;
            }
        }
        Ok(())
    }

    fn maybe_fire_interrupt(&mut self, maybe_fire: Interrupts) {
        let mut current_if = self
            .memory
            .read(io_registers::Addresses::InterruptFired as i32);
        current_if |= maybe_fire.bits();
        self.memory
            .store(io_registers::Addresses::InterruptFired as i32, current_if);
    }

    fn handle_timer(&mut self) -> Result<timer::Timer> {
        // Execute the timer at the rise of TCycle 4, such that it can control writes coming in from
        // the CPU.
        if self.cpu.t_state.get() != 100 {
            let (new_timer, should_interrupt) = self.timer.execute_mcycle();
            self.maybe_fire_interrupt(should_interrupt);
            Ok(new_timer)
        } else {
            Ok(self.timer)
        }
    }

    fn handle_serial(&mut self) -> serial::Controller {
        let (new_serial, should_interrupt) = self.serial.execute_tcycle();
        self.maybe_fire_interrupt(should_interrupt);
        new_serial
    }

    fn handle_gpu(&mut self) -> gpu::Gpu {
        let (next_gpu, should_interrupt) = self.gpu.execute_t_cycle(&mut self.screen);
        self.maybe_fire_interrupt(should_interrupt);
        next_gpu
    }

    fn execute_t_cycle(&mut self) -> Result<()> {
        if self.cpu.t_state.get() == 3
            && self.cpu.state.decode_mode == cpu::DecodeMode::Decode
            && !self.cpu.is_handling_interrupt
        {
            let pc_plus =
                |x| self.read_request(self.cpu.registers.get(cpu::register::Register::PC) + x);
            let disas =
                gb_disas::decode::decode(pc_plus(0)? as u8, pc_plus(1)? as u8, pc_plus(2)? as u8);
            if let core::result::Result::Ok(op) = disas {
                println!(
                    "{:04X?}\t{}",
                    self.cpu.registers.get(cpu::register::Register::PC),
                    op
                );
            } else {
                println!(
                    "{:04X?}\tBad opcode {:X?}",
                    self.cpu.registers.get(cpu::register::Register::PC),
                    pc_plus(0)?
                );
            }
        }

        // Do all the rising edge sampling operations.
        self.handle_cpu_memory_reads()?;
        let new_timer = self.handle_timer()?;
        let new_serial = self.handle_serial();
        let new_gpu = self.handle_gpu();
        self.cpu.execute_t_cycle(&mut self.memory)?;
        // Finally, do all the next state replacement.
        self.timer = new_timer;
        self.serial = new_serial;
        self.gpu = new_gpu;
        self.handle_cpu_memory_writes()?;

        Ok(())
    }

    pub fn execute_machine_cycle(&mut self) -> Result<()> {
        for i in 0..4 {
            self.execute_t_cycle()?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn is_fetching(&self) -> bool { self.cpu.state.decode_mode == cpu::DecodeMode::Fetch }
}

#[cfg(test)]
impl System {
    pub fn new_test_system(cart: Box<dyn mmu::MemoryMapped>) -> System {
        System::new_with_cart(cart)
    }

    // pub fn memory_mut(&mut self) -> &mut mmu::Memory { &mut self.memory }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu { &mut self.cpu }

    pub fn memory_write(&mut self, raw_address: i32, value: i32) {
        self.write_request(raw_address, value).unwrap();
    }

    pub fn memory_read(&self, raw_address: i32) -> i32 { self.read_request(raw_address).unwrap() }
    pub fn memory_read_16(&self, raw_address: i32) -> i32 {
        self.memory_read(raw_address) | (self.memory_read(raw_address + 1) << 8)
    }
}
