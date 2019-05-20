use crate::cart;
use crate::cpu;
use crate::cpu::register::Register;
use crate::system;

use super::image::*;
use super::*;

use std::path::Path;

macro_rules! test_target {
    (
        $($test_name:ident);*
        ;
    ) => {
        $(
            #[test]
            #[allow(non_snake_case)]
            fn $test_name() {
                let path = stringify!($test_name).replace("___", "-").replace("__", "/");
                run_target(&path);
            }
        )*
    };
}

// Timer.
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
    acceptance__timer__tima_reload;
    acceptance__timer__tima_write_reloading;
    acceptance__timer__tma_write_reloading;
);

// OAM DMA
test_target!(
    acceptance__oam_dma__basic;
    acceptance__oam_dma__reg_read;
    acceptance__oam_dma_start;
    acceptance__oam_dma_timing;
    acceptance__oam_dma_restart;
    acceptance__div_timing;
);

// Timings.
test_target!(
    acceptance__add_sp_e_timing;

    acceptance__call_timing;
    acceptance__call_timing2;
    acceptance__call_cc_timing;
    acceptance__call_cc_timing2;
    acceptance__jp_timing;
    acceptance__jp_cc_timing;
    acceptance__ld_hl_sp_e_timing;
    acceptance__ei_sequence;
    acceptance__interrupts__ie_push;
    acceptance__push_timing;
    acceptance__pop_timing;
    

    acceptance__ret_timing;
    acceptance__ret_cc_timing;

    acceptance__reti_timing;
    acceptance__reti_intr_timing;

    acceptance__rst_timing;

    acceptance__intr_timing;
    
    acceptance__ei_timing;
    acceptance__di_timing___GS;
    acceptance__rapid_di_ei;
);

// Misc
test_target!(
    acceptance__boot_regs___dmgABC;
    acceptance__halt_ime0_ei;
    acceptance__halt_ime0_nointr_timing;
    acceptance__halt_ime1_timing;
    acceptance__halt_ime1_timing2___GS;
    acceptance__if_ie_registers;
);

// PPU.
test_target!(
    acceptance__ppu__hblank_ly_scx_timing___GS;
    acceptance__ppu__intr_1_2_timing___GS;
    acceptance__ppu__intr_2_0_timing;
    acceptance__ppu__intr_2_mode0_timing;
    acceptance__ppu__intr_2_mode0_timing_sprites;
    acceptance__ppu__intr_2_mode3_timing;
    acceptance__ppu__intr_2_oam_ok_timing;
    acceptance__ppu__vblank_stat_intr___GS;
);

// Wilbert tests.
test_target!(
    wilbert__intr_0_timing;
    wilbert__intr_1_timing;
    wilbert__intr_2_timing;
    wilbert__intr_2_0_timing;
    wilbert__intr_2_mode0_timing;
    wilbert__intr_2_mode3_timing;

    wilbert__intr_2_mode0_scx1_timing_nops;
    wilbert__intr_2_mode0_scx2_timing_nops;
    wilbert__intr_2_mode0_scx3_timing_nops;
    wilbert__intr_2_mode0_scx4_timing_nops;
    wilbert__intr_2_mode0_scx5_timing_nops;
    wilbert__intr_2_mode0_scx6_timing_nops;
    wilbert__intr_2_mode0_scx7_timing_nops;
    wilbert__intr_2_mode0_scx8_timing_nops;

    wilbert__lcdon_mode_timing;
    
    wilbert__ly_lyc_0_write___GS;
    wilbert__ly_lyc_0___GS;
    wilbert__ly_lyc_144___GS;
    wilbert__ly_lyc_153_write___GS;
    wilbert__ly_lyc_153___GS;
    wilbert__ly_lyc_write___GS;

    wilbert__ly00_01_mode0_2;
    wilbert__ly00_mode0_2___GS;
    wilbert__ly00_mode2_3;
    wilbert__ly00_mode3_0;

    wilbert__ly143_144_145;
    wilbert__ly143_144_152_153;
    wilbert__ly143_144_mode0_1;
    wilbert__ly143_144_mode3_0;

    wilbert__ly_new_frame___GS;
);

//     acceptance__ppu__stat_irq_blocking;
// acceptance__ppu__intr_2_mode0_timing_sprites;
// acceptance__ppu__lcdon_timing___dmgABCmgbS;

fn run_target(target: &str) {
    static INIT: std::sync::Once = std::sync::ONCE_INIT;
    let name = "ignoreme".to_string();
    INIT.call_once(|| {
        crate::log::setup_logging(crate::log::LogSettings {
            interrupts: false,
            disassembly: false,
            timer: false,
            dma: false,
        })
        .unwrap();
    });

    use std::path::PathBuf;

    let mut path = base_path_to("test_roms");
    path.push(format!("{}.gb", target));

    let golden_path = golden_image_path(target);
    let golden_image = if golden_path.exists() {
        Some(load_golden_image(golden_path))
    } else {
        None
    };

    let cart = cart::from_file(path.to_str().unwrap());
    let mut system = system::System::new();
    system.set_cart(cart);

    let mut before_screen = system.get_screen().to_vec();

    let max_num_frames = 60 * 50;
    let max_num_frames_same_screen = 50;

    let mut num_frames = 0;
    let mut num_frames_same_screen = 0;
    while max_num_frames > 0 {
        // About 1 vsync worth of cycles.
        for _ in 0..17556 {
            system.execute_machine_cycle().unwrap();
        }
        if golden_image.is_some()
            && system.get_screen() == golden_image.as_ref().unwrap().as_slice()
        {
            break;
        } else if system.get_screen() == before_screen.as_slice() {
            num_frames_same_screen += 1;
        } else {
            before_screen = system.get_screen().to_vec();
            num_frames_same_screen = 0;
        }
        num_frames += 1;
        if num_frames_same_screen >= max_num_frames_same_screen || num_frames >= max_num_frames {
            break;
        }
    }

    if target.starts_with("wilbert") && golden_image.is_none() {
        dump_system_image(Path::new("./wilbert_golden"), target, &system);
        panic!();
    }

    if system.get_screen() != golden_image.unwrap().as_slice() {
        dump_system_image(Path::new("./failed_tests"), target, &system);
        panic!("{} failed.", target);
    } else {
        // dump_system_image(Path::new("./succeeded_tests"), target, &system);
    }
}
