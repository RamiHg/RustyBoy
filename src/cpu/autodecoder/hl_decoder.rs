use super::{loader::*, *};
use crate::cpu::register::Register;

impl MicroCode {
    pub fn from(hl: &HLMicroCode) -> [MicroCode; 4] {
        let mut t1 = MicroCode::default();
        let mut t2 = MicroCode::default();
        let mut t3 = MicroCode::default();
        let mut t4 = MicroCode::default();
        // Handle memory accesses. Memory accesses are synchronized as following
        // (Note: Moore FSM assumed. I.e., assertion takes effect at next rising edge.)
        //   T1: If reading, assert read latch.  Assert address latch.
        //   T2: If writing, assert write and data latches
        //   T3: If reading, sample data bus. If writing, keep write latch asserted.
        //   T4: If writing, deassert write latch. Aassert read latch. (handled by CPU).
        if let Some(mode) = hl.memory_mode {
            t1.mem_set_address = true;
            t1.mem_reg_address = hl.memory_address_source.unwrap().as_address();
            match mode {
                MemoryMode::Read => {
                    t1.mem_read_enable = true; // not actually needed.
                    t3.reg_write_enable = true;
                }
                MemoryMode::Write => {
                    t2.mem_write_enable = true;
                }
            }
        };
        // Select the register that the memory op will read into/write from.
        if let Some(register) = hl.register_select {
            t3.reg_select = register.as_single();
        }
        // Setup the ALU TMP register. This mostly happens in T3.
        match hl.alu_tmp {
            // If the source is a register, select and read it.
            Some(OpSource::Register(register)) => {
                assert!(register.is_single());
                t3.alu_bus_select = register;
                t3.alu_bus_tmp_read = true;
            }
            // If it's memory, sample the memory at T3.
            Some(OpSource::Memory) => {
                assert!(t1.mem_read_enable);
                t3.alu_bus_tmp_read_mem = true;
            }
            None => (),
            _ => panic!("Unexpected ALU TMP: {:?}.", hl.alu_tmp),
        }
        // Setup the ALU ACT register read, and result write. This mostly happens in T3 and T4.
        match hl.alu_rd {
            Some(OpSource::Register(register)) => {
                assert!(register.is_single());
                t3.reg_select = register;
                t3.alu_act_read = true;
                // Write the result.
                t4.reg_select = register;
                t4.alu_write = true;
                t4.reg_write_enable = true;
            }
            None => (),
            _ => panic!("Unexpected ALU RD: {:?}.", hl.alu_rd),
        }
        // Create the T4 cycle, which does ALU.
        if let Some(op) = &hl.alu_op {
            match op {
                &Op::Alu(alu_op) => t4.alu_op = alu_op,
                Op::Addc1 => {
                    t3.alu_bus_f_reset = true;
                    t4.alu_op = AluOp::Addc;
                    t4.alu_f_force_carry = true;
                }
                _ => panic!("OP placeholder not filled."),
            }
        }
        // Control flags.
        t2.alu_bus_select = Register::F;
        t2.alu_bus_f_read = true;
        match hl.alu_tmp_flag {
            Some(TempFlagControl::ReadWrite) => t4.alu_bus_f_write = 0xF,
            Some(TempFlagControl::Write { mask }) => t4.alu_bus_f_write = mask,
            None => (),
        }
        // And incrementing.
        if let Some(incrementer) = hl.incrementer {
            t4.inc_write = true;
            let destination_reg = match incrementer.addr {
                OpSource::Register(reg) => reg,
                _ => panic!("Invalid INC ADDR: {:?}.", incrementer.addr),
            };
            use IncrementOp::*;
            match incrementer.op {
                op @ Inc | op @ Dec => {
                    //   assert_eq!(Some(incrementer.addr), hl.memory_address_source,
                    // "INC/DEC operations must write back to same address used as source. INCADDR:
                    // {:?}. ADDR: {:?}", incrementer.addr,
                    // hl.memory_address_source);
                    assert!(destination_reg.is_pair());
                    t4.inc_op = if op == Inc { IncOp::Inc } else { IncOp::Dec };
                    t4.inc_dest = destination_reg;
                }
                MovPc => {
                    t4.inc_op = IncOp::Mov;
                    t4.inc_dest = Register::PC;
                    t4.inc_skip_latch = true;
                }
            }
        }
        // TODO: Ending
        if let EndMode::Yes = hl.end_mode {
            t4.is_end = true;
        }
        [t1, t2, t3, t4]
    }
}
