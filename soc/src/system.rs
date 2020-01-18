use bitflags::bitflags;

use crate::cart::Cart;
use crate::cpu;
use crate::error;
use crate::gpu;
use crate::io_registers;
use crate::joypad;
use crate::mmu;
use crate::{dma, serial, timer, util};

use error::Result;
use gpu::Color;

bitflags! {
    pub struct Interrupts: i32 {
        const VBLANK = 0b0001;
        const STAT   = 0b0010;
        const TIMER  = 0b0100;
        const SERIAL = 0b1000;
        const JOYPAD = 0b10000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum TState {
    T1,
    T2,
    T3,
    T4,
}

impl cpu::TState {
    pub fn get_as_tstate(self) -> TState {
        match self.get() {
            1 => TState::T1,
            2 => TState::T2,
            3 => TState::T3,
            4 => TState::T4,
            _ => panic!(),
        }
    }
}

#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct System {
    cpu: cpu::Cpu,
    gpu: gpu::Gpu,

    memory: mmu::Memory,
    timer: timer::Timer,
    serial: serial::Controller,
    dma: dma::Dma,
    joypad: joypad::Joypad,

    #[cfg_attr(feature = "serialize", serde(skip))]
    #[cfg(feature = "audio")]
    apu: Option<crate::apu::Apu>,

    #[cfg_attr(feature = "serialize", serde(skip))]
    screen: Vec<Color>,
    pub cart: Option<Box<dyn Cart>>,
}

impl Default for System {
    fn default() -> System {
        use micro_code::register::Register;
        let mut cpu = cpu::Cpu::default();
        // Set the initial register values.
        cpu.registers.set(Register::A, 0x01);
        cpu.registers.set(Register::F, 0xB0);
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::C, 0x13);
        cpu.registers.set(Register::D, 0x00);
        cpu.registers.set(Register::E, 0xD8);
        cpu.registers.set(Register::H, 0x01);
        cpu.registers.set(Register::L, 0x4D);
        cpu.registers.set(Register::SP, 0xFFFE);

        System {
            cpu,
            gpu: gpu::Gpu::default(),
            memory: mmu::Memory::default(),
            timer: timer::Timer::new(),
            serial: serial::Controller::new(),
            dma: dma::Dma::new(),
            joypad: joypad::Joypad::default(),
            screen: vec![Color::Black; (gpu::LCD_WIDTH * gpu::LCD_HEIGHT) as usize],
            cart: None,
            #[cfg(feature = "audio")]
            apu: None,
        }
    }
}

impl System {
    pub fn new_complete() -> System {
        #[allow(unused_mut)]
        let mut system = System::default();
        #[cfg(feature = "audio")]
        // Initialize audio.
        {
            system.apu = Some(Default::default());
        }
        system
    }

    pub fn restore_from_deserialize(&mut self) {
        self.screen = vec![Color::Black; (gpu::LCD_WIDTH * gpu::LCD_HEIGHT) as usize];
        #[cfg(feature = "audio")]
        {
            self.apu = Some(Default::default());
        }
    }
    pub fn set_cart(&mut self, cart: Box<dyn Cart>) {
        self.cart = Some(cart);
    }

    pub fn cpu(&self) -> &cpu::Cpu {
        &self.cpu
    }
    pub fn gpu(&self) -> &gpu::Gpu {
        &self.gpu
    }
    pub fn screen(&self) -> &[Color] {
        &self.screen
    }

    pub fn joypad_mut(&mut self) -> &mut joypad::Joypad {
        &mut self.joypad
    }

    fn read_request(&self, raw_address: i32) -> Result<i32> {
        let modules: &[Option<&dyn mmu::MemoryMapped>] = &[
            Some(&self.timer),
            Some(self.cart.as_ref().unwrap().as_ref().as_ref()),
            Some(&self.serial),
            Some(&self.gpu),
            Some(&self.dma),
            Some(&self.joypad),
            #[cfg(feature = "audio")]
            self.apu.as_ref().map(|x| -> &dyn mmu::MemoryMapped { x }),
            Some(&self.memory),
        ];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in modules.iter().flatten() {
            if let Some(result) = module.read(address) {
                return Ok(result);
            }
        }
        Err(error::Type::TODOMemoryBus)
    }

    fn write_request(&mut self, raw_address: i32, value: i32) -> Result<()> {
        let modules: &mut [Option<&mut dyn mmu::MemoryMapped>] = &mut [
            Some(&mut self.timer),
            Some(self.cart.as_mut().unwrap().as_mut().as_mut()),
            Some(&mut self.serial),
            Some(&mut self.gpu),
            Some(&mut self.dma),
            Some(&mut self.joypad),
            #[cfg(feature = "audio")]
            self.apu.as_mut().map(|x| -> &mut dyn mmu::MemoryMapped { x }),
            Some(&mut self.memory),
        ];
        let address = mmu::Address::from_raw(raw_address)?;
        for module in modules.iter_mut().flatten() {
            if let Some(()) = module.write(address, value) {
                return Ok(());
            }
        }
        Err(error::Type::TODOMemoryBus)
    }

