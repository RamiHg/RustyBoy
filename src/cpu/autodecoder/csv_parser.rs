use std::collections::HashMap;

use super::{asm, asm_parser::parse_op};

type OpList = Vec<asm::Op>;
type MCycleList = Vec<MCycle>;

#[derive(Debug)]
pub struct MCycle {
    pub t1: Option<OpList>,
    pub t2: Option<OpList>,
    pub t3: Option<OpList>,
    pub t4: Option<OpList>,
}

fn extract_tcycle(i: usize, num_ops: usize, record: &csv::StringRecord) -> Option<OpList> {
    let mut result = OpList::new();
    for op_index in 0..num_ops {
        let value = record.get(i + op_index).unwrap();
        if !value.is_empty() {
            result.push(parse_op(value));
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn extract_mcycle(mcycle: usize, record: &csv::StringRecord) -> MCycle {
    // Skip leftmost 4 columns.
    let skip = 4;
    if mcycle == 0 {
        MCycle {
            t1: None,
            t2: None,
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
            if mcycle.t1.is_some()
                || mcycle.t2.is_some()
                || mcycle.t3.is_some()
                || mcycle.t4.is_some()
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
