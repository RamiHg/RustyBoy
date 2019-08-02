use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use micro_code::micro_code::MicroCode;
use micro_code::pla::DecoderBuilder;

fn main() {
    let decode_builder = DecoderBuilder::new();

    let mut microcode_map = HashMap::<MicroCode, i32>::new();

    let mut pla = Vec::new();

    let mut num_entries = 0;
    for opcode in 0..=255 {
        let code_indices = decode_builder
            .decode(opcode, false)
            .iter()
            .map(|opcode| {
                num_entries += 1;
                if let Some(&index) = microcode_map.get(opcode) {
                    index
                } else {
                    let index = microcode_map.len() as i32;
                    microcode_map.insert(*opcode, index);
                    index
                }
            })
            .collect::<Vec<i32>>();
        pla.push(code_indices);
    }

    let int_to_code = microcode_map
        .into_iter()
        .map(|(k, v)| (v, k))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<MicroCode>>();
    let as_bytes = unsafe {
        std::slice::from_raw_parts(
            int_to_code.as_ptr() as *const u8,
            std::mem::size_of::<MicroCode>() * int_to_code.len(),
        )
    }
    .to_vec();
    println!("Original is {}. New has only {}.", num_entries, int_to_code.len());
    let reinterpret = unsafe {
        std::slice::from_raw_parts(as_bytes.as_ptr() as *const MicroCode, int_to_code.len())
    };
    assert_eq!(reinterpret.to_vec(), int_to_code);
}
