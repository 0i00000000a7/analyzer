#![allow(unused_assignments)]
//! Ancestor-based BMS ↔ Triangular BMS conversion.

use crate::Matrix;
use std::collections::BTreeSet;

type Column = Vec<i32>;
type Columns = Vec<Column>;

fn col_eq(a: &Column, b: &Column) -> bool {
    a == b
}

fn col_less(a: &Column, b: &Column) -> bool {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    a.len() < b.len()
}

fn col_geq(a: &Column, b: &Column) -> bool {
    !col_less(a, b)
}

/// Lexicographic compare of two column sequences.
fn seq_cmp(a: &Columns, b: &Columns) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if col_eq(&a[i], &b[i]) {
            continue;
        }
        return if col_less(&a[i], &b[i]) { -1 } else { 1 };
    }
    if a.len() == b.len() {
        return 0;
    }
    if a.len() < b.len() {
        -1
    } else {
        1
    }
}

/// Largest 1-based row where col[row-1] > 0; 0 if all zero.
fn last_positive_row(col: &Column) -> i32 {
    for i in (0..col.len()).rev() {
        if col[i] > 0 {
            return i as i32 + 1;
        }
    }
    0
}

fn increment_prefix(col: &Column, count: i32) -> Column {
    let mut r = col.clone();
    for i in 0..(count as usize).min(r.len()) {
        r[i] += 1;
    }
    r
}

fn decrement_prefix(col: &Column, count: i32) -> Column {
    let mut r = col.clone();
    for i in 0..(count as usize).min(r.len()) {
        if r[i] == 0 {
            return Column::new(); // sentinel failure
        }
        r[i] -= 1;
    }
    r
}

fn increment_row(col: &Column, row: i32 /*1-based*/) -> Column {
    let mut r = col.clone();
    if row >= 1 && (row as usize) <= r.len() {
        r[(row - 1) as usize] += 1;
    }
    r
}

fn zero_from_row(col: &Column, row: i32 /*1-based*/) -> Column {
    let mut r = col.clone();
    for i in ((row - 1) as usize)..r.len() {
        r[i] = 0;
    }
    r
}

fn first_row_column(value: i32, n: usize) -> Column {
    let mut r = vec![0i32; n];
    r[0] = value;
    r
}

struct AncestorIndex {
    n: usize,
    column_count: usize,
    _columns: Columns,
    parents: Vec<Vec<i32>>,            // parents[row][col]; -1 = none
    ancestors: Vec<Vec<BTreeSet<usize>>>, // ancestors[row][col]
}

impl AncestorIndex {
    fn new(cols: &Columns) -> Self {
        if cols.is_empty() {
            return AncestorIndex {
                n: 0,
                column_count: 0,
                _columns: cols.clone(),
                parents: Vec::new(),
                ancestors: Vec::new(),
            };
        }
        let column_count = cols.len();
        let n = cols[0].len();

        let mut parents = vec![vec![-1i32; column_count]; n + 1];
        let mut ancestors: Vec<Vec<BTreeSet<usize>>> = vec![vec![BTreeSet::new(); column_count]; n + 1];

        // Row 0 (virtual): ancestors are all columns to the left
        for c in 1..column_count {
            parents[0][c] = c as i32 - 1;
            for a in 0..c {
                ancestors[0][c].insert(a);
            }
        }

        // Rows 1..n
        for row in 1..=n {
            let vi = row - 1;
            for c in 0..column_count {
                let mut parent: i32 = -1;
                // iterate up (ancestors of row-1 at c) in reverse (right to left)
                let up: Vec<usize> = ancestors[row - 1][c].iter().cloned().collect();
                for &it in up.iter().rev() {
                    if cols[it][vi] < cols[c][vi] {
                        parent = it as i32;
                        break;
                    }
                }
                parents[row][c] = parent;
                if parent >= 0 {
                    let pu = parent as usize;
                    ancestors[row][c] = ancestors[row][pu].clone();
                    ancestors[row][c].insert(pu);
                }
            }
        }

        AncestorIndex {
            n,
            column_count,
            _columns: cols.clone(),
            parents,
            ancestors,
        }
    }