    fn is_invalid_source_address(address: i32) -> bool {
        address >= 0xFE00 && address < 0xFEA0
    }

    fn handle_cpu_memory_reads(&mut self) -> Result<()> {
        debug_assert!(!(self.cpu.state.read_latch && self.cpu.state.write_latch));
        let t_state = self.cpu.t_state.get();
        if self.cpu.state.read_latch {
            if t_state >= 2 {
                // During DMA, reads return 0xFF.
                let maybe_data = if self.dma.is_active()
                    && System::is_invalid_source_address(self.cpu.state.address_latch)
                    && self.cpu.state.address_latch > 0x100
                {
                    strict_fail!("Reading during dma {:X}", self.cpu.state.address_latch);
                    Ok(0xFF)
                } else {
                    self.read_request(self.cpu.state.address_latch)
                };
                if let Ok(value) = maybe_data {
                    self.cpu.state.data_latch = value;
                } else {
                    match self.cpu.state.address_latch {
                        // TODO: Cleanup. These registers are provided from the MemoryBus hacky
                        // system.
                        0xFF41 | 0xFF40 | 0xFF45 | 0xFF44 => (),
                        _ => {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "Could not find any memory module accepting {:X}.",
                                self.cpu.state.address_latch
                            );
                            panic!()
                        }
                    }
                }
            } else if false {
                // Write garbage in data latch to catch bad reads.
                self.cpu.state.data_latch = -1;
            }
        }
        Ok(())
    }

    fn handle_cpu_memory_writes(&mut self) -> Result<()> {
        // Service write requests at T=4's rising edge.
        if self.cpu.state.write_latch {
            strict_assert!(
                !self.dma.is_active(),
                "Attempting to write to {:X} while DMA is active.",
                self.cpu.state.address_latch
            );
            if self.dma.is_active() {
                return Ok(());
            }
            debug_assert!(util::is_16bit(self.cpu.state.address_latch));
            debug_assert!(util::is_8bit(self.cpu.state.data_latch));
            if self.cpu.t_state.get() == 4
                && !(self.dma.is_active()
                    && System::is_invalid_source_address(self.cpu.state.address_latch))
            {
                self.write_request(self.cpu.state.address_latch, self.cpu.state.data_latch)?;
            }
        }
        Ok(())
    }

    fn maybe_fire_interrupt(&mut self, maybe_fire: Interrupts) {
        let mut current_if = self.memory.read(io_registers::Addresses::InterruptFired);
        current_if |= maybe_fire.bits();
        self.memory.store(io_registers::Addresses::InterruptFired, current_if);
    }

    fn temp_hack_get_bus(&self) -> mmu::MemoryBus {
        let is_dma =
            self.dma.is_active() && System::is_invalid_source_address(self.cpu.state.address_latch);
        mmu::MemoryBus {
            address_latch: self.cpu.state.address_latch,
            data_latch: self.cpu.state.data_latch,
            read_latch: self.cpu.state.read_latch && !is_dma,
            write_latch: self.cpu.state.write_latch && !is_dma,
            t_state: self.cpu.t_state.get(),
        }
    }

    fn handle_dma(&mut self) -> Result<()> {
        let bus = self.temp_hack_get_bus();
        let request = self.dma.execute_tcycle(&bus);
        if let Some(request) = request {
            let value = self.read_request(request.source_address)?;
            trace!(target: "dma", "Setting {:X?} from {:X?} with {:X?}", request.destination_address, request.source_address, value);
            // Since we know the destination has to be OAM, skip the mmu routing.
            mmu::MemoryMapped::write(
                &mut self.gpu,
                mmu::Address::from_raw(request.destination_address)?,
                value,
            )
            .ok_or_else(|| error::Type::InvalidOperation("DMA destination was not OAM".into()))
        } else {
            Ok(())
        }
    }

    fn handle_timer(&mut self) -> Result<()> {
        let bus = self.temp_hack_get_bus();
        let (new_timer, should_interrupt) = self.timer.execute_tcycle(&bus);
        self.timer = new_timer;
        self.maybe_fire_interrupt(should_interrupt);
        Ok(())
    }

    fn handle_serial(&mut self) -> serial::Controller {
        let (new_serial, should_interrupt) = self.serial.execute_tcycle();
        self.maybe_fire_interrupt(should_interrupt);
        new_serial
    }

    fn handle_gpu(&mut self) {
        let mut bus = self.temp_hack_get_bus();
        self.gpu.execute_tcycle_tick(self.cpu.t_state.get_as_tstate(), &mut bus);
        let should_interrupt = self.gpu.execute_tcycle_tock(
            self.cpu.t_state.get_as_tstate(),
            &mut bus,
            &mut self.screen,
        );
        if self.gpu.at_vblank() {
            self.maybe_fire_interrupt(Interrupts::VBLANK);
        }
        self.cpu.state.data_latch = bus.data_latch;
        self.maybe_fire_interrupt(should_interrupt);
    }

    fn handle_joypad(&mut self) {
        let should_interrupt = self.joypad.execute_tcycle();
        self.maybe_fire_interrupt(should_interrupt);
    }

    fn execute_tcycle(&mut self) -> Result<()> {
        #[cfg(feature = "disas")]
        self.print_disassembly()?;
        // Do all the rising edge sampling operations.
        self.handle_cpu_memory_reads()?;
        self.handle_gpu();
        self.cpu.execute_t_cycle(&mut self.memory, self.gpu.hack())?;
        self.handle_timer()?;
        let new_serial = self.handle_serial();

        self.handle_joypad();
        // Finally, do all the next state replacement.
        self.serial = new_serial;
        self.handle_cpu_memory_writes()?;
        // Last step is DMA.
        self.handle_dma()?;
        self.cpu.t_state.inc();

        Ok(())
    }

    pub fn execute_machine_cycle(&mut self) -> Result<()> {
        for _ in 0..4 {
            self.execute_tcycle()?;
        }
        Ok(())
    }

    pub fn is_vsyncing(&self) -> bool {
        self.gpu.is_vsyncing()
    }

    #[cfg(test)]
    pub fn is_fetching(&self) -> bool {
        self.cpu.state.decode_mode == cpu::DecodeMode::Fetch
    }

    #[cfg(feature = "disas")]
    fn print_disassembly(&self) -> Result<()> {
        if self.cpu.t_state.get() == 1
            && self.cpu.state.decode_mode == cpu::DecodeMode::Fetch
            && !self.cpu.is_handling_interrupt
            && !self.cpu.is_halted
        {
            let pc = self.cpu.registers.get(cpu::register::Register::PC);
            trace!(target: "disas", "SP {:X}", self.cpu.registers.get(cpu::register::Register::SP));
            if pc >= 0xFFFD {
                // Can probably just fix this if indeed some instructions are here.
                trace!(target: "disas", "PC too large, at {:X}", pc);
                return Ok(());
            }
            let pc_plus = |x| self.read_request(pc + x);
            let disas =
                gb_disas::decode::decode(pc_plus(0)? as u8, pc_plus(1)? as u8, pc_plus(2)? as u8);
            if let core::result::Result::Ok(op) = disas {
                trace!(
                    target: "disas",
                    "{:04X?}\t{:X?}\t{}",
                    self.cpu.registers.get(cpu::register::Register::PC),
                    pc_plus(0)?,
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
        Ok(())
    }
}

#[cfg(test)]
impl System {
    pub fn new_test_system(cart: Box<dyn Cart>) -> System {
        let mut system = System::default();
        system.set_cart(cart);
        // Clear all registers.
        system.cpu.registers = cpu::register::File::default();
        // Start with the GPU disabled.
        system.write_request(io_registers::Addresses::LcdControl as i32, 0).unwrap();
        // And the timer.
        system.memory_write(io_registers::Addresses::TimerControl as i32, 0);
        system
    }

    // pub fn memory_mut(&mut self) -> &mut mmu::Memory { &mut self.memory }
    pub fn memory(&self) -> &mmu::Memory {
        &self.memory
    }
    pub fn cpu_mut(&mut self) -> &mut cpu::Cpu {
        &mut self.cpu
    }
    pub fn gpu_mut(&mut self) -> &mut gpu::Gpu {
        &mut self.gpu
    }

    pub fn memory_read(&self, raw_address: i32) -> i32 {
        use mmu::Address;
        use num_traits::FromPrimitive;
        match Address::from_raw(raw_address) {
            Ok(Address(mmu::Location::Registers, _)) => {
                use io_registers::Addresses::*;
                match io_registers::Addresses::from_i32(raw_address).unwrap() {
                    LcdControl => Some(self.gpu.ctrl()),
                    LcdStatus => Some(self.gpu.stat()),
                    LcdY => Some(self.gpu.y()),
                    LcdYCompare => Some(self.gpu.lyc()),
                    _ => None,
                }
            }
            Ok(Address(mmu::Location::OAM, _)) => {
                println!("Reading it from {}", raw_address);
                Some(self.gpu.oam(raw_address) as i32)
            }
            _ => None,
        }
        .unwrap_or_else(|| self.read_request(raw_address).unwrap())
    }

    pub fn memory_write(&mut self, raw_address: i32, value: i32) {
        use crate::io_registers::Register as _;
        use num_traits::FromPrimitive;

        debug_assert!(util::is_8bit(value));
        if let Some(address) = io_registers::Addresses::from_i32(raw_address) {
            use io_registers::Addresses::*;
            match address {
                LcdControl => self.gpu.ctrl_mut().set(value),
                LcdStatus => self.gpu.stat_mut().set(value),
                //LcdYCompare => return self.gpu.lyc(),
                _ => (),
            };
        }
        if raw_address == io_registers::Addresses::Dma as i32 {
            self.dma.set_control(value);
        } else if raw_address == io_registers::Addresses::TimerControl as i32 {
            self.timer.set_control(value);
        } else {
            self.write_request(raw_address, value).unwrap();
        }
    }
    pub fn memory_read_16(&self, raw_address: i32) -> i32 {
        self.memory_read(raw_address) | (self.memory_read(raw_address + 1) << 8)
    }
}
