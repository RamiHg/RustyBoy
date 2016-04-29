use memory::Memory;
use alu::*;

use std::num::Wrapping;

pub struct Cpu {
    gprs : [u8; 8],
    flags : FlagRegister,
    pc : u16,
    sp : u16,

    memory : Memory,
}

const REG_A : usize = 0;
const REG_B : usize = 1;
const REG_C : usize = 2;
const REG_D : usize = 3;
const REG_E : usize = 4;
const REG_F : usize = 5;
const REG_H : usize = 6;
const REG_L : usize = 7;
const REG_INVALID: usize = 8;

/* 
 * My references:
 * http://imrannazar.com/Gameboy-Z80-Opcode-Map
 * http://clrhome.org/table/
 * http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
 * http://gameboy.mongenel.com/asmschool.html
*/

// Utility functions in Cpu
impl Cpu {
    /// Combine two 8-bit registers
    fn combine_regs(&self, high : usize, low : usize) -> u16 {
        ((self.gprs[high] as u16) << 8) | (self.gprs[low] as u16)
    }

    fn set_combined_regs(&mut self, high : usize, low : usize, val : u16) {
        self.gprs[low] = (val & 0xFF) as u8;
        self.gprs[high] = (val >> 8) as u8;
    }

    fn peek_8_imm(&self) -> u8 {
        self.memory.read_general_8(self.pc as usize)
    }

    fn peek_i8_imm(&self) -> i8 {
        self.memory.read_general_8(self.pc as usize) as i8
    }

    fn peek_16_imm(&self) -> u16 {
        let byte0 = self.memory.read_general_8(self.pc as usize);
        let byte1 = self.memory.read_general_8(self.pc as usize + 1);
        ((byte1 as u16) << 8) | (byte0 as u16)
    }

    // 8-bit loads

    fn load_8_imm(&mut self, reg : usize) -> i32 {
        self.pc += 1;
        let imm = self.peek_8_imm();
        self.gprs[reg] = imm;
        self.pc += 1;
        return 8;
    }

    fn mov_8(&mut self, dst : usize, src : usize) -> i32 {
        self.gprs[dst] = self.gprs[src];
        self.pc += 1;
        return 4;
    }

    fn mov_8_indirect(&mut self, dst : usize, src_high : usize, src_low : usize) -> i32 {
        let src_addr = self.combine_regs(src_high, src_low);
        let mem_value = self.memory.read_general_8(src_addr as usize);
        self.gprs[dst] = mem_value;
        self.pc += 1;
        return 8;
    }

    fn mov_8_indirect_imm(&mut self, dst : usize) -> i32 {
        self.pc += 1;
        let mem_location = self.peek_16_imm();
        self.pc += 2;
        let value = self.memory.read_general_8(mem_location as usize);
        self.gprs[dst] = value;
        return 16;
    }

    /// LD dst, ($offset + src)
    fn mov_8_offseted_indirect(&mut self, dst : usize, offset : u16, src : usize) -> i32 {
        let mem_location = self.gprs[src] as u16 + offset;
        let value = self.memory.read_general_8(mem_location as usize);
        self.gprs[dst] = value;
        self.pc += 1;
        return 8;
    }

    // LD dst, ($offset + imm_offset)
    fn mov_8_offseted_imm(&mut self, dst : usize, offset : u16) -> i32 {
        self.pc += 1;
        let imm_offset = self.peek_8_imm();
        let mem_location = offset + imm_offset as u16;
        let value = self.memory.read_general_8(mem_location as usize);
        self.gprs[dst] = value;
        self.pc += 1;
        return 12;
    }

    // TODO: Super hacky to return hl. refactor
    fn mov_8_a_hl(&mut self) -> u16 {
        let hl : u16 = self.combine_regs(REG_H, REG_L);
        let value = self.memory.read_general_8(hl as usize);
        self.gprs[REG_A] = value;
        return hl;
    }

    // LD A, (HLD)
    fn mov_8_a_dec_hl(&mut self) -> i32 {
        let hl = self.mov_8_a_hl();
        // Now, decrement HL (making sure to go around runtime checks)
        self.set_combined_regs(REG_H, REG_L, (Wrapping(hl) - Wrapping(1_u16)).0);
        self.pc += 1;
        return 8;
    }

