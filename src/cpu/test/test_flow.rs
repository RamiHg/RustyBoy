use super::*;
use crate::cpu::register::Register::*;

#[rustfmt::skip]
fn simple_destination(return_addr: usize) -> Vec<u8> {
    vec![
        // INC A
        INC_A,
        // JP return_addr
        JP, return_addr as u8, (return_addr >> 8) as u8
    ]
}

#[test]
fn test_jp() {
    #[rustfmt::skip]
    let ops: [u8; 4] = [
        // JP 0xD000
        JP, 0x00, 0xD0, // INC A
        DEC_A,
    ];
    with_default()
        .set_mem_range(0xD000, &simple_destination(0xC000 + ops.len()))
        .execute_instructions(&ops)
        .assert_reg_eq(A, 1)
        .assert_mcycles(4 + 1 + 4);
}

#[test]
fn test_jp_hl() {
    #[rustfmt::skip]
    let ops: [u8; 2] = [
        // JP HL
        0xE9,
        DEC_A
    ];
    with_default()
        .set_mem_range(0xD000, &simple_destination(0xC000 + ops.len()))
        .set_reg(HL, 0xD000)
        .execute_instructions(&ops)
        .assert_reg_eq(A, 1)
        .assert_mcycles(1 + 1 + 4);
}

#[test]
fn test_jr() {
    #[rustfmt::skip]
    let ops: Vec<u8> = vec![
        // JR 10
        0x18, 0x0A,
        DEC_A, DEC_A, DEC_A, DEC_A, DEC_A,
         // JP 0xD000
        JP, 0x00, 0xD0,
        DEC_A, DEC_A,
         // JR -7
        0x18, 0xF9
    ];
    with_default()
        .set_mem_range(0xD000, &simple_destination(0xC000 + ops.len()))
        .execute_instructions(&ops)
        .assert_reg_eq(A, 1)
        .assert_mcycles(3 + 3 + 4 + 1 + 4);
}

#[test]
fn test_jp_cc() {
    for &(op, flag, is_set) in &[
        (0xC2, Flags::ZERO, false),
        (0xCA, Flags::ZERO, true),
        (0xD2, Flags::CARRY, false),
        (0xDA, Flags::CARRY, true),
    ] {
        #[rustfmt::skip]
        let ops: [u8; 4] = [
            // JP cc, 0xD000
            op, 0x00, 0xD0,
            DEC_A
        ];
        with_default()
            .set_mem_range(0xD000, &simple_destination(0xC000 + ops.len()))
            .set_flag(flag, is_set)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 1)
            .assert_mcycles(4 + 1 + 4);
        with_default()
            .set_mem_range(0xD000, &simple_destination(0xC000 + ops.len()))
            .set_flag(flag, !is_set)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 0xFF)
            .assert_mcycles(3 + 1);
    }
}

#[test]
fn test_jr_cc() {
    for &(op, flag, is_set) in &[
        (0x20, Flags::ZERO, false),
        (0x28, Flags::ZERO, true),
        (0x30, Flags::CARRY, false),
        (0x38, Flags::CARRY, true),
    ] {
        #[rustfmt::skip]
        let ops: Vec<u8> = vec![
            // JR cc, 1
            op, 0x01,
            DEC_A,
            INC_A
        ];
        with_default()
            .set_flag(flag, is_set)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 1)
            .assert_mcycles(3 + 1);
        let mut expected_flags = Flags::ZERO | Flags::HCARRY;
        if !is_set {
            expected_flags |= flag;
        };
        with_default()
            .set_flag(flag, !is_set)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 0)
            .assert_flags(expected_flags)
            .assert_mcycles(2 + 1 + 1);
    }
}

#[test]
/// Tests the basic functionality of CALL. I.e., stack address, new PC.
fn test_call_bootstrap() {
    #[rustfmt::skip]
    let ops: Vec<u8> = vec![
        // CALL 0xD000
        0xCD, 0x00, 0xD0
    ];
    with_default()
        .set_reg(Register::SP, 0xFFFF)
        .execute_instructions_for_mcycles(&ops, 6)
        .assert_reg_eq(Register::PC, 0xD000)
        .assert_reg_eq(Register::SP, 0xFFFF - 2)
        .assert_mem_16bit_eq(0xFFFF - 2, 0xC003)
        .assert_mcycles(6);
}

#[test]
fn test_call_ret() {
    // Build the call site.
    #[rustfmt::skip]
    let call_site: Vec<u8>  = vec![
        INC_A,
        // RET
        0xC9
    ];
    #[rustfmt::skip]
    let ops: Vec<u8> = vec![
        // CALL 0xD000
        0xCD, 0x00, 0xD0,
        DEC_A,
    ];
    with_default()
        .set_mem_range(0xD000, &call_site)
        .execute_instructions(&ops)
        .assert_reg_eq(A, 0)
        .assert_flags(Flags::ZERO | Flags::SUB)
        .assert_mcycles(6 + 1 + 4 + 1);
}

#[test]
fn test_call_cc() {
    for &(op, flag, is_set) in &[
        (0xC4, Flags::ZERO, false),
        (0xCC, Flags::ZERO, true),
        (0xD4, Flags::CARRY, false),
        (0xDC, Flags::CARRY, true),
    ] {
        // Build the call site.
        #[rustfmt::skip]
        let call_site: Vec<u8>  = vec![
            INC_A,
            // RET
            0xC9
        ];
        #[rustfmt::skip]
        let ops: Vec<u8> = vec![
            // CALL cc, 0xD000
            op, 0x00, 0xD0,
            DEC_A,
            INC_A
        ];
        with_default()
            .set_flag(flag, is_set)
            .set_reg(Register::SP, 0xFFFF)
            .set_mem_range(0xD000, &call_site)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 1)
            .assert_mcycles(6 + 1 + 4 + 2);
        let mut expected_flags = Flags::ZERO | Flags::HCARRY;
        if !is_set {
            expected_flags |= flag;
        };
        with_default()
            .set_flag(flag, !is_set)
            .set_reg(Register::SP, 0xFFFF)
            .set_mem_range(0xD000, &call_site)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 0)
            .assert_flags(expected_flags)
            .assert_mcycles(3 + 2);
    }
}

#[test]
fn test_ret_cc() {
    for &(op, flag, is_set) in &[
        (0xC0, Flags::ZERO, false),
        (0xC8, Flags::ZERO, true),
        (0xD0, Flags::CARRY, false),
        (0xD8, Flags::CARRY, true),
    ] {
        // Build the call site.
        #[rustfmt::skip]
        let call_site: Vec<u8>  = vec![
            // RET cc
            op,
            INC_A,
            // RET
            0xC9
        ];
        #[rustfmt::skip]
        let ops: Vec<u8> = vec![
            // CALL 0xD000
            0xCD, 0x00, 0xD0,
        ];
        with_default()
            .set_flag(flag, is_set)
            .set_reg(Register::SP, 0xFFFF)
            .set_mem_range(0xD000, &call_site)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 0)
            .assert_mcycles(6 + 5);
        with_default()
            .set_flag(flag, !is_set)
            .set_reg(Register::SP, 0xFFFF)
            .set_mem_range(0xD000, &call_site)
            .execute_instructions(&ops)
            .assert_reg_eq(A, 1)
            .assert_mcycles(6 + 2 + 1 + 4);
    }
}
