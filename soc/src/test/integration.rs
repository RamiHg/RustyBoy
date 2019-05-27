use crate::cart;
use crate::cpu;
use crate::cpu::register::Register;
use crate::gpu;
use crate::system;

use super::image::*;
use super::*;

use std::path::Path;
use std::rc::Rc;

pub fn run_target_with_options(
    target: &str,
    cart: Box<cart::Cart>,
    golden_image: &Option<Vec<gpu::Pixel>>,
    options: gpu::Options,
) -> bool {
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

    let mut system = system::System::new();
    system.set_cart(cart);
    system.gpu_mut().options = options;
    system.gpu_mut().enable_display();

    let mut before_screen = system.get_screen().to_vec();

    let max_num_frames = 60 * 50;
    let max_num_frames_same_screen = 5;
    let max_num_white_screen = 50;

    let mut num_frames = 0;
    let mut num_frames_same_screen = 0;
    let mut num_white_screen = 0;
    while max_num_frames > 0 {
        while !system.is_vsyncing() {
            system.execute_machine_cycle().unwrap();
        }
        while system.is_vsyncing() {
            system.execute_machine_cycle().unwrap();
        }
        if golden_image.is_some()
            && system.get_screen() == golden_image.as_ref().unwrap().as_slice()
        {
            break;
        } else if is_white_screen(system.get_screen()) {
            num_white_screen += 1;
        } else if system.get_screen() == before_screen.as_slice() {
            num_frames_same_screen += 1;
        } else {
            before_screen = system.get_screen().to_vec();
            num_frames_same_screen = 0;
        }
        num_frames += 1;
        if num_frames_same_screen >= max_num_frames_same_screen
            || num_frames >= max_num_frames
            || num_white_screen >= max_num_white_screen
        {
            break;
        }
    }

    // loop {
    //     system.execute_machine_cycle().unwrap();
    //     if system
    //         .cpu_mut()
    //         .registers
    //         .get(cpu::register::Register::INSTR)
    //         == 0x40
    //     {
    //         break;
    //     }
    // }
    //

    // let passes = system.cpu_mut().registers.get(cpu::register::Register::A) == 0;
    // return passes;

    if target.starts_with("wilbert") && golden_image.is_none() {
        //dump_system_image(Path::new("./wilbert_golden"), target, &system);
        return false;
    }

    if system.get_screen() != golden_image.as_ref().unwrap().as_slice() {
        //dump_system_image(Path::new("./failed_tests"), target, &system);
        return false;
    } else {
        // dump_system_image(Path::new("./succeeded_tests"), target, &system);
    }

    true
}

pub fn run_target(target: &str) -> bool {
    use std::path::PathBuf;
    let golden_path = golden_image_path(target);
    let golden_image = if golden_path.exists() {
        Some(load_golden_image(golden_path))
    } else {
        None
    };

    let mut path = base_path_to("test_roms");
    path.push(format!("{}.gb", target));

    let cart = cart::from_file(path.to_str().unwrap());
    run_target_with_options(target, cart, &golden_image, gpu::Options::new())
}
