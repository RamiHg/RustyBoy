use crate::cpu;
use crate::error;
use crate::gpu;
use crate::io_registers;
use crate::mmu;
use crate::{dma, serial, timer, util};

use error::Result;
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
    timer: Box<timer::Timer>,
    serial: serial::Controller,
    dma: dma::Dma,
    cart: Box<mmu::MemoryMapped>,

    screen: Vec<Pixel>,

    test: i32,
}

impl System {
    pub fn new_with_cart(cart: Box<mmu::MemoryMapped>) -> System {
        use cpu::register::Register;
        let mut cpu = cpu::Cpu::new();
        // Set the initial register values.
        cpu.registers.set(Register::A, 0x01);
        cpu.registers.set(Register::F, 0xB0);
        cpu.registers.set(Register::C, 0x13);
        cpu.registers.set(Register::E, 0xD8);
        cpu.registers.set(Register::H, 0x01);
        cpu.registers.set(Register::L, 0x4D);
        cpu.registers.set(Register::SP, 0xFFFE);

        System {
            cpu: cpu,
            gpu: gpu::Gpu::new(),
            memory: mmu::Memory::new(),
            timer: Box::new(timer::Timer::new()),
            serial: serial::Controller::new(),
            dma: dma::Dma::new(),
            cart,
            test: 0,

            screen: vec![Pixel::zero(); (gpu::LCD_WIDTH * gpu::LCD_HEIGHT) as usize],
        }
    }

    pub fn gpu(&self) -> &gpu::Gpu { &self.gpu }
    pub fn get_screen(&self) -> &[Pixel] { &self.screen }

    fn read_request(&self, raw_address: i32) -> Result<i32> {
        let modules = [
            self.timer.as_ref(),
            self.cart.as_ref(),
            &self.serial,
            &self.gpu,
            &self.dma,
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
            self.timer.as_mut(),
            self.cart.as_mut(),
            &mut self.serial,
            &mut self.gpu,
            &mut self.dma,
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
            if t_state == 2 {
                self.cpu.state.data_latch = self.read_request(self.cpu.state.address_latch)?;
            // println!(
            //     "Reading {:X?} from {:X?}",
            //     self.cpu.state.data_latch, self.cpu.state.address_latch
            // );
            } else if false {
                // Write garbage in data latch to catch bad reads.
                self.cpu.state.data_latch = -1;
            }
        }
        Ok(())
    }

    fn handle_cpu_memory_writes(&mut self, sh: bool) -> Result<()> {
        // Service write requests at T=4's rising edge.
        if self.cpu.state.write_latch {
            // println!(
            //     "Writing {:X?} to {:X}",
            //     self.cpu.state.data_latch, self.cpu.state.address_latch
            // );
            debug_assert!(util::is_16bit(self.cpu.state.address_latch));
            debug_assert!(util::is_8bit(self.cpu.state.data_latch));
            // if self.cpu.t_state.get() == 1 && sh {
            if sh {
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

    fn handle_dma_mcycle(&mut self) -> Result<()> {
        let (dma, request) = self.dma.execute_mcycle();
        self.dma = dma;
        if let Some(request) = request {
            let value = self.read_request(request.source_address)?;
            // Since we know the destination has to be OAM, skip the mmu routing.
            mmu::MemoryMapped::write(
                &mut self.gpu,
                mmu::Address::from_raw(request.destination_address)?,
                value,
            )
            .ok_or(error::Type::InvalidOperation(
                "DMA destination was not OAM".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn handle_timer(&mut self) -> Result<Box<timer::Timer>> {
        use mmu::MemoryMapped2;
        let bus = mmu::MemoryBus {
            address_latch: self.cpu.state.address_latch,
            data_latch: self.cpu.state.data_latch,
            read_latch: false,
            write_latch: self.cpu.state.write_latch,
            t_state: self.cpu.t_state.get(),
        };
        let (new_timer, should_interrupt) = self.timer.clone().execute_tcycle(&bus);
        self.maybe_fire_interrupt(should_interrupt);
        Ok(new_timer)
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
        if self.cpu.t_state.get() == 1
            && self.cpu.state.decode_mode == cpu::DecodeMode::Fetch
            && !self.cpu.is_handling_interrupt
        {
            let pc_plus =
                |x| self.read_request(self.cpu.registers.get(cpu::register::Register::PC) + x);
            let disas =
                gb_disas::decode::decode(pc_plus(0)? as u8, pc_plus(1)? as u8, pc_plus(2)? as u8);
            if let core::result::Result::Ok(op) = disas {
                trace!(
                    target: "disas",
                    "{:04X?}\t{}",
                    self.cpu.registers.get(cpu::register::Register::PC),
                    op
                );
            } else {
                trace!(
                    target: "disas",
                    "{:04X?}\tBad opcode {:X?}",
                    self.cpu.registers.get(cpu::register::Register::PC),
                    pc_plus(0)?
                );
            }
        }
        // Do all the rising edge sampling operations.
        let should_write = self.cpu.t_state.get() == 4;

        self.cpu.execute_t_cycle(&mut self.memory)?;
        self.handle_cpu_memory_reads()?;
        let new_timer = self.handle_timer()?;
        let new_serial = self.handle_serial();
        let new_gpu = self.handle_gpu();
        // Finally, do all the next state replacement.
        self.timer = new_timer;
        self.serial = new_serial;
        self.gpu = new_gpu;
        self.handle_cpu_memory_writes(should_write)?;
        self.cpu.t_state.inc();

        Ok(())
    }

    pub fn execute_machine_cycle(&mut self) -> Result<()> {
        self.handle_dma_mcycle()?;

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
        let mut system = System::new_with_cart(cart);
        // Clear all registers.
        system.cpu.registers = cpu::register::File::new();
        // Start with the GPU disabled.
        system
            .write_request(io_registers::Addresses::LcdControl as i32, 0)
            .unwrap();
        system
    }

    // pub fn memory_mut(&mut self) -> &mut mmu::Memory { &mut self.memory }
    pub fn memory(&self) -> &mmu::Memory { &self.memory }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu { &mut self.cpu }

    pub fn memory_write(&mut self, raw_address: i32, value: i32) {
        self.write_request(raw_address, value).unwrap();
    }

    pub fn memory_read(&self, raw_address: i32) -> i32 { self.read_request(raw_address).unwrap() }
    pub fn memory_read_16(&self, raw_address: i32) -> i32 {
        self.memory_read(raw_address) | (self.memory_read(raw_address + 1) << 8)
    }
}
