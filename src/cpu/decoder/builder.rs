use crate::cpu;

use cpu::alu;
use cpu::micro_code::*;
use cpu::register::Register;

pub struct Builder {
    current_code: MicroCode,
    codes: Vec<MicroCode>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            current_code: MicroCode::new(),
            codes: Vec::new(),
        }
    }

    pub fn nothing_then(mut self) -> Builder {
        self.current_code = MicroCode::new();
        self.then()
    }

    pub fn then(mut self) -> Builder {
        self.codes.push(self.current_code);
        self.current_code = MicroCode::new();
        self
    }

    pub fn then_done(mut self) -> Vec<MicroCode> {
        self.current_code.is_done = true;
        self.codes.push(self.current_code);
        self.codes
    }

    // ALU.
    fn unconditional_alu(mut self, op: AluOp) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_none());
        self.current_code.alu_stage = Some(AluStage {
            op,
            flag_condition: None,
        });
        self
    }

    pub fn alu_move(self, destination: Register, source: Register) -> Builder {
        self.unconditional_alu(AluOp::Move {
            destination,
            source,
        })
    }

    pub fn on_condition(mut self, test_flags: alu::Flags, test_is_set: bool) -> Builder {
        debug_assert!(self.current_code.alu_stage.is_some());
        self.current_code.alu_stage = Some(AluStage {
            op: self.current_code.alu_stage.unwrap().op,
            flag_condition: Some(FlagCondition {
                test_flags,
                test_is_set,
            }),
        });
        self
    }

    pub fn binary_op(self, op: alu::BinaryOp, lhs: Register, rhs: Register) -> Builder {
        self.unconditional_alu(AluOp::BinaryOp { op, lhs, rhs })
    }

    pub fn unary_op(self, op: alu::UnaryOp, register: Register) -> Builder {
        self.unconditional_alu(AluOp::UnaryOp { op, register })
    }

    // (Pre-ALU) Register control.
    pub fn move_reg(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::Move(destination, source));
        self
    }

    pub fn set_reg(mut self, register: Register, value: i32) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::Set(register, value));
        self
    }

    pub fn pre_alu_sign_extend(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.register_control_stage = Some(RegisterControl::SignExtend {
            destination,
            source,
        });
        self
    }

    pub fn post_alu_register_control(mut self, control: RegisterControl) -> Builder {
        debug_assert!(self.current_code.register_control_stage.is_none());
        self.current_code.post_alu_control_stage = Some(control);
        self
    }

    // (Post-ALU) Register control.
    pub fn post_alu_move_reg(mut self, destination: Register, source: Register) -> Builder {
        debug_assert!(self.current_code.post_alu_control_stage.is_none());
        self.current_code.post_alu_control_stage = Some(RegisterControl::Move(destination, source));
        self
    }

    // Memory.

    pub fn read_mem(mut self, destination: Register, address: Register) -> Builder {
        debug_assert!(self.current_code.memory_stage.is_none());
        self.current_code.memory_stage = Some(MemoryStage::Read {
            destination,
            address,
        });
        self
    }

    pub fn write_mem(mut self, address: Register, value: Register) -> Builder {
        debug_assert!(self.current_code.memory_stage.is_none());
        self.current_code.memory_stage = Some(MemoryStage::Write { address, value });
        self
    }

    pub fn maybe_increment(mut self, increment: Option<IncrementerStage>) -> Builder {
        debug_assert!(self.current_code.incrementer_stage.is_none());
        self.current_code.incrementer_stage = increment;
        self
    }

    pub fn increment(self, register: Register) -> Builder {
        self.incrementer_stage(IncrementerStage::Increment(register))
    }

    pub fn decrement(self, register: Register) -> Builder {
        self.incrementer_stage(IncrementerStage::Decrement(register))
    }

    pub fn incrementer_stage(mut self, stage: IncrementerStage) -> Builder {
        debug_assert!(self.current_code.incrementer_stage.is_none());
        self.current_code.incrementer_stage = Some(stage);
        self
    }

    pub fn decode() -> Vec<MicroCode> {
        Builder::new()
            .read_mem(Register::TEMP_LOW, Register::PC)
            .do_decode()
            .increment(Register::PC)
            .then_done()
    }

    // Misc stages.
    pub fn post_alu_restore_flags(mut self, source: Register, mask: alu::Flags) -> Builder {
        debug_assert!(self.current_code.post_alu_control_stage.is_none());
        self.current_code.post_alu_control_stage =
            Some(RegisterControl::RestoreFlags { source, mask });
        self
    }

    pub fn conditional_done(mut self, test_flags: alu::Flags, test_is_set: bool) -> Builder {
        debug_assert!(self.current_code.decoder_stage.is_none());
        self.current_code.decoder_stage = Some(DecoderStage::ConditionalDone(FlagCondition {
            test_flags,
            test_is_set,
        }));
        self
    }

    fn do_decode(mut self) -> Builder {
        self.current_code.decoder_stage = Some(DecoderStage::Decode);
        self
    }
}