    fn has_ancestor_column(&self, element_col: usize, row: usize, ancestor_col: usize) -> bool {
        if row > self.n || element_col >= self.column_count || ancestor_col >= self.column_count {
            return false;
        }
        self.ancestors[row][element_col].contains(&ancestor_col)
    }

    fn parent_is_column(&self, element_col: usize, row: usize, parent_col: usize) -> bool {
        if row > self.n || element_col >= self.column_count {
            return false;
        }
        self.parents[row][element_col] == parent_col as i32
    }
}

// ============================================================
// Triangular BMS → Standard BMS
// ============================================================

pub fn triangular_to_bms(m: &Matrix) -> Matrix {
    let mut cols = m.clone();
    if cols.is_empty() {
        return Vec::new();
    }
    let n = cols[0].len();
    if n < 2 {
        return Vec::new();
    }

    let mut idx = cols.len() as i32 - 1;

    while idx >= 0 {
        let iu = idx as usize;
        let x = cols[iu].clone();
        let row_n_minus_2 = if n >= 2 { x[n - 2] } else { 0 };
        if row_n_minus_2 > 0 {
            idx -= 1;
            continue;
        }

        let k = last_positive_row(&x);
        if k + 2 > n as i32 {
            idx -= 1;
            continue;
        }

        let y = increment_prefix(&x, k + 1);
        let z = increment_prefix(&y, k + 2);

        let y_idx = iu + 1;
        let machine_start = iu + 2;

        if y_idx >= cols.len()
            || !col_eq(&cols[y_idx], &y)
            || machine_start >= cols.len()
            || col_less(&cols[machine_start], &z)
        {
            idx -= 1;
            continue;
        }

        let ancestor = AncestorIndex::new(&cols);
        let mut x_prime: Columns = Vec::new();
        let mut cursor = machine_start;
        let mut x_end = cursor;
        let mut last_l: i32 = -1;
        let mut last_stopped_by_x_parent = false;

        loop {
            if cursor >= cols.len() || col_less(&cols[cursor], &z) {
                x_end = cursor;
                break;
            }

            let t = cols[cursor].clone();

            // Find matching rows l (0..k+1) where t[l] has ancestor in y
            let mut l: i32 = -1;
            for row in 0..=(k as usize + 1) {
                if ancestor.has_ancestor_column(cursor, row, y_idx) {
                    l = row as i32; // take max
                }
            }
            if l < 0 {
                return Vec::new(); // non-standard
            }

            let stopped_by_x_parent = (l <= k) && ancestor.parent_is_column(cursor, (l + 1) as usize, iu);

            let mut t_prime = decrement_prefix(&t, l);
            if t_prime.is_empty() {
                return Vec::new();
            }

            if stopped_by_x_parent {
                t_prime = zero_from_row(&t_prime, l + 2);
            }

            x_prime.push(t_prime);
            cursor += 1;

            last_l = l;
            last_stopped_by_x_parent = stopped_by_x_parent;

            if stopped_by_x_parent {
                x_end = cursor;
                break;
            }
        }

        // Determine if we keep y and the original X
        let mut keep_case1 = false;
        if x_end < cols.len() {
            let frc = first_row_column(z[0], n);
            keep_case1 = col_geq(&cols[x_end], &frc);
        }

        let mut keep_case2 = false;
        if last_l >= 0 && x_end > 0 && x_end - 1 < cols.len() {
            if cols[x_end - 1][last_l as usize] == 0 && ancestor.parent_is_column(x_end - 1, last_l as usize, y_idx) {
                keep_case2 = true;
            }
        }

        let keep_case3 = last_stopped_by_x_parent
            && (last_l + 1) < n as i32
            && x_end > 0
            && (x_end - 1) < cols.len()
            && cols[x_end - 1][(last_l + 1) as usize] > 0;

        let keep_original_yx = keep_case1 || keep_case2 || keep_case3;

        if keep_original_yx {
            // Keep y and x_prime after x, original X remains
            let mut new_cols: Columns = Vec::with_capacity(cols.len() + x_prime.len());
            new_cols.extend_from_slice(&cols[..iu + 1]);
            new_cols.extend(x_prime.iter().cloned());
            new_cols.extend_from_slice(&cols[iu + 1..]);
            cols = new_cols;
        } else {
            // Replace y..X with x_prime only
            let mut new_cols: Columns = Vec::with_capacity(cols.len());
            new_cols.extend_from_slice(&cols[..iu + 1]);
            new_cols.extend(x_prime.iter().cloned());
            new_cols.extend_from_slice(&cols[x_end..]);
            cols = new_cols;
        }

        idx -= 1;
    }

    cols
}

