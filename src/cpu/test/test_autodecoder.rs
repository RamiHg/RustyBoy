use crate::cpu::autodecoder::asm::*;
use crate::cpu::autodecoder::csv_parser::parse_csv;

use crate::cpu::autodecoder::asm_compiler::compile_op;

#[test]
fn test_parse_csv() { parse_csv(r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv"); }

fn compile_op_list(op_list: &Option<Vec<Op>>) {
    if op_list.is_some() {
        for op in op_list.as_ref().unwrap() {
            compile_op(op);
        }
    }
}

#[test]
fn test_compile_csv() {
    let rules = parse_csv(r"/Users/Ramy/Downloads/CPU Design - Instruction Breakdown.csv");
    for (rule, mcycles) in rules.iter() {
        for mcycle in mcycles {
            compile_op_list(&mcycle.t1);
            compile_op_list(&mcycle.t2);
            compile_op_list(&mcycle.t3);
            compile_op_list(&mcycle.t4);
        }
    }
}
