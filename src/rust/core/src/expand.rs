//! BMS matrix expansion and lexicographic order.

use crate::Matrix;

/// Expand a BMS matrix by `fs` fundamental-sequence steps.
pub fn expand_bms(m: &Matrix, fs: i32) -> Matrix {
    let l = m.len();
    if l == 0 {
        return Vec::new();
    }

    // Determine row count
    let rows = m.iter().map(|c| c.len()).max().unwrap_or(0);

    // Build uniform matrix (pad with 0)
    let mut s: Vec<Vec<i32>> = Vec::with_capacity(l);
    for i in 0..l {
        let mut col = m[i].clone();
        while col.len() < rows {
            col.push(0);
        }
        s.push(col);
    }

    // Calculate parent matrix: parents[col][row]
    let mut parents: Vec<Vec<i32>> = vec![Vec::new(); l];
    for row in 0..rows {
        if row == 0 {
            let mut stack: Vec<usize> = Vec::new();
            for col in 0..l {
                while let Some(&back) = stack.last() {
                    if s[back][0] >= s[col][0] {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                parents[col].push(if stack.is_empty() { -1 } else { stack.last().unwrap().clone() as i32 });
                stack.push(col);
            }
        } else {
            for col in 0..l {
                let mut k = col as i32;
                while k >= 0 && s[k as usize][row] >= s[col][row] {
                    k = parents[k as usize][row - 1];
                }
                parents[col].push(k);
            }
        }
    }

    // Find the highest non-zero row in the last column
    let mut x: i32 = -1;
    while ((x + 1) as usize) < rows && s[l - 1][(x + 1) as usize] > 0 {
        x += 1;
    }

    // Not a limit ordinal — just remove the last column
    if x < 0 {
        let mut res: Matrix = Vec::with_capacity(l - 1);
        for i in 0..l - 1 {
            res.push(s[i].clone());
        }
        return res;
    }

    let mut bad_root = parents[l - 1][x as usize];

    // If bad root is -1 at row x, fall back to row 0
    if bad_root < 0 {
        bad_root = parents[l - 1][0];
        if bad_root < 0 {
            // Still no parent — just remove the last column
            let mut res: Matrix = Vec::with_capacity(l - 1);
            for i in 0..l - 1 {
                res.push(s[i].clone());
            }
            return res;
        }
    }

    let bad_length = l - 1 - bad_root as usize;

    // Ascension values for rows below x
    let mut asc_value = vec![0i32; rows];
    for i in 0..x as usize {
        asc_value[i] = s[l - 1][i] - s[bad_root as usize][i];
    }

    // Ascension matrix
    let mut asc_mat = vec![vec![0i32; rows]; bad_length];
    for i in 0..x as usize {
        for j in 0..bad_length {
            let mut k = j as i32 + bad_root;
            while k > bad_root {
                k = parents[k as usize][i];
            }
            asc_mat[j][i] = if k == bad_root { 1 } else { 0 };
        }
    }

    // Build result: keep all columns except the last
    let mut res: Matrix = Vec::with_capacity(l - 1 + (fs as usize) * bad_length);
    for i in 0..l - 1 {
        res.push(s[i].clone());
    }

    // Expand: repeat bad part with ascension
    for step in 1..=fs {
        for j in bad_root as usize..l - 1 {
            let mut col = vec![0i32; rows];
            for k in 0..rows {
                col[k] = s[j][k] + asc_value[k] * step * asc_mat[j - bad_root as usize][k];
            }
            res.push(col);
        }
    }

    res
}

/// Lexicographic order on matrices. Returns 1, -1, or 0.
pub fn matrix_lex_order(a: &Matrix, b: &Matrix) -> i32 {
    let max_rows = a.len().max(b.len());
    for i in 0..max_rows {
        let a_len = if i < a.len() { a[i].len() } else { 0 };
        let b_len = if i < b.len() { b[i].len() } else { 0 };
        let max_cols = a_len.max(b_len);
        for j in 0..max_cols {
            let va = if i < a.len() && j < a[i].len() { a[i][j] } else { 0 };
            let vb = if i < b.len() && j < b[i].len() { b[i][j] } else { 0 };
            if va > vb {
                return 1;
            }
            if va < vb {
                return -1;
            }
        }
    }
    0
}
