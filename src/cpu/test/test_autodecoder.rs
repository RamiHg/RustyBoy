use crate::cpu::autodecoder::asm::*;
use crate::cpu::autodecoder::csv_parser::parse_csv;

use crate::cpu::autodecoder::asm_compiler::compile_op_list;

#[test]
fn test_parse_csv() { parse_csv(r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv"); }

fn maybe_compile_op_list(op_list: &Option<Vec<Op>>) {
    if let Some(list) = op_list {
        compile_op_list(list);
    }
}

#[test]
fn test_compile_csv() {
    let rules = parse_csv(r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv");
    for (rule, mcycles) in rules.iter() {
        dbg!(rule);
        for mcycle in mcycles {
            maybe_compile_op_list(&mcycle.t1);
            maybe_compile_op_list(&mcycle.t2);
            maybe_compile_op_list(&mcycle.t3);
            maybe_compile_op_list(&mcycle.t4);
        }
    }
}