    // LD A, (HLI)
    fn mov_8_a_inc_hl(&mut self) -> i32 {
        let hl = self.mov_8_a_hl();
        // Increment HL
        self.set_combined_regs(REG_H, REG_L, (Wrapping(hl) + Wrapping(1_u16)).0);
        self.pc += 1;
        return 8;
    }

    fn store_8(&mut self, dst_high : usize, dst_low : usize, src : usize) -> i32 {
        let dst_addr = self.combine_regs(dst_high, dst_low);
        self.memory.store_general_8(dst_addr as usize, self.gprs[src]);
        self.pc += 1;
        return 8;
    }

    fn store_8_imm(&mut self, dst_high : usize, dst_low : usize) -> i32 {
        let dst_addr = self.combine_regs(dst_high, dst_low);
        self.pc += 1;
        let imm = self.peek_8_imm();
        self.memory.store_general_8(dst_addr as usize, imm);
        self.pc += 1;
        return 12;
    }

    fn store_8_immdst(&mut self, src : usize) -> i32 {
        self.pc += 1;
        let mem_location = self.peek_16_imm();
        self.pc += 2;
        self.memory.store_general_8(mem_location as usize, self.gprs[src]);
        return 16;
    }

    // LD ($offset + dst), src
    fn store_8_offseted(&mut self, offset : u16, dst : usize, src : usize) -> i32 {
        let mem_location = self.gprs[dst] as u16 + offset;
        self.memory.store_general_8(mem_location as usize, self.gprs[src]);
        self.pc += 1;
        return 8;
    }

    // LD ($offset + imm_offset), src
    fn store_8_offseted_imm(&mut self, offset : u16, src : usize) -> i32 {
        self.pc += 1;
        let imm_offset = self.peek_8_imm();
        let mem_location = offset + imm_offset as u16;
        self.memory.store_general_8(mem_location as usize, self.gprs[src]);
        self.pc += 1;
        return 12;
    }

    // LD (HLD), A
    fn store_8_a_dec_hl(&mut self) -> i32 {
        let hl : u16 = self.combine_regs(REG_H, REG_L);
        self.memory.store_general_8(hl as usize, self.gprs[REG_A]);
        self.pc += 1;

        // Decrement HL
        self.set_combined_regs(REG_H, REG_L, (Wrapping(hl) - Wrapping(1_u16)).0);
        return 8;
    }

    // LD (HLI), A
    fn store_8_a_inc_hl(&mut self) -> i32 {
        let hl : u16 = self.combine_regs(REG_H, REG_L);
        self.memory.store_general_8(hl as usize, self.gprs[REG_A]);
        self.pc += 1;

        // Increment HL
        self.set_combined_regs(REG_H, REG_L, (Wrapping(hl) + Wrapping(1_u16)).0);
        return 8;
    }

    // 16-bit memory ops

    fn mov_16_imm(&mut self, high: usize, low: usize) -> i32 {
        self.pc += 1;
        let value = self.peek_16_imm();
        self.set_combined_regs(high, low, value);
        self.pc += 2;
        return 12;
    }

    fn mov_16_imm_sp(&mut self) -> i32 {
        self.pc += 1;
        let value = self.peek_16_imm();
        self.sp = value;
        self.pc += 2;
        return 12;
    }

    fn mov_hl_to_sp(&mut self) -> i32 {
        let value = self.combine_regs(REG_H, REG_L);
        self.sp = value;
        self.pc += 1;
        return 8;
    }

    fn mov_spn_to_hl(&mut self) -> i32 {
        self.pc += 1;
        let offset = self.peek_i8_imm();
        let (sp, flags) = add_u16_i8(self.sp, offset);
        self.sp = sp;
        self.flags = flags;
        self.pc += 1;
        return 12;
    }

    fn mov_sp_to_nn(&mut self) -> i32 {
        self.pc += 1;
        let address = self.peek_16_imm();
        self.pc += 2;
        self.memory.store_general_16(address as usize, self.sp);
        return 20;
    }

