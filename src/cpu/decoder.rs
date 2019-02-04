use crate::cpu::micro_code::*;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Error, Result};
use crate::memory::Memory;

pub fn execute(cpu: &mut Cpu, memory: &Memory) -> Result<InstrResult> {
    // This method uses the amazing guide to decode instructions programatically:
    // https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html

    let opcode = cpu.registers.get(Register::TEMP_LOW);
    // Deconstruct the op into its components.
    let op_z = opcode & 0b00000111;
    let op_y = (opcode & 0b00111000) >> 3;
    let op_x = (opcode & 0b11000000) >> 6;
    let op_q = op_y & 0b001;
    let op_p = (op_y & 0b110) >> 1;

    let err = Error::InvalidOpcode(opcode);

    // Validating for documentation things that are tautologies.
    debug_assert!(op_p <= 3);

    //#[allow(unreachable_patterns)]
    let mut micro_codes: Vec<MicroCode> = match op_x {
        // x = 0
        0 => match op_z {
            // z = 2
            2 => match op_q {
                // q = 1
                1 => match op_p {
                    0 => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, Register::BC)
                        .then_done()),
                    1 => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, Register::DE)
                        .then_done()),
                    2 => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, Register::DE)
                        .then_done()),
                    3 | _ => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, Register::DE)
                        .then_done()),
                },
                _ => Err(err),
            },
            _ => Err(err),
        },
        // // x = 1
        // 1 => match op_z {
        //     6 => match op_y {
        //         6 => Err(err),
        //         dest_reg => Ok(IndirectLoad::new(
        //             SingleTable::from_i32(dest_reg).unwrap(),
        //             Register::HL,
        //             HLOp::None,
        //         )
        //         .into()),
        //     },
        //     _ => Err(err),
        // },
        _ => Err(err),
    }?;

    // Execute the first microcode immediately.
    let setup_code = micro_codes.remove(0);
    assert!(
        setup_code.memory_stage == MemoryStage::None,
        "First micro-code of any instruction cannot contain any memory operations: {:?}.",
        setup_code.memory_stage
    );
    setup_code.execute(cpu, memory)?;

    Ok(InstrResult::Decode(micro_codes))
}
