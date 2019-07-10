#![warn(warnings)]
#![deny(clippy::all)]

#[macro_use]
mod window;

use structopt::StructOpt;

use window::*;

use soc::cart;
use soc::gpu;
use soc::joypad;
use soc::log;
use soc::system;

#[derive(StructOpt)]
#[structopt(name = "rusty_boy")]
struct Opt {
    #[structopt(parse(from_os_str))]
    cart_path: std::path::PathBuf,

    #[structopt(
        long = "serialize_path",
        short = "sp",
        parse(from_os_str),
        default_value = "./serialized.json"
    )]
    serialize_path: std::path::PathBuf,

    #[structopt(long)]
    fixed_window: bool,
}

// Helpful links:
// Cycle-accurate docs: https://github.com/AntonioND/giibiiadvance/blob/master/docs/TCAGBD.pdf
// https://github.com/gbdev/awesome-gbdev#emulator-development
// https://www.youtube.com/watch?v=HyzD8pNlpwI
// https://www.youtube.com/watch?v=GBYwjch6oEE
// PPU tests: https://github.com/mattcurrie/mealybug-tearoom-tests
// PPU additions to mooneye tests: https://github.com/wilbertpol/mooneye-gb/tree/master/tests

fn key_map(key: glutin::VirtualKeyCode) -> Option<joypad::Key> {
    use joypad::Key;
    let pad = match key {
        glutin::VirtualKeyCode::Space => Key::A,
        glutin::VirtualKeyCode::M => Key::B,
        glutin::VirtualKeyCode::Escape => Key::Start,
        glutin::VirtualKeyCode::Tab => Key::Select,
        glutin::VirtualKeyCode::W => Key::Up,
        glutin::VirtualKeyCode::A => Key::Left,
        glutin::VirtualKeyCode::S => Key::Down,
        glutin::VirtualKeyCode::D => Key::Right,
        _ => Key::NumKeys,
    };
    if let Key::NumKeys = pad {
        None
    } else {
        Some(pad)
    }
}

fn main() {
    let args = Opt::from_args();

    log::setup_logging(log::LogSettings {
        interrupts: true,
        disassembly: false,
        timer: false,
        dma: true,
        gpu: false,
    })
    .unwrap();

    // Set up the window.
    let mut window = Window::init(args.fixed_window);

    // Load the gameboy cart.
    let cart = cart::from_file(args.cart_path.to_str().unwrap());
    let mut system = system::System::new_complete();
    system.set_cart(cart);

    loop {
        let _now = std::time::Instant::now();
        while !system.is_vsyncing() {
            system.execute_machine_cycle().unwrap();
        }
        //println!("{} ms", _now.elapsed().as_micros() as f32 / 1000.0);
        // Update the screen.
        window.update_screen(system.get_screen());

        let mut running = true;
        for event in window.get_events() {
            match event {
                glutin::WindowEvent::CloseRequested => running = false,
                glutin::WindowEvent::KeyboardInput { input, .. }
                    if input.state == glutin::ElementState::Released =>
                {
                    match input.virtual_keycode {
                        Some(glutin::VirtualKeyCode::F7) => {
                            println!("Serializing to {}.", args.serialize_path.to_str().unwrap());
                            serialize(&system, &args)
                        }
                        Some(glutin::VirtualKeyCode::F8) => {
                            println!(
                                "Deserializing from {}.",
                                args.serialize_path.to_str().unwrap()
                            );
                            deserialize(&mut system, &args)
                        }
                        Some(virtual_key) => {
                            if let Some(key) = key_map(virtual_key) {
                                system.get_joypad_mut().release(key);
                            }
                        }
                        _ => (),
                    }
                }
                glutin::WindowEvent::KeyboardInput { input, .. }
                    if input.state == glutin::ElementState::Pressed =>
                {
                    if let Some(virtual_key) = input.virtual_keycode {
                        if let Some(key) = key_map(virtual_key) {
                            system.get_joypad_mut().press(key);
                        }
                    }
                }
                _ => (),
            }
        }
        if !running {
            break;
        }
        while system.is_vsyncing() {
            system.execute_machine_cycle().unwrap();
        }
        window.swap_buffers();
    }
}

fn serialize(system: &system::System, args: &Opt) {
    use std::fs::File;
    let file = File::create(&args.serialize_path).unwrap();
    bincode::serialize_into(file, system).unwrap();
}

fn deserialize(system: &mut system::System, args: &Opt) {
    use std::fs::File;
    let file = File::open(&args.serialize_path).unwrap();
    *system = bincode::deserialize_from(file).unwrap();
    system.restore_from_deserialize();
}