    fn push_16_reg(&mut self, high: usize, low: usize) -> i32 {
        let value = self.combine_regs(high, low);
        self.memory.store_general_16(self.sp as usize, value);
        self.sp -= 2;
        self.pc += 1;
        return 16;
    }

    fn pop_16_reg(&mut self, high: usize, low: usize) -> i32 {
        let value = self.memory.read_general_16(self.sp as usize);
        self.set_combined_regs(high, low, value);
        self.sp += 2;
        self.pc += 1;
        return 12;
    }

    // 8-bit alu
    fn add_8_reg_reg(&mut self, dst: usize, src: usize) -> i32 {
        let (result, flags) = add_u8_u8(self.gprs[dst], self.gprs[src]);
        self.pc += 1;
        self.flags = flags;
        self.gprs[dst] = result;
        return 4;
    }

    fn add_hl_to_a(&mut self) -> i32 {
        let value = self.memory.read_general_8(self.combine_regs(REG_H, REG_L) as usize);
        let (result, flags) = add_u8_u8(self.gprs[REG_A], value);
        self.pc += 1;
        self.flags = flags;
        self.gprs[REG_A] = result;
        return 8;
    }

    fn add_imm_8_to_a(&mut self) -> i32 {
        self.pc += 1;
        let value = self.peek_8_imm();
        self.pc += 1;
        let (result, flags) = add_u8_u8(self.gprs[REG_A], value);
        self.flags = flags;
        self.gprs[REG_A] = result;
        return 8;
    }

    // ADC A, reg
    fn adc_reg_to_a(&mut self, reg: usize) -> i32 {
        let (result, flags) = adc_u8_u8(self.gprs[REG_A], self.gprs[reg], 
            self.flags.get_bit(FlagBits::CARRY));

        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 4;
    }

    // ADC A, (HL)
    fn adc_hl_to_a(&mut self) -> i32 {
        let value = self.memory.read_general_8(self.combine_regs(REG_H, REG_L) as usize);
        let (result, flags) = adc_u8_u8(self.gprs[REG_A], value, 
            self.flags.get_bit(FlagBits::CARRY));
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }

    // ADC A, n
    fn adc_imm_8_to_a(&mut self) -> i32 {
        self.pc += 1;
        let value = self.peek_8_imm();
        self.pc += 1;
        let (result, flags) = adc_u8_u8(self.gprs[REG_A], value,
            self.flags.get_bit(FlagBits::CARRY));
        self.flags = flags;
        self.gprs[REG_A] = result;
        return 8;
    }

    // SUB reg
    fn sub_reg(&mut self, reg: usize) -> i32 {
        let (result, flags) = sub_i8_i8(self.gprs[REG_A], self.gprs[reg]);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 4;
    }

    // SUB (HL)
    fn sub_hl(&mut self) -> i32 {
        let value = self.memory.read_general_8(self.combine_regs(REG_H, REG_L) as usize);
        let (result, flags) = sub_i8_i8(self.gprs[REG_A], value);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }

    // SUB n
    fn sub_imm_8(&mut self) -> i32 {
        self.pc += 1;
        let value = self.peek_8_imm();
        let (result, flags) = sub_i8_i8(self.gprs[REG_A], value);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }

    // SBC A, reg
    fn sbc_reg(&mut self, reg: usize) -> i32 {
        let (result, flags) = sbc_i8_i8(self.gprs[REG_A], self.gprs[reg],
            self.flags.get_bit(FlagBits::CARRY));
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 4;
    }

    // SBC A, (HL)
    fn sbc_hl(&mut self) -> i32 {
        let value = self.memory.read_general_8(self.combine_regs(REG_H, REG_L) as usize);
        let (result, flags) = sbc_i8_i8(self.gprs[REG_A], value,
            self.flags.get_bit(FlagBits::CARRY));
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }

    // SBC A, n
    fn sbc_imm_8(&mut self) -> i32 {
        self.pc += 1;
        let value = self.peek_8_imm();
        let (result, flags) = sbc_i8_i8(self.gprs[REG_A], value,
            self.flags.get_bit(FlagBits::CARRY));
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }

