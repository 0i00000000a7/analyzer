//! 1-Y / ω-Y sequence ↔ DBMS (Dimensional BMS) conversion.

use crate::triangular::triangular_to_bms;
use crate::wy::build_wy_mountain_with_rows;
use crate::zero_y::{bms_to_0y_sequence, zero_y_to_bms};
use crate::Matrix;

fn parse_seq_str(s: &str) -> Vec<i32> {
    s.split(',').map(|item| item.trim().parse().unwrap_or(0)).collect()
}

/// Check if row label transition is an ω-boundary
fn is_omega_boundary(a: &[i32], b: &[i32]) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if b.len() >= 2 && b[0] == 0 {
        let ext_a = if a.len() >= 2 { a[1] } else { 0 };
        let ext_b = b[1];
        return ext_b > ext_a;
    }
    if b.len() > a.len() + 1 {
        return true;
    }
    if b.len() > a.len() && *b.last().unwrap() > 0 {
        return true;
    }
    false
}

/// Count ω-boundaries in the ω-Y mountain row labels
fn count_omega_boundaries(wy_rows: &[Vec<i32>]) -> i32 {
    let mut count = 0;
    for i in 1..wy_rows.len() {
        if is_omega_boundary(&wy_rows[i - 1], &wy_rows[i]) {
            count += 1;
        }
    }
    count
}

/// Convert a 1-Y sequence to a DBMS matrix.
/// Below [1,3]: uses zeroYToBMS. [1,3]+: returns fixed placeholder ≥(0)(1)(2,1,,1).
pub fn one_y_to_dbms(seq: &[i32]) -> Matrix {
    if seq.is_empty() || seq[0] == 0 {
        return Vec::new();
    }
    for &v in seq {
        if v < 0 {
            return Vec::new();
        }
    }

    // Check for ω-boundaries using ω-Y mountain
    let (mountain, wy_rows) = build_wy_mountain_with_rows(seq, -1, false);
    if mountain.is_empty() {
        return Vec::new();
    }
    if count_omega_boundaries(&wy_rows) > 0 {
        // No stable algorithm for [1,3]+ yet; return fixed placeholder
        let mut placeholder: Matrix = Vec::new();
        placeholder.push(vec![0]); // (0)
        placeholder.push(vec![1]); // (1)
        placeholder.push(vec![2, 1, -2, 1]); // (2,1,,1)
        return placeholder;
    }

    // Below [1,3]: use 0-Y → BMS conversion
    let mut bms = zero_y_to_bms(seq);
    if bms.is_empty() {
        return Vec::new();
    }
    for col in &mut bms {
        while col.len() > 1 && *col.last().unwrap() == 0 {
            col.pop();
        }
    }
    bms
}

/// Convert a DBMS matrix to a 1-Y sequence.
pub fn dbms_to_one_y(dbms: &Matrix) -> Vec<i32> {
    if dbms.is_empty() || dbms[0].is_empty() {
        return Vec::new();
    }

    let mut has_marker = false;
    for col in dbms {
        for &v in col {
            if v == -2 {
                has_marker = true;
                break;
            }
        }
    }

    if has_marker {
        return Vec::new();
    }

    let seq_str = bms_to_0y_sequence(dbms);
    if seq_str.is_empty() {
        return Vec::new();
    }
    parse_seq_str(&seq_str)
}

/// Format a DBMS matrix as a readable string like (0)(1)(2,1)
pub fn dbms_to_string(dbms: &Matrix) -> String {
    let mut out = String::new();
    for col in dbms {
        out += "(";
        let mut first = true;
        for (_r, v) in col.iter().enumerate() {
            if *v == -2 {
                out += ",,";
                first = true;
                continue;
            }
            if !first {
                out += ",";
            }
            out += &v.to_string();
            first = false;
        }
        out += ")";
    }
    out
}

/// Convert DBMS to standard BMS.
pub fn dbms_to_bms(dbms: &Matrix) -> Matrix {
    for col in dbms {
        for &v in col {
            if v == -2 {
                return Vec::new();
            }
        }
    }
    let mut max_len = 0;
    for col in dbms {
        if col.len() > max_len {
            max_len = col.len();
        }
    }
    if max_len == 0 {
        return Vec::new();
    }
    let target = max_len.max(2);
    let mut padded: Matrix = Vec::with_capacity(dbms.len());
    for col in dbms {
        let mut r = col.clone();
        r.resize(target, 0);
        padded.push(r);
    }
    triangular_to_bms(&padded)
}
