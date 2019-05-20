#![warn(warnings)]
#![deny(clippy::all)]

#[macro_use]
mod graphics;

use structopt::StructOpt;

use graphics::*;

use soc::cart;
use soc::error;
use soc::gpu;
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
    little: bool,
}

// Helpful links:
// Cycle-accurate docs: https://github.com/AntonioND/giibiiadvance/blob/master/docs/TCAGBD.pdf
// https://github.com/gbdev/awesome-gbdev#emulator-development
// https://www.youtube.com/watch?v=HyzD8pNlpwI
// https://www.youtube.com/watch?v=GBYwjch6oEE
// PPU tests: https://github.com/mattcurrie/mealybug-tearoom-tests
// PPU additions to mooneye tests: https://github.com/wilbertpol/mooneye-gb/tree/master/tests

/// Helpful macro to run a GL command and make sure no errors are generated.

fn main() -> error::Result<()> {
    let args = Opt::from_args();
    let little = args.little;

    log::setup_logging(log::LogSettings {
        interrupts: little,
        disassembly: little,
        timer: false,
        dma: false,
    })
    .unwrap();

    // Set up the window.
    let mut window = Window::init();

    // Load the gameboy cart.
    let cart = cart::from_file(args.cart_path.to_str().unwrap());
    let mut system = system::System::new();
    system.set_cart(cart);
    loop {
        //let now = std::time::Instant::now();
        if little {
            while !system.is_vsyncing() {
                system.execute_machine_cycle()?;
            }
            for _ in 0..37000 {
                system.execute_machine_cycle()?;
            }
        } else {
            while system.is_vsyncing() {
                system.execute_machine_cycle()?;
            }
            while !system.is_vsyncing() {
                system.execute_machine_cycle()?;
            }
        }
        //println!("{} ms", now.elapsed().as_micros() as f32 / 1000.0);
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
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        if !running {
            break;
        }
        window.swap_buffers();
        if little {
            break;
        }
    }

    Ok(())
}

fn serialize(system: &system::System, args: &Opt) {
    use std::fs::File;
    let file = File::create(&args.serialize_path).unwrap();
    serde_json::to_writer_pretty(file, system).unwrap();
}

fn deserialize(system: &mut system::System, args: &Opt) {
    use std::fs::File;
    let file = File::open(&args.serialize_path).unwrap();
    *system = serde_json::from_reader(file).unwrap();
    system.restore_from_deserialize();
    system.set_cart(cart::from_file(args.cart_path.to_str().unwrap()));
}