    // General helper functions that get called by ADC/SBC/INC/DEC (TODO: Actually use for ADC/SBC)

    fn op_u8_flag_helper_reg<F>(&mut self, reg: usize, result_reg: usize, op: F) -> i32
        where F : Fn(u8, u8, &FlagRegister) -> (u8, FlagRegister) {

        let (result, flags) = op(self.gprs[REG_A], self.gprs[reg], &self.flags);
        self.flags = flags;
        self.gprs[result_reg] = result;
        self.pc += 1;
        return 4;
    }

    fn op_u8_flag_helper_hl<F>(&mut self, result_reg: usize, write_to_hl: bool, op: F) -> i32
        where F : Fn(u8, u8, &FlagRegister) -> (u8, FlagRegister) {

        let hl_value = self.combine_regs(REG_H, REG_L) as usize;
        let value = self.memory.read_general_8(hl_value);
        let (result, flags) = op(self.gprs[REG_A], value, &self.flags);
        self.flags = flags;
        if write_to_hl {
            self.memory.store_general_8(hl_value, result);
        }
        else {
            self.gprs[result_reg] = result;
        }
        self.pc += 1;
        return 12; // WARNING: May or may not match actual instruction cycle count
    }
    
    // General helper function that gets called by 
    // AND, OR, XOR
    
    // OP reg
    fn op_u8_helper_reg<F>(&mut self, reg: usize, op: F) -> i32
        where F : Fn(u8, u8) -> (u8, FlagRegister) {
            
        let (result, flags) = op(self.gprs[REG_A], self.gprs[reg]);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 4;
    }
    
    // OP (HL)
    fn op_u8_helper_hl<F>(&mut self, op: F) -> i32 
        where F : Fn(u8, u8) -> (u8, FlagRegister) {
          
        let value = self.memory.read_general_8(self.combine_regs(REG_H, REG_L) as usize);
        let (result, flags) = op(self.gprs[REG_A], value);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;
    }
    
    // OP n
    fn op_u8_helper_imm_8<F>(&mut self, op: F) -> i32 
        where F : Fn(u8, u8) -> (u8, FlagRegister) {
        
        self.pc += 1;
        let value = self.peek_8_imm();
        let (result, flags) = op(self.gprs[REG_A], value);
        self.flags = flags;
        self.gprs[REG_A] = result;
        self.pc += 1;
        return 8;  
    }
    
    // AND reg
    fn and_reg(&mut self, reg: usize) -> i32 {
        self.op_u8_helper_reg(reg, and_u8_u8)
    }
    
    // AND (HL)
    fn and_hl(&mut self) -> i32 {
        self.op_u8_helper_hl(and_u8_u8)
    }
    
    // AND n
    fn and_imm_8(&mut self) -> i32 {
        self.op_u8_helper_imm_8(and_u8_u8)
    }
    
    // OR reg
    fn or_reg(&mut self, reg: usize) -> i32 {
        self.op_u8_helper_reg(reg, or_u8_u8)
    }
    
    // OR (HL)
    fn or_hl(&mut self) -> i32 {
        self.op_u8_helper_hl(or_u8_u8)
    }
    
    // OR n
    fn or_imm_8(&mut self) -> i32 {
        self.op_u8_helper_imm_8(or_u8_u8)
    }
    
    // XOR reg
    fn xor_reg(&mut self, reg: usize) -> i32 {
        self.op_u8_helper_reg(reg, xor_u8_u8)
    }
    
    // XOR (HL)
    fn xor_hl(&mut self) -> i32 {
        self.op_u8_helper_hl(xor_u8_u8)
    }
    
    // XOR n
    fn xor_imm_8(&mut self) -> i32 {
        self.op_u8_helper_imm_8(xor_u8_u8)
    }

    // CP reg
    fn cp_reg(&mut self, reg: usize) -> i32 { self.op_u8_helper_reg(reg, cp_u8_u8) }
    // CP (HL)
    fn cp_hl(&mut self) -> i32 { self.op_u8_helper_hl(cp_u8_u8) }
    // CP n
    fn cp_imm_8(&mut self) -> i32 { self.op_u8_helper_imm_8(cp_u8_u8) }

