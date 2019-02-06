use crate::cpu::micro_code::*;
use crate::cpu::register::Register;
use crate::cpu::{Cpu, Error, Result};
use crate::memory::Memory;

// TODO: Refactor into impl.
/// Executes a "Decode" special stage.
/// Assumes that the current micro-code has already read (and will increment) PC.
#[allow(clippy::cyclomatic_complexity)]
pub fn execute_decode_stage(cpu: &mut Cpu, memory: &Memory) -> Result<Option<SideEffect>> {
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

    // Validating preconditions for documentation.
    debug_assert!(op_p <= 3 && op_p >= 0);
    debug_assert!(op_y <= 7 && op_y >= 0);
    debug_assert!(op_z <= 7 && op_z >= 0);
    debug_assert!(op_x <= 3);
    debug_assert!(op_q == 0 || op_q == 1);

    // At some point this will be converted into a static table with pre-defined instruction
    // opcodes. For now, it's ease of development.

    let mut micro_codes: Vec<MicroCode> = match op_x {
        // x = 0
        0 => match op_z {
            // z = 1
            1 => match op_q {
                // q = 0
                // LD (BC/DE/HL/SP)
                // Can probably just store straight into register pairs.
                0 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .move_reg(Register::from_sp_pair_table(op_p), Register::TEMP)
                    .increment(IncrementerStage::PC)
                    .then_done()),
                1 | _ => Err(err),
            },
            // z = 2
            2 => {
                let (pair, increment) = match op_p {
                    0 => (Register::BC, None),
                    1 => (Register::DE, None),
                    2 => (Register::HL, Some(IncrementerStage::HLI)),
                    3 | _ => (Register::HL, Some(IncrementerStage::HLD)),
                };
                match op_q {
                    // q = 0
                    // LD (BC/De/HLI/HLD), A
                    0 => Ok(Builder::new()
                        .nothing_then()
                        .write_mem(pair, Register::A)
                        .maybe_increment(increment)
                        .then_done()),
                    // q = 1
                    // LD A, (BC/DE/HLI/HLD)
                    1 | _ => Ok(Builder::new()
                        .nothing_then()
                        .read_mem(Register::A, pair)
                        .maybe_increment(increment)
                        .then_done()),
                }
            }
            // z = 6. 8-bit immediate loading.
            6 => match op_y {
                // LD (HL), imm8
                6 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    // TODO: Implement stores!
                    .then_done()),
                // LD r[y], imm8
                _ => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::from_single_table(op_y), Register::PC)
                    .increment(IncrementerStage::PC)
                    .then_done()),
            },
            _ => Err(err),
        },
        // x = 1. LD r[y], r[z] with the exception of HALT.
        1 => match op_z {
            6 if op_y == 6 => Err(err), // HALT
            // z = 6. LD r[y], (HL)
            6 => Ok(Builder::new()
                .nothing_then()
                .read_mem(Register::from_single_table(op_y), Register::HL)
                .then_done()),
            // LD r[y], r[z]
            _ => Ok(Builder::new()
                .move_reg(
                    Register::from_single_table(op_y),
                    Register::from_single_table(op_z),
                )
                .then_done()),
        },
        // x = 3. Assorted.
        3 | _ => match op_z {
            // z = 2.
            2 => match op_y {
                7 => Ok(Builder::new()
                    .nothing_then()
                    .read_mem(Register::TEMP_LOW, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::TEMP_HIGH, Register::PC)
                    .increment(IncrementerStage::PC)
                    .then()
                    .read_mem(Register::A, Register::TEMP)
                    .then_done()),
                _ => Err(err),
            },
            _ => Err(err),
        },
    }?;

    // The first microcode is special in the sense that it "execute" at the same time as the decode
    // step. This, however, means that it is limited only to its alu stages.
    let setup_code = micro_codes.remove(0);
    assert!(
        setup_code.memory_stage.is_none(),
        "First micro-code of any instruction cannot contain any memory operations: {:?}.",
        setup_code.memory_stage
    );
    assert!(
        setup_code.incrementer_stage.is_none(),
        "Cannot use incrementers during the first machine cycle: {:?}.",
        setup_code.incrementer_stage
    );
    assert!(setup_code.special_stage.is_none());
    let setup_result = setup_code.execute(cpu, memory)?;
    assert!(setup_result.side_effect.is_none());
    // Microcode cannot mark as done if there are other micro codes to execute!
    assert!(!setup_result.is_done || micro_codes.is_empty());
    if micro_codes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SideEffect::Decode(micro_codes)))
    }
}
