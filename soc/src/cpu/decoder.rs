use micro_code_gen::{MicroCodeList, Pla};

#[derive(Default)]
pub struct Decoder {
    pla: Pla,
}

impl Decoder {
    pub fn decode(&self, op: i32, in_cb_mode: bool) -> MicroCodeList {
        use std::convert::TryFrom;
        self.pla.microcodes_for(u8::try_from(op).unwrap(), in_cb_mode)
    }

    pub fn interrupt_handler(&self) -> MicroCodeList {
        self.pla.interrupt_handler()
    }
}
