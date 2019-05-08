use super::op_map::{MCycle, MCycleList, MCycleMap, SourceOpList};
use super::parser;

fn extract_tcycle(i: usize, num_ops: usize, record: &csv::StringRecord) -> SourceOpList {
    let mut result = Vec::new();
    for op_index in 0..num_ops {
        let value = record.get(i + op_index).unwrap();
        if !value.is_empty() {
            result.push(parser::parse_op(value));
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
        let i = skip + 8 + (mcycle - 1) * 13;
        MCycle {
            t1: extract_tcycle(i, 4, record),
            t2: extract_tcycle(i + 4, 1, record),
            t3: extract_tcycle(i + 4 + 1, 4, record),
            t4: extract_tcycle(i + 4 + 1 + 4, 4, record),
        }
    }
}

pub fn parse_csv(content: &[u8]) -> MCycleMap {
    let mut rdr = csv::Reader::from_reader(content);
    let mut code_map = MCycleMap::new();
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
        code_map.insert(name.replace(" ", "").to_string(), MCycleList(mcycles));
    }
    code_map
}
