use crate::cart;
use crate::cpu;
use crate::cpu::register::Register;
use crate::system;

use super::*;

macro_rules! test_target {
    (
        $($test_name:ident);*
        ;
    ) => {
        $(
            #[test]
            #[allow(non_snake_case)]
            fn $test_name() {
                let path = stringify!($test_name).replace("__", "/");
                run_target(path.as_str());
            }
        )*
    };
}

test_target!(
    acceptance__timer__div_write;
    acceptance__timer__rapid_toggle;
    acceptance__timer__tim00;
    acceptance__timer__tim00_div_trigger;
    acceptance__timer__tim01;
    acceptance__timer__tim01_div_trigger;
    acceptance__timer__tim10;
    acceptance__timer__tim10_div_trigger;
    acceptance__timer__tim11;
    acceptance__timer__tim11_div_trigger;
);

fn run_target(target: &str) {
    use std::path::PathBuf;

    let mut path = PathBuf::from("./test_roms");
    path.push(format!("{}.gb", target));

    let cart = cart::from_file(path.to_str().unwrap());
    let mut system = system::System::new_with_cart(cart);

    let mut before_screen = system.get_screen().to_vec();

    let max_num_frames = 60 * 50;
    let max_num_frames_same_screen = 35;

    let mut num_frames = 0;
    let mut num_frames_same_screen = 0;
    while max_num_frames > 0 {
        // About 10 vsyncs worth of cycles.
        for _ in 0..175560 {
            system.execute_machine_cycle().unwrap();
        }
        if system.get_screen() == before_screen.as_slice() {
            num_frames_same_screen += 10;
        } else {
            before_screen = system.get_screen().to_vec();
            num_frames_same_screen = 0;
        }
        num_frames += 10;
        if num_frames_same_screen >= max_num_frames_same_screen || num_frames >= max_num_frames {
            break;
        }
    }

    if system.get_screen() != load_golden_image(target).as_slice() {
        dump_system_image(Path::new("./failed_tests"), target, &system);
        panic!("{} failed.", target);
    } else {
        dump_system_image(Path::new("./succeeded_tests"), target, &system);
    }
}
