use bms_core::bms::bms_to_bocf;
use bms_core::term::term_to_veblen;
use std::path::PathBuf;

fn parse_matrix(input: &str) -> Vec<Vec<i32>> {
    let mut m = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find('(') {
        let after = &rest[start + 1..];
        match after.find(')') {
            None => break,
            Some(end) => {
                let col: Vec<i32> = after[..end]
                    .split(',')
                    .filter_map(|p| p.trim().parse().ok())
                    .collect();
                m.push(col);
                rest = &after[end + 1..];
            }
        }
    }
    m
}

/// Minimal JSON string-value extractor for the two fields we need.
fn extract_fields(data: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = data[pos..].find("\"input\"") {
        let start = pos + rel + 7;
        let Some(eq) = data[start..].find(':') else { break };
        let vs = start + eq + 1;
        let Some(q) = data[vs..].find('"') else { break };
        let s = vs + q + 1;
        let mut end = s;
        let mut val = String::new();
        while end < data.len() {
            let b = data.as_bytes()[end];
            if b == b'\\' {
                val.push(data.as_bytes()[end + 1] as char);
                end += 2;
            } else if b == b'"' {
                end += 1;
                break;
            } else {
                val.push(b as char);
                end += 1;
            }
        }
        let input = val;
        // find the formattedVeblen of the same object (up to the next "input" key)
        let obj_end = data[end..]
            .find("\"input\"")
            .map(|i| end + i)
            .unwrap_or(data.len());
        let chunk = &data[end..obj_end];
        let mut veblen = String::new();
        if let Some(rel2) = chunk.find("\"formattedVeblen\"") {
            let vs2 = rel2 + 18;
            let Some(q2) = chunk[vs2..].find('"') else { continue };
            let s2 = vs2 + q2 + 1;
            let mut e2 = s2;
            while e2 < chunk.len() {
                let b = chunk.as_bytes()[e2];
                if b == b'\\' {
                    veblen.push(chunk.as_bytes()[e2 + 1] as char);
                    e2 += 2;
                } else if b == b'"' {
                    break;
                } else {
                    veblen.push(b as char);
                    e2 += 1;
                }
            }
        }
        out.push((input, veblen));
        pos = obj_end + 1;
    }
    out
}

#[test]
fn all_test_cases_veblen() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../test-cases.json");
    let data = std::fs::read_to_string(&path).expect("read test-cases.json");
    let cases = extract_fields(&data);
    let mut fail = 0;
    let mut checked = 0;
    for (input, expected) in &cases {
        if input.is_empty() || expected.is_empty() {
            continue;
        }
        checked += 1;
        let m = parse_matrix(input);
        let ord = bms_to_bocf(&m);
        let got = term_to_veblen(&ord);
        if &got != expected {
            fail += 1;
            if fail <= 15 {
                println!("MISMATCH: {} expected={} got={}", input, expected, got);
            }
        }
    }
    println!("checked {} cases: fail={}", checked, fail);
    assert_eq!(fail, 0, "test-case mismatches");
}
