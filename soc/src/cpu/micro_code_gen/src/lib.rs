use arraydeque::ArrayDeque;

use micro_code::micro_code::MicroCode;

pub const MAX_MICROCODES_PER_OP: usize = 22;
pub type MicroCodeList = ArrayDeque<[MicroCode; MAX_MICROCODES_PER_OP]>;

pub struct Pla {
    microcode_array: Vec<MicroCode>,
    pla: Vec<Vec<u16>>,
}

impl Default for Pla {
    fn default() -> Self {
        let microcode_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/microcode_array.bin"));
        let microcode_array = unsafe {
            std::slice::from_raw_parts(
                microcode_bytes.as_ptr() as *const MicroCode,
                microcode_bytes.len() / std::mem::size_of::<MicroCode>(),
            )
        }
        .to_vec();
        let pla_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/pla.bin"))
            .chunks_exact(2)
            .map(|b| u16::from_ne_bytes([b[0], b[1]]))
            .collect::<Vec<u16>>();
        let mut pla = Vec::new();
        let mut i = 0;
        while i < pla_bytes.len() {
            let num_indices = usize::from(pla_bytes[i]);
            assert!(
                num_indices <= MAX_MICROCODES_PER_OP,
                "Got {} microcodes. Max is {}",
                num_indices,
                MAX_MICROCODES_PER_OP
            );
            pla.push(Vec::from(&pla_bytes[i + 1..i + 1 + num_indices]));
            i += 1 + num_indices;
        }
        assert_eq!(i, pla_bytes.len());
        assert_eq!(pla.len(), 512 + 1);
        Pla { microcode_array, pla }
    }
}

impl Pla {
    pub fn microcodes_for(&self, opcode: u8, cb_mode: bool) -> MicroCodeList {
        let real_opcode = usize::from(opcode) + if cb_mode { 256 } else { 0 };
        self.opcodes_at(real_opcode)
    }

    pub fn interrupt_handler(&self) -> MicroCodeList {
        self.opcodes_at(self.pla.len() - 1)
    }

    fn opcodes_at(&self, idx: usize) -> MicroCodeList {
        self.pla[idx]
            .iter()
            .map(|idx| self.microcode_array[usize::from(*idx)])
            .collect::<MicroCodeList>()
    }
}
