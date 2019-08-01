use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[macro_use]
extern crate proc_quote;

use micro_code::pla::DecoderBuilder;

fn codegen(opcodes: impl Iterator<Item = String>) -> String {
    let the_code = quote! {
        #(
            let (opcode, microcodes) = #opcodes;
        )*
    }
    .to_string();
    unescape::unescape(&the_code).unwrap()
}

fn main() {
    let decode_builder = DecoderBuilder::new();

    let mut opcode_map = HashMap::<i32, _>::new();
    let nop = decode_builder.decode(0, false);

    for opcode in 1..=1 {
        let code = decode_builder.decode(opcode, false);
        // Serialize!
        //let serialized = code.iter().map(|x| format!("{:#?}", x)).collect::<Vec<String>>();
        if code != nop {
            opcode_map.insert(opcode, code); //format!("{:#?}", code));
        }
    }

    println!("{}", codegen(opcode_map.iter().map(|x| format!("{:#?}", x))));
}
