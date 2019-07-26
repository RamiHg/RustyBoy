use soc::cart;
use soc::cpu;
use soc::log;
use soc::system;

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
                let result = run_target(&path);
                assert!(result);
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

    // Fails. Maybe investigate some day.
    // acceptance__interrupts__ie_push;
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
    acceptance__ppu__intr_2_mode3_timing;
    acceptance__ppu__intr_2_oam_ok_timing;
    acceptance__ppu__vblank_stat_intr___GS;

    // Not even going to try to fix this one...
    // acceptance__ppu__intr_2_mode0_timing_sprites;
);

// Wilbert tests.
test_target!(
    wilbert__intr_0_timing;
    wilbert__intr_1_timing;
    wilbert__intr_2_timing;
    wilbert__intr_2_0_timing;
    wilbert__intr_2_mode0_timing;
    wilbert__intr_2_mode3_timing;

    wilbert__hblank_ly_scx_timing___GS;
    wilbert__hblank_ly_scx_timing_nops;
    wilbert__hblank_ly_scx_timing_variant_nops;

    wilbert__vblank_if_timing;

    wilbert__intr_1_2_timing___GS;
    wilbert__vblank_stat_intr___GS;

    wilbert__intr_2_mode0_scx1_timing_nops;
    wilbert__intr_2_mode0_scx2_timing_nops;
    wilbert__intr_2_mode0_scx3_timing_nops;
    wilbert__intr_2_mode0_scx4_timing_nops;
    wilbert__intr_2_mode0_scx5_timing_nops;
    wilbert__intr_2_mode0_scx6_timing_nops;
    wilbert__intr_2_mode0_scx7_timing_nops;
    wilbert__intr_2_mode0_scx8_timing_nops;

    wilbert__lcdon_mode_timing;

    wilbert__ly_lyc___GS;

    // wilbert__ly_lyc_write___GS;
    // wilbert__ly_lyc_0_write___GS;
    // wilbert__ly_lyc_153_write___GS;

    wilbert__ly_lyc_0___GS;

    wilbert__ly00_01_mode0_2;
    wilbert__ly00_mode0_2___GS;
    wilbert__ly00_mode1_0___GS;
    wilbert__ly00_mode2_3;
    wilbert__ly00_mode3_0;

    wilbert__ly143_144_145;
    wilbert__ly143_144_152_153;
    wilbert__ly143_144_mode0_1;
    wilbert__ly143_144_mode3_0;

    wilbert__ly_lyc_144___GS;
    wilbert__ly_lyc_153___GS;

    wilbert__ly_new_frame___GS;
);

pub fn run_target(target: &str) -> bool {
    static INIT: std::sync::Once = std::sync::ONCE_INIT;
    INIT.call_once(|| {
        log::setup_logging(log::LogSettings { ..Default::default() }).unwrap();
    });

    // Load the cart.
    let cart_path = std::path::PathBuf::from("test_roms").join(target).with_extension("gb");
    assert!(cart_path.exists(), "{:?} does not exist.", cart_path);

    let mut system = system::System::default();
    system.set_cart(cart::from_file(cart_path.to_str().unwrap()));

    let break_opcode = if target.contains("wilbert") { 0xED } else { 0x40 };

    loop {
        system.execute_machine_cycle().unwrap();
        let op = system.cpu().registers.get(cpu::register::Register::INSTR);
        if op == break_opcode {
            break;
        }
    }

    system.cpu().registers.get(cpu::register::Register::A) == 0
}
