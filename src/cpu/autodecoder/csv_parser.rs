use std::collections::HashMap;

use super::{asm, asm_parser::parse_op};
use crate::cpu::register::Register;

/// Uncompiled micro-code ops as parsed from the CSV. While in this high-level form, can support
/// operations like operand remapping.
#[derive(Debug)]
pub struct SourceOpList(Vec<asm::Op>);

type MCycleList = Vec<MCycle>;

#[derive(Debug)]
pub struct MCycle {
    pub t1: SourceOpList,
    pub t2: SourceOpList,
    pub t3: SourceOpList,
    pub t4: SourceOpList,
}

fn extract_tcycle(i: usize, num_ops: usize, record: &csv::StringRecord) -> SourceOpList {
    let mut result = Vec::new();
    for op_index in 0..num_ops {
        let value = record.get(i + op_index).unwrap();
        if !value.is_empty() {
            result.push(parse_op(value));
        }
    }
    SourceOpList(result)
}

fn extract_mcycle(mcycle: usize, record: &csv::StringRecord) -> MCycle {
    // Skip leftmost 4 columns.
    let skip = 4;
    if mcycle == 0 {
        MCycle {
            t1: SourceOpList(Vec::new()),
            t2: SourceOpList(Vec::new()),
            t3: extract_tcycle(skip, 4, record),
            t4: extract_tcycle(skip + 4, 4, record),
        }
    } else {
        let i = skip + 8 + (mcycle - 1) * 12;
        MCycle {
            t1: extract_tcycle(i, 3, record),
            t2: extract_tcycle(i + 3, 1, record),
            t3: extract_tcycle(i + 3 + 1, 4, record),
            t4: extract_tcycle(i + 3 + 1 + 4, 4, record),
        }
    }
}

pub fn parse_csv(path: &str) -> HashMap<String, MCycleList> {
    let mut rdr = csv::Reader::from_path(path).unwrap();
    let mut code_map = HashMap::new();
    // Ignore the first two lines.
    for result in rdr.records().skip(2) {
        let record: csv::StringRecord = result.unwrap();
        let name = &record[0];
        // Go through all the possible mcycles (6 max).
        let mut mcycles = Vec::new();
        for i in 0..=5 {
            let mcycle = extract_mcycle(i, &record);
            if !mcycle.t1.0.is_empty()
                || !mcycle.t2.0.is_empty()
                || !mcycle.t3.0.is_empty()
                || !mcycle.t4.0.is_empty()
            {
                mcycles.push(mcycle);
            } else {
                break;
            }
        }
        code_map.insert(name.replace(" ", "").to_string(), mcycles);
    }
    code_map
}

use asm::MaybeArg;
use asm::Op;

impl SourceOpList {
    fn remap_arg(arg: &asm::MaybeArg, from: &asm::Arg, to: &asm::Arg) -> asm::MaybeArg {
        if let Some(from) = &arg.0 {
            asm::MaybeArg(Some(to.clone()))
        } else {
            arg.clone()
        }
    }

    fn remap_op<Extractor, Zipper>(
        &self,
        from: &asm::Arg,
        with: &asm::Arg,
        extract: Extractor,
        zipper: Zipper,
    ) -> SourceOpList
    where
        Extractor: Fn(&Op) -> &MaybeArg,
        Zipper: Fn((MaybeArg, &Op)) -> Op,
    {
        SourceOpList(
            self.0
                .iter()
                .map(extract)
                .map(|arg| SourceOpList::remap_arg(arg, from, with))
                .zip(self.0.iter())
                .map(zipper)
                .collect(),
        )
    }

    pub fn remap_lhs_reg(&self, with: Register) -> SourceOpList {
        self.remap_op(
            &asm::Arg::Lhs,
            &asm::Arg::Register(with),
            |op| &op.lhs,
            |(arg, op)| Op {
                lhs: arg,
                ..op.clone()
            },
        )
    }

    pub fn remap_rhs(&self, with: Register) -> SourceOpList {
        self.remap_op(
            &asm::Arg::Rhs,
            &asm::Arg::Register(with),
            |op| &op.rhs,
            |(arg, op)| Op {
                rhs: arg,
                ..op.clone()
            },
        )
    }
}
