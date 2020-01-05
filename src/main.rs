#![warn(warnings)]
#![deny(clippy::all)]

#[macro_use]
mod window;
mod sim;

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
        default_value = "./serialized.bincode"
    )]
    #[cfg(feature = "serialize")]
    serialize_path: std::path::PathBuf,

    // Logging.
    #[structopt(long)]
    log_audio: bool,
}

// Helpful links:
// Cycle-accurate docs: https://github.com/AntonioND/giibiiadvance/blob/master/docs/TCAGBD.pdf
// https://github.com/gbdev/awesome-gbdev#emulator-development
// https://www.youtube.com/watch?v=HyzD8pNlpwI
// https://www.youtube.com/watch?v=GBYwjch6oEE
// PPU tests: https://github.com/mattcurrie/mealybug-tearoom-tests
// PPU additions to mooneye tests: https://github.com/wilbertpol/mooneye-gb/tree/master/tests

fn key_map(key: glutin::event::VirtualKeyCode) -> Option<joypad::Key> {
    use glutin::event::VirtualKeyCode;
    use joypad::Key;

    let pad = match key {
        VirtualKeyCode::Space => Key::A,
        VirtualKeyCode::M => Key::B,
        VirtualKeyCode::Escape => Key::Start,
        VirtualKeyCode::Tab => Key::Select,
        VirtualKeyCode::W => Key::Up,
        VirtualKeyCode::A => Key::Left,
        VirtualKeyCode::S => Key::Down,
        VirtualKeyCode::D => Key::Right,
        _ => Key::NumKeys,
    };
    if let Key::NumKeys = pad {
        None
    } else {
        Some(pad)
    }
}

fn main() {
    use glutin::event::Event;
    use glutin::event::{ElementState, KeyboardInput, WindowEvent};
    use glutin::event_loop::ControlFlow;
    use std::time::Instant;

    let args = Opt::from_args();

    log::setup_logging(log::LogSettings {
        interrupts: false,
        disassembly: false,
        timer: false,
        dma: false,
        gpu: false,
        audio: args.log_audio,
    })
    .unwrap();

    // Create the simulator.
    let mut simulator = {
        // Load the gameboy cart.
        let cart = cart::from_file(args.cart_path.to_str().unwrap());
        let mut system = system::System::new_complete();
        system.set_cart(cart);
        sim::Simulator::with_system(system)
    };

    // Set up the window.
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = Window::with_event_loop(&event_loop);

    let mut last_screen = Vec::new();

    // And just run!
    let mut sim_timer = Instant::now();
    let mut fps_timer = Instant::now();
    let mut fps_counter = 0;

    event_loop.run(move |event, _, control_flow| {
        let elapsed = sim_timer.elapsed();
        sim_timer += elapsed;

        // We will never get events if we don't ask for them, so we sleep, but not for too long.
        *control_flow = ControlFlow::WaitUntil(sim_timer + std::time::Duration::from_millis(1));

        // Handle any window event now.
        if let Event::RedrawRequested(..) = event {
            if !last_screen.is_empty() {
                window.update_screen(&last_screen);
            }
            window.swap_buffers();

            fps_counter += 1;
            let elapsed = fps_timer.elapsed();
            if elapsed.as_secs() > 0 {
                fps_timer += elapsed;
                println!("Avg FPS: {}", fps_counter / elapsed.as_secs());
                fps_counter = 0;
            }
        } else if let Event::WindowEvent { event, .. } = event {
            match event {
                // CloseRequested. End the loop.
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                // Handle any keyboard input.
                WindowEvent::KeyboardInput {
                    input: KeyboardInput { virtual_keycode, state, .. },
                    ..
                } => {
                    if let Some(key) = virtual_keycode.and_then(key_map) {
                        if let ElementState::Pressed = state {
                            simulator.press_key(key);
                        } else {
                            simulator.release_key(key);
                        }
                    }
                }
                _ => (),
            }
        }

        // Run the simulation!
        if let Some(screen) = simulator.update(elapsed.as_micros() as f32 * 1e-6) {
            window.request_redraw();
            last_screen = screen;
        }
    });

    // loop {
    //     let _now = std::time::Instant::now();
    //     while !system.is_vsyncing() {
    //         system.execute_machine_cycle().unwrap();
    //     }
    //     //println!("{} ms", _now.elapsed().as_micros() as f32 / 1000.0);
    //     // Update the screen.
    //     window.update_screen(system.get_screen());

    //     let mut running = true;
    //     for event in window.get_events() {
    //         match event {
    //             glutin::WindowEvent::CloseRequested => running = false,
    //             glutin::WindowEvent::KeyboardInput { input, .. }
    //                 if input.state == glutin::ElementState::Released =>
    //             {
    //                 match input.virtual_keycode {
    //                     Some(glutin::VirtualKeyCode::F7) => {
    //                         println!("Serializing to {}.", args.serialize_path.to_str().unwrap());
    //                         serialize(&system, &args)
    //                     }
    //                     Some(glutin::VirtualKeyCode::F8) => {
    //                         println!(
    //                             "Deserializing from {}.",
    //                             args.serialize_path.to_str().unwrap()
    //                         );
    //                         deserialize(&mut system, &args)
    //                     }
    //                     Some(virtual_key) => {
    //                         if let Some(key) = key_map(virtual_key) {
    //                             system.get_joypad_mut().release(key);
    //                         }
    //                     }
    //                     _ => (),
    //                 }
    //             }
    //             glutin::WindowEvent::KeyboardInput { input, .. }
    //                 if input.state == glutin::ElementState::Pressed =>
    //             {
    //                 if let Some(virtual_key) = input.virtual_keycode {
    //                     if let Some(key) = key_map(virtual_key) {
    //                         system.get_joypad_mut().press(key);
    //                     }
    //                 }
    //             }
    //             _ => (),
    //         }
    //     }
    //     if !running {
    //         break;
    //     }
    //     while system.is_vsyncing() {
    //         system.execute_machine_cycle().unwrap();
    //     }
    //     window.swap_buffers();
    // }
}

#[cfg(feature = "serialize")]
fn serialize(system: &system::System, args: &Opt) {
    use std::fs::File;
    let file = File::create(&args.serialize_path).unwrap();
    bincode::serialize_into(file, system).unwrap();
}

#[cfg(feature = "serialize")]
fn deserialize(system: &mut system::System, args: &Opt) {
    use std::fs::File;
    let file = File::open(&args.serialize_path).unwrap();
    *system = bincode::deserialize_from(file).unwrap();
    system.restore_from_deserialize();
}
