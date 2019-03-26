use crate::cpu::register::Register;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IncOp {
    Mov = 0b00,
    Inc = 0b01,
    Dec = 0b10,
}

impl Default for IncOp {
    fn default() -> Self {
        IncOp::Mov
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AluOp {
    Mov,
    Add,
}

impl Default for AluOp {
    fn default() -> Self {
        AluOp::Mov
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AluOutSelect {
    Result,
    Tmp,
    A,
    ACT,
    F,
}

impl Default for AluOutSelect {
    fn default() -> Self {
        AluOutSelect::Result
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct MicroCode {
    // These two flags control the RD and WR signal registers on the memory bus. Alone, they do not
    //  do much other than signal to the memory controller intent.
    pub mem_read_enable: bool,
    pub mem_write_enable: bool,

    // Register control.
    pub reg_select: Register,
    pub reg_write_enable: bool,
    pub reg_to_data: bool,

    // Address control.
    /// If true, will drive the address bus from the register file, and more importantly, write
    /// into the address buffer register.
    pub reg_to_addr_buffer: bool,
    pub addr_select: Register,
    /// If true, will overwrite the selected address register with the value in the address bus.
    pub addr_write_enable: bool,

    // Incrementer control.
    pub inc_op: IncOp,
    /// Drives the address bus with the result of the incrementer.
    pub inc_to_addr_bus: bool,

    // Alu control.
    pub alu_op: AluOp,
    pub alu_out_select: AluOutSelect,
    pub alu_to_data: bool,
    /// Overwrites the selected ALU register with the value in the data bus.
    pub alu_reg_write_enable: bool,
    pub alu_write_f_mask: u8,

    // Control flow.
    pub is_end: bool,
    pub is_cond_end: bool,
}
