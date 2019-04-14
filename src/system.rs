use crate::error::{self, Result};
use crate::io_registers;
use crate::serial;
use crate::timer;
use crate::{cpu, mmu, util};

pub struct FireInterrupt(pub bool);

pub struct System {
    cpu: cpu::Cpu,

    memory: mmu::Memory,
    timer: timer::Timer,
    serial: serial::Controller,
    cart: Box<mmu::MemoryMapped>,
}

impl System {
    pub fn new_with_cart(cart: Box<mmu::MemoryMapped>) -> System {
        System {
            cpu: cpu::Cpu::new(),
            memory: mmu::Memory::new(),
            timer: timer::Timer::new(),
            serial: serial::Controller::new(),
            cart,
        }
    }

    fn read_request(&self, raw_address: i32) -> Result<i32> {
        let modules = [&self.timer, self.cart.as_ref(), &self.serial, &self.memory];
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
        assert!(!(self.cpu.state.read_latch && self.cpu.state.write_latch));
        let t_state = self.cpu.t_state.get();
        if self.cpu.state.read_latch {
            if t_state == 3 {
                self.cpu.state.data_latch = self.read_request(self.cpu.state.address_latch)?;
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
            assert!(util::is_16bit(self.cpu.state.address_latch));
            assert!(util::is_8bit(self.cpu.state.data_latch));
            if self.cpu.t_state.get() == 4 {
                self.write_request(self.cpu.state.address_latch, self.cpu.state.data_latch)?;
            }
        }
        Ok(())
    }

    fn handle_timer(&mut self) -> Result<timer::Timer> {
        // Execute the timer at the rise of TCycle 4, such that it can control writes coming in from
        // the CPU.
        if self.cpu.t_state.get() == 1 {
            let (new_timer, should_interrupt) = self.timer.execute_mcycle();
            if should_interrupt.0 {
                // Immediately set the interrupt fired flag. In real hardware, we would simply
                // assert a flag for the interrupt handler. This is so that the CPU can override it
                // if it wants to.
                let mut interrupt_fired = self.memory.get_mut_register(io_registers::InterruptFlag);
                interrupt_fired.set_timer(true);
            }
            Ok(new_timer)
        } else {
            Ok(self.timer)
        }
    }

    fn handle_serial(&mut self) -> serial::Controller {
        let (new_serial, should_interrupt) = self.serial.execute_tcycle();
        if should_interrupt.0 {
            let mut interrupt_fired = self.memory.get_mut_register(io_registers::InterruptFlag);
            interrupt_fired.set_serial(true);
        }
        new_serial
    }

    fn execute_t_cycle(&mut self) -> Result<()> {
        // Do all the rising edge sampling operations.
        self.handle_cpu_memory_reads()?;
        let new_timer = self.handle_timer()?;
        let new_serial = self.handle_serial();
        self.cpu.execute_t_cycle(&mut self.memory)?;
        // Finally, do all the next state replacement.
        self.timer = new_timer;
        self.serial = new_serial;
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
        System {
            cpu: cpu::Cpu::new(),
            memory: mmu::Memory::new(),
            cart: cart,
            timer: timer::Timer::new(),
            serial: serial::Controller::new(),
        }
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