// ============================================================
// Standard BMS → Triangular BMS
// ============================================================

pub fn bms_to_triangular(m: &Matrix) -> Matrix {
    let mut cols = m.clone();
    if cols.is_empty() {
        return Vec::new();
    }
    let n = cols[0].len();
    if n < 2 {
        return Vec::new();
    }

    let mut idx: usize = 0;
    let mut steps: i64 = 0;
    const STEP_LIMIT: i64 = 100000;

    while idx < cols.len() {
        steps += 1;
        if steps > STEP_LIMIT {
            return Vec::new();
        }

        let x = cols[idx].clone();
        let k = last_positive_row(&x);
        if k >= n as i32 - 1 {
            idx += 1;
            continue;
        }

        let y = increment_prefix(&x, k + 1);
        let z = increment_row(&y, k + 2);

        let x_start = idx + 1;
        if x_start >= cols.len() || col_less(&cols[x_start], &z) {
            idx += 1;
            continue;
        }

        let mut x_end = x_start;
        while x_end < cols.len() && col_geq(&cols[x_end], &z) {
            x_end += 1;
        }

        let ancestor = AncestorIndex::new(&cols);
        let mut x_prime: Columns = Vec::new();

        for cursor in x_start..x_end {
            let t = cols[cursor].clone();

            let mut l: i32 = -1;
            for row in 0..=(k as usize + 1) {
                if ancestor.has_ancestor_column(cursor, row, idx) {
                    l = row as i32;
                }
            }
            if l < 0 {
                return Vec::new();
            }

            let is_last = cursor == x_end - 1;
            if is_last {
                if l < 0 || (l as usize) >= n {
                    return Vec::new();
                }
                if ancestor.parent_is_column(cursor, l as usize, idx) && t[l as usize] == 0 {
                    l -= 1;
                }
            }
            if l < 0 {
                return Vec::new();
            }

            let t_prime = increment_prefix(&t, l);
            x_prime.push(t_prime);
        }

        // comparison matrix: (y, ...x_prime, (y[0]+1,0,...,0))
        let mut insertion: Columns = Vec::new();
        insertion.push(y.clone());
        insertion.extend(x_prime.iter().cloned());

        let mut comparison = insertion.clone();
        comparison.push(first_row_column(y[0] + 1, n));

        let remainder: Columns = cols[x_end..].to_vec();

        // Always delete X
        cols.drain(x_start..x_end);

        // Insert if comparison > remainder (lexicographic tuple-of-tuples)
        if seq_cmp(&comparison, &remainder) > 0 {
            let mut new_cols: Columns = Vec::with_capacity(cols.len() + insertion.len());
            new_cols.extend_from_slice(&cols[..x_start]);
            new_cols.extend(insertion.iter().cloned());
            new_cols.extend_from_slice(&cols[x_start..]);
            cols = new_cols;
        }

        idx += 1;
    }

    cols
}
