use super::asm::{AluCommand, Arg, Command, Op};
use super::micro_code::{AluOutSelect, MicroCode};
use crate::cpu::register::Register;

impl AluOutSelect {
    fn from_register(register: &Register) -> Option<AluOutSelect> {
        use AluOutSelect::*;
        match register {
            Register::ALU_TMP => Some(Tmp),
            Register::A => Some(A),
            Register::ACT => Some(ACT),
            Register::F => Some(F),
            _ => None,
        }
    }
}

fn expect_arg(maybe_arg: &Option<Arg>) -> &Arg {
    maybe_arg
        .as_ref()
        .unwrap_or_else(|| panic!("Op missing a required argument."))
}

pub fn compile_op(op: &Op) -> MicroCode {
    let lhs = || expect_arg(&op.lhs);
    let rhs = || expect_arg(&op.rhs);

    use Command::*;
    match op.cmd {
        LD => compile_ld(lhs(), rhs()),
        _ => {
            println!("Quietly ignoring {:?}", op);
            MicroCode::default()
        }
    }
}

fn compile_ld(lhs: &Arg, rhs: &Arg) -> MicroCode {
    // Dirty secret: we have to use the ALU to do any (8-bit) register moves.
    let destination = AluOutSelect::from_register(lhs.expect_as_register()).unwrap_or_else(|| {
        panic!(
            "LD can only have an ALU register as a destination: {:?}",
            lhs
        )
    });
    let source = *rhs.expect_as_register();

    dbg!(lhs);
    dbg!(rhs);

    MicroCode {
        // Write the source to the data bus.
        reg_select: source,
        reg_to_data: true,
        // Sample the data bus into the ALU register.
        alu_out_select: destination,
        alu_reg_write_enable: true,
        ..Default::default()
    }
}
