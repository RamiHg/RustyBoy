use super::*;

use crate::gpu::registers::*;
use crate::io_registers;
use crate::system::Interrupts;

impl TestContext {
    fn ly_is_lyc(&self) -> bool {
        let stat = LcdStatus(
            self.system
                .memory_read(io_registers::Addresses::LcdStatus as i32),
        );
        stat.ly_is_lyc()
    }

    fn stat_mut(&mut self) -> &mut LcdStatus { self.system.gpu_mut().stat_mut() }

    fn line(&self) -> i32 {
        self.system
            .memory_read(io_registers::Addresses::LcdY as i32)
    }

    fn has_interrupt(&self, interrupt: Interrupts) -> bool {
        (self
            .system
            .memory_read(io_registers::Addresses::InterruptFired as i32)
            & interrupt.bits())
            != 0
    }

    fn clear_interrupts(&mut self) {
        self.system
            .memory_write(io_registers::Addresses::InterruptFired as i32, 0);
    }
}

// This test is really ugly and readable. Perhaps I will refactor it one day.
#[test]
fn test_ly_is_lyc() {
    let mut context = with_default()
        .set_mem_8bit(0xFFFF, 0xFF)
        .set_gpu_enabled()
        .execute_instructions_for_mcycles(&INF_LOOP, 1)
        .wait_for_vsync();
    context.stat_mut().set_enable_coincident_int(true);
    assert_eq!(context.gpu_mode(), LcdMode::VBlank);
    // Wait 10 * 114 clocks for vsync to end.
    for i in 0..((10 * 114) - 2) {
        context.tick();
    }

    context.clear_interrupts();
    assert_eq!(context.line(), 0);
    assert_eq!(context.gpu_mode(), LcdMode::VBlank);
    context.tick();
    assert_eq!(context.gpu_mode(), LcdMode::HBlank);
    assert_eq!(context.line(), 0);
    assert_eq!(context.ly_is_lyc(), false);
    assert!(!context.has_interrupt(Interrupts::STAT));
    context.tick();
    assert_eq!(context.gpu_mode(), LcdMode::ReadingOAM);
    assert!(!context.has_interrupt(Interrupts::STAT));
    // Get into the next line.
    for i in 0..113 {
        context.tick();
        context
            .system
            .memory_write(io_registers::Addresses::LcdYCompare as i32, 1);
    }
    for line in 1..=152 {
        if line <= 143 {
            assert_eq!(context.gpu_mode(), LcdMode::HBlank);
        }
        assert_eq!(context.ly_is_lyc(), false);
        assert_eq!(context.line(), line);
        context.tick();
        assert!(context.has_interrupt(Interrupts::STAT));
        context.clear_interrupts();
        for i in 1..114 {
            assert_eq!(context.ly_is_lyc(), i <= 112);
            if i == 112 {
                context
                    .system
                    .memory_write(io_registers::Addresses::LcdYCompare as i32, line + 1);
            }
            context.tick();
        }
    }

    assert_eq!(context.gpu_mode(), LcdMode::VBlank);
    assert_eq!(context.line(), 153);
    assert_eq!(context.ly_is_lyc(), false);
    context.tick();
    assert_eq!(context.line(), 0);
    assert_eq!(context.ly_is_lyc(), true);
    assert!(context.has_interrupt(Interrupts::STAT));
    context.clear_interrupts();
    context.tick();
    assert_eq!(context.line(), 0);
    assert!(!context.ly_is_lyc());
    assert!(!context.has_interrupt(Interrupts::STAT));
    context
        .system
        .memory_write(io_registers::Addresses::LcdYCompare as i32, 0);
    context.tick();
    assert!(context.ly_is_lyc());
    assert!(context.has_interrupt(Interrupts::STAT));
    context.clear_interrupts();
    for _ in 3..114 {
        assert_eq!(context.ly_is_lyc(), true);
        assert_eq!(context.gpu_mode(), LcdMode::VBlank);
        context.tick();
    }
    assert_eq!(context.gpu_mode(), LcdMode::HBlank);
    assert_eq!(context.ly_is_lyc(), false);
}

#[test]
fn test_vbl_int_timing() {
    let mut context = with_default()
        .set_mem_8bit(0xFFFF, 0xFF)
        .set_gpu_enabled()
        .execute_instructions_for_mcycles(&INF_LOOP, 1);
    assert_eq!(context.gpu_mode(), LcdMode::ReadingOAM);
    context.stat_mut().set_enable_vblank_int(true);
    // Wait for line 144.
    for _ in 0..114 * 144 - 1 {
        context.tick();
    }
    assert_eq!(context.line(), 144);
    assert_eq!(context.gpu_mode(), LcdMode::HBlank);
    assert!(!context.has_interrupt(Interrupts::VBLANK));
    assert!(!context.has_interrupt(Interrupts::STAT));
    context.tick();
    assert_eq!(context.gpu_mode(), LcdMode::VBlank);
    assert!(context.has_interrupt(Interrupts::VBLANK));
    assert!(context.has_interrupt(Interrupts::STAT));
    context.clear_interrupts();
    // TODO: Test the fact that the STAT interrupt can trigger again.
    context.tick();
    assert!(!context.has_interrupt(Interrupts::VBLANK));
    assert!(!context.has_interrupt(Interrupts::STAT));
}

#[test]
fn test_oam_int_timing() {
    let mut context = with_default().set_mem_8bit(0xFFFF, 0xFF).set_gpu_enabled();
    context.stat_mut().set_enable_oam_int(true);
    context = context.execute_instructions_for_mcycles(&INF_LOOP, 1);
    assert_eq!(context.gpu_mode(), LcdMode::ReadingOAM);
    assert!(context.has_interrupt(Interrupts::STAT));

    for i in 1..114 {
        context.tick();
    }

    for line in 1..=153 {
        // cycle = 0 interrupt. on lines 1 to 143
        if line <= 143 {
            assert_eq!(context.gpu_mode(), LcdMode::HBlank);
            assert!(context.has_interrupt(Interrupts::STAT));
            context.clear_interrupts();
        }
        context.tick();
        // cycle = 4 interrupt (only on lines 0 (and 153) and 144)
        if line == 0 || line == 144 || line == 153 {
            assert!(context.has_interrupt(Interrupts::STAT));
            context.clear_interrupts();
        }
        context.tick();
        assert!(!context.has_interrupt(Interrupts::STAT));
        context.tick();
        if line >= 144 {
            assert!(context.has_interrupt(Interrupts::STAT));
            context.clear_interrupts();
        }
        for _ in 3..114 {
            assert!(!context.has_interrupt(Interrupts::STAT));
            context.tick();
        }
    }
}

#[test]
fn test_nosprite_hblank_timing() {
    let mut context = with_default().set_mem_8bit(0xFFFF, 0xFF).set_gpu_enabled();
    context.stat_mut().set_enable_hblank_int(true);
    for _ in 0..144 {
        let mut prev_mode = context.gpu_mode();
        for _ in 0..114 {
            if prev_mode == LcdMode::TransferringToLcd && context.gpu_mode() == LcdMode::HBlank {
                assert!(context.has_interrupt(Interrupts::STAT));
                context.clear_interrupts();
            } else {
                assert!(!context.has_interrupt(Interrupts::STAT));
            }
            prev_mode = context.gpu_mode();
            context.tick();
        }
    }
}