    // INC reg
    fn inc_reg(&mut self, reg: usize) -> i32 { self.op_u8_flag_helper_reg(reg, reg, inc_u8_u8) }
    // INC (HL)
    fn inc_hl(&mut self) -> i32 { self.op_u8_flag_helper_hl(REG_INVALID, true, inc_u8_u8) }

    // DEC reg
    fn dec_reg(&mut self, reg: usize) -> i32 { self.op_u8_flag_helper_reg(reg, reg, dec_u8_u8) }
    // DEC (HL)
    fn dec_hl(&mut self) -> i32 { self.op_u8_flag_helper_hl(REG_INVALID, true, dec_u8_u8) }

}

impl Cpu {
    /// Executes an instruction op-code.
    /// 
    /// The PC will be incremented to the expected location
    /// after the command is executed.
    /// Returns the number of cycles spent for the instruction
    fn execute_instruction(&mut self, opcode : u8) -> i32 {
        let ret = match opcode {
            // 8-bit immediate load
            0x3E => self.load_8_imm(REG_A),
            0x06 => self.load_8_imm(REG_B),
            0x0E => self.load_8_imm(REG_C),
            0x16 => self.load_8_imm(REG_D),
            0x1E => self.load_8_imm(REG_E),
            0x26 => self.load_8_imm(REG_H),
            0x2E => self.load_8_imm(REG_L),

            // 8-bit register direct/indirect/immediate move
            0x7F => self.mov_8(REG_A, REG_A),
            0x78 => self.mov_8(REG_A, REG_B),
            0x79 => self.mov_8(REG_A, REG_C),
            0x7A => self.mov_8(REG_A, REG_D),
            0x7B => self.mov_8(REG_A, REG_E),
            0x7C => self.mov_8(REG_A, REG_H),
            0x7D => self.mov_8(REG_A, REG_L),
            0x7E => self.mov_8_indirect(REG_A, REG_H, REG_L),
            0x0A => self.mov_8_indirect(REG_A, REG_B, REG_C),
            0x1A => self.mov_8_indirect(REG_A, REG_D, REG_E),
            0xFA => self.mov_8_indirect_imm(REG_A),

            0x47 => self.mov_8(REG_B, REG_A),
            0x40 => self.mov_8(REG_B, REG_B),
            0x41 => self.mov_8(REG_B, REG_C),
            0x42 => self.mov_8(REG_B, REG_D),
            0x43 => self.mov_8(REG_B, REG_E),
            0x44 => self.mov_8(REG_B, REG_H),
            0x45 => self.mov_8(REG_B, REG_L),
            0x46 => self.mov_8_indirect(REG_B, REG_H, REG_L),

            0x4F => self.mov_8(REG_C, REG_A),
            0x48 => self.mov_8(REG_C, REG_B),
            0x49 => self.mov_8(REG_C, REG_C),
            0x4A => self.mov_8(REG_C, REG_D),
            0x4B => self.mov_8(REG_C, REG_E),
            0x4C => self.mov_8(REG_C, REG_H),
            0x4D => self.mov_8(REG_C, REG_L),
            0x4E => self.mov_8_indirect(REG_C, REG_H, REG_L),

            0x57 => self.mov_8(REG_D, REG_A),
            0x50 => self.mov_8(REG_D, REG_B),
            0x51 => self.mov_8(REG_D, REG_C),
            0x52 => self.mov_8(REG_D, REG_D),
            0x53 => self.mov_8(REG_D, REG_E),
            0x54 => self.mov_8(REG_D, REG_H),
            0x55 => self.mov_8(REG_D, REG_L),
            0x56 => self.mov_8_indirect(REG_D, REG_H, REG_L),

            0x5F => self.mov_8(REG_E, REG_A),
            0x58 => self.mov_8(REG_E, REG_B),
            0x59 => self.mov_8(REG_E, REG_C),
            0x5A => self.mov_8(REG_E, REG_D),
            0x5B => self.mov_8(REG_E, REG_E),
            0x5C => self.mov_8(REG_E, REG_H),
            0x5D => self.mov_8(REG_E, REG_L),
            0x5E => self.mov_8_indirect(REG_E, REG_H, REG_L),

            0x67 => self.mov_8(REG_H, REG_A),
            0x60 => self.mov_8(REG_H, REG_B),
            0x61 => self.mov_8(REG_H, REG_C),
            0x62 => self.mov_8(REG_E, REG_D),
            0x63 => self.mov_8(REG_H, REG_E),
            0x64 => self.mov_8(REG_H, REG_H),
            0x65 => self.mov_8(REG_H, REG_L),
            0x66 => self.mov_8_indirect(REG_H, REG_H, REG_L),

            0x6F => self.mov_8(REG_L, REG_A),
            0x68 => self.mov_8(REG_L, REG_B),
            0x69 => self.mov_8(REG_L, REG_C),
            0x6A => self.mov_8(REG_L, REG_D),
            0x6B => self.mov_8(REG_L, REG_E),
            0x6C => self.mov_8(REG_L, REG_H),
            0x6D => self.mov_8(REG_L, REG_L),
            0x6E => self.mov_8_indirect(REG_L, REG_H, REG_L),

            // 8-bit store into (nn)
            0x77 => self.store_8(REG_H, REG_L, REG_A),
            0x70 => self.store_8(REG_H, REG_L, REG_B),
            0x71 => self.store_8(REG_H, REG_L, REG_C),
            0x72 => self.store_8(REG_H, REG_L, REG_D),
            0x73 => self.store_8(REG_H, REG_L, REG_E),
            0x74 => self.store_8(REG_H, REG_L, REG_H),
            0x75 => self.store_8(REG_H, REG_L, REG_L),

            0x02 => self.store_8(REG_B, REG_C, REG_A),
            0x12 => self.store_8(REG_D, REG_E, REG_A),

            0xEA => self.store_8_immdst(REG_A),

            // TODO: Move me
            0x36 => self.store_8_imm(REG_H, REG_L),

            0xF2 => self.mov_8_offseted_indirect(REG_A, 0xFF00, REG_C),
            0xF0 => self.mov_8_offseted_imm(REG_A, 0xFF00),

            0xE2 => self.store_8_offseted(0xFF00, REG_C, REG_A),

            0xE0 => self.store_8_offseted_imm(0xFF00, REG_A),

            0x3A => self.mov_8_a_dec_hl(),
            0x2A => self.mov_8_a_inc_hl(),
            0x32 => self.store_8_a_dec_hl(),
            0x22 => self.store_8_a_inc_hl(),

            // 16-bit memory operations
            0x01 => self.mov_16_imm(REG_B, REG_C),
            0x11 => self.mov_16_imm(REG_D, REG_E),
            0x21 => self.mov_16_imm(REG_H, REG_L),
            0x31 => self.mov_16_imm_sp(),

            0xF9 => self.mov_hl_to_sp(),
            0xF8 => self.mov_spn_to_hl(),
            0x08 => self.mov_sp_to_nn(),

            0xF5 => self.push_16_reg(REG_A, REG_F),
            0xC5 => self.push_16_reg(REG_B, REG_C),
            0xD5 => self.push_16_reg(REG_D, REG_E),
            0xE5 => self.push_16_reg(REG_H, REG_L),

            0xF1 => self.pop_16_reg(REG_A, REG_F),
            0xC1 => self.pop_16_reg(REG_B, REG_C),
            0xD1 => self.pop_16_reg(REG_D, REG_E),
            0xE1 => self.pop_16_reg(REG_H, REG_L),

            // 8-bit ALU
            0x87 => self.add_8_reg_reg(REG_A, REG_A),
            0x80 => self.add_8_reg_reg(REG_A, REG_B),
            0x81 => self.add_8_reg_reg(REG_A, REG_C),
            0x82 => self.add_8_reg_reg(REG_A, REG_D),
            0x83 => self.add_8_reg_reg(REG_A, REG_E),
            0x84 => self.add_8_reg_reg(REG_A, REG_H),
            0x85 => self.add_8_reg_reg(REG_A, REG_L),
            0x86 => self.add_hl_to_a(),
            0xC6 => self.add_imm_8_to_a(),

            0x8F => self.adc_reg_to_a(REG_A),
            0x88 => self.adc_reg_to_a(REG_B),
            0x89 => self.adc_reg_to_a(REG_C),
            0x8A => self.adc_reg_to_a(REG_D),
            0x8B => self.adc_reg_to_a(REG_E),
            0x8C => self.adc_reg_to_a(REG_H),
            0x8D => self.adc_reg_to_a(REG_L),
            0x8E => self.adc_hl_to_a(),
            0xCE => self.adc_imm_8_to_a(),

            0x97 => self.sub_reg(REG_A),
            0x90 => self.sub_reg(REG_B),
            0x91 => self.sub_reg(REG_C),
            0x92 => self.sub_reg(REG_D),
            0x93 => self.sub_reg(REG_E),
            0x94 => self.sub_reg(REG_H),
            0x95 => self.sub_reg(REG_L),
            0x96 => self.sub_hl(),
            0xD6 => self.sub_imm_8(),

            0x9F => self.sbc_reg(REG_A),
            0x98 => self.sbc_reg(REG_B),
            0x99 => self.sbc_reg(REG_C),
            0x9A => self.sbc_reg(REG_D),
            0x9B => self.sbc_reg(REG_E),
            0x9C => self.sbc_reg(REG_H),
            0x9D => self.sbc_reg(REG_L),
            0x9E => self.sbc_hl(),
            0xDE => self.sbc_imm_8(),
            
            0xA7 => self.and_reg(REG_A),
            0xA0 => self.and_reg(REG_B),
            0xA1 => self.and_reg(REG_C),
            0xA2 => self.and_reg(REG_D),
            0xA3 => self.and_reg(REG_E),
            0xA4 => self.and_reg(REG_H),
            0xA5 => self.and_reg(REG_L),
            0xA6 => self.and_hl(),
            0xE6 => self.and_imm_8(),
            
            0xB7 => self.or_reg(REG_A),
            0xB0 => self.or_reg(REG_B),
            0xB1 => self.or_reg(REG_C),
            0xB2 => self.or_reg(REG_D),
            0xB3 => self.or_reg(REG_E),
            0xB4 => self.or_reg(REG_H),
            0xB5 => self.or_reg(REG_L),
            0xB6 => self.or_hl(),
            0xF6 => self.or_imm_8(),
            
            0xAF => self.xor_reg(REG_A),
            0xA8 => self.xor_reg(REG_B),
            0xA9 => self.xor_reg(REG_C),
            0xAA => self.xor_reg(REG_D),
            0xAB => self.xor_reg(REG_E),
            0xAC => self.xor_reg(REG_H),
            0xAD => self.xor_reg(REG_L),
            0xAE => self.xor_hl(),
            0xEE => self.xor_imm_8(),

            0xBF => self.cp_reg(REG_A),
            0xB8 => self.cp_reg(REG_B),
            0xB9 => self.cp_reg(REG_C),
            0xBA => self.cp_reg(REG_D),
            0xBB => self.cp_reg(REG_E),
            0xBC => self.cp_reg(REG_H),
            0xBD => self.cp_reg(REG_L),
            0xBE => self.cp_hl(),
            0xFE => self.cp_imm_8(),

            0x3C => self.inc_reg(REG_A),
            0x04 => self.inc_reg(REG_B),
            0x0C => self.inc_reg(REG_C),
            0x14 => self.inc_reg(REG_D),
            0x1C => self.inc_reg(REG_E),
            0x24 => self.inc_reg(REG_H),
            0x2C => self.inc_reg(REG_L),
            0x34 => self.inc_hl(),

            0x3D => self.dec_reg(REG_A),
            0x05 => self.dec_reg(REG_B),
            0x0D => self.dec_reg(REG_C),
            0x15 => self.dec_reg(REG_D),
            0x1D => self.dec_reg(REG_E),
            0x25 => self.dec_reg(REG_H),
            0x2D => self.dec_reg(REG_L),
            0x35 => self.dec_hl(),


            _    => panic!("Oops"),
        };

        ret
    }
}
