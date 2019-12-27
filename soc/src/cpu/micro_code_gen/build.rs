use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use micro_code::micro_code::MicroCode;
use micro_code::pla::DecoderBuilder;

fn main() {
    let decode_builder = DecoderBuilder::default();

    let mut microcode_map = HashMap::<MicroCode, u16>::new();
    let mut pla = Vec::<u16>::new();
    let mut friendly_pla = Vec::new();

    let mut process_opcodes = |codes: Vec<MicroCode>| {
        let mut code_indices = codes
            .iter()
            .map(|opcode| {
                if let Some(&index) = microcode_map.get(opcode) {
                    index
                } else {
                    let index = u16::try_from(microcode_map.len()).unwrap();
                    microcode_map.insert(*opcode, index);
                    index
                }
            })
            .collect::<Vec<u16>>();
        friendly_pla.push(code_indices.clone());
        code_indices.insert(0, u16::try_from(code_indices.len()).unwrap());
        pla.append(&mut code_indices);
    };

    for &cb_mode in &[false, true] {
        for opcode in 0..=255 {
            process_opcodes(decode_builder.decode(opcode, cb_mode));
        }
    }
    process_opcodes(decode_builder.interrupt_handler());
    // TODO: Should probably just sort rather than creating a reverse map..
    let microcode_array = microcode_map
        .into_iter()
        .map(|(k, v)| (v, k))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<MicroCode>>();
    let as_bytes = unsafe {
        std::slice::from_raw_parts(
            microcode_array.as_ptr() as *const u8,
            std::mem::size_of::<MicroCode>() * microcode_array.len(),
        )
    }
    .to_vec();
    
    /*
    let mut mc_file = File::create("/tmp/ramy/mc_array.svh").unwrap();
    for (i, code) in microcode_array.iter().enumerate() {
        write!(mc_file, "{}",
            format!("{:#?}", code).replace("MicroCode {", "'{ ").replace(",\n}", "\n}")).unwrap();
        if i != microcode_array.len() - 1{
            write!(mc_file, ",\n").unwrap();
        }
    }

    mc_file = File::create("/tmp/ramy/opcode_ids.mem").unwrap();
    let mut i = 0;
    for (idx, ids) in friendly_pla.iter().enumerate() {
        write!(mc_file, "{}", i).unwrap();
        if idx != friendly_pla.len() - 1 {
            write!(mc_file, ",");
        }
        write!(mc_file, "\n");
        i += ids.len();
    }

    mc_file = File::create("/tmp/ramy/opcode_indices.mem").unwrap();
    for (idx0, ids) in friendly_pla.iter().enumerate() {
        for (idx,i) in ids.iter().enumerate() {
            write!(mc_file, "{}", i).unwrap();
            if idx0 != friendly_pla.len() - 1 || idx != ids.len() - 1 {
                write!(mc_file, ",");
            }
            write!(mc_file, "\n");
        }
    }

    // cp /tmp/mc_array.svh ~/repos/GitHub/RamyBoy/src/cpu/mc_array.svh

    */

    assert_eq!(std::mem::size_of::<micro_code::register::Register>(), 1);
    assert_eq!(std::mem::size_of::<MicroCode>(), 33);
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut f = File::create(Path::new(&out_dir).join("microcode_array.bin")).unwrap();
    f.write_all(&as_bytes).unwrap();
    let mut f = File::create(Path::new(&out_dir).join("pla.bin")).unwrap();
    f.write_all(unsafe { std::slice::from_raw_parts(pla.as_ptr() as *const u8, 2 * pla.len()) })
        .unwrap();
}
