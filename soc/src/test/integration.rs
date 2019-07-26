use crate::cart;
use crate::cpu;

use crate::gpu;
use crate::system;

use super::*;

pub fn run_target_with_options(target: &str, cart: Box<cart::Cart>, options: gpu::Options) -> bool {
    static INIT: std::sync::Once = std::sync::ONCE_INIT;
    let name = "ignoreme".to_string();
    INIT.call_once(|| {
        crate::log::setup_logging(crate::log::LogSettings {
            interrupts: false,
            disassembly: false,
            timer: false,
            dma: false,
            ..Default::default()
        })
        .unwrap();
    });

    let mut system = system::System::default();
    system.set_cart(cart);
    system.gpu_mut().options = options;

    let break_opcode = if target.contains("wilbert") { 0xED } else { 0x40 };

    loop {
        system.execute_machine_cycle().unwrap();
        let op = system.cpu_mut().registers.get(cpu::register::Register::INSTR);
        if op == break_opcode {
            break;
        }
    }

    system.cpu_mut().registers.get(cpu::register::Register::A) == 0
}

pub fn run_target(target: &str) -> bool {
    let mut path = base_path_to("test_roms");
    path.push(format!("{}.gb", target));
    assert!(path.exists(), "{:?} does not exist.", path);

    let cart = cart::from_file(path.to_str().unwrap());
    run_target_with_options(target, cart, gpu::Options::new())
}
