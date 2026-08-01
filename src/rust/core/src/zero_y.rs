//! 0-Y sequence conversions and expansion (Mt. Fuji algorithm).

use crate::Matrix;
use crate::Mountain;

/// Convert a 0-Y sequence to a BMS matrix.
pub fn zero_y_to_bms(seq: &[i32]) -> Matrix {
    let l = seq.len();
    let mut res: Matrix = vec![Vec::new(); l];
    let mut parents: Vec<i32> = (0..l as i32).map(|i| i - 1).collect();
    let mut cur: Vec<i32> = seq.to_vec();

    loop {
        let mut next = vec![0i32; l];
        let mut has_parent = false;

        for i in 0..l {
            let mut k = i as i32;
            while k >= 0 && cur[k as usize] >= cur[i] {
                k = parents[k as usize];
            }
            parents[i] = k;
            if k >= 0 {
                has_parent = true;
                next[i] = cur[i] - cur[k as usize];
                let row = res[i].len();
                let v = res[k as usize][row] + 1;
                res[i].push(v);
            } else {
                next[i] = 1;
                res[i].push(0);
            }
        }

        if !has_parent {
            break;
        }
        cur = next;
    }

    // Remove the last row (all-1s iteration), keeping at least one row per column
    for i in 0..l {
        if res[i].len() > 1 {
            res[i].pop();
        }
    }

    // Pad to uniform length
    let max_rows = res.iter().map(|c| c.len()).max().unwrap_or(0);
    for i in 0..l {
        while res[i].len() < max_rows {
            res[i].push(0);
        }
    }

    res
}

/// Convert a BMS matrix to its equivalent 0-Y sequence string.
pub fn bms_to_0y_sequence(m: &Matrix) -> String {
    let cols = m.len();
    if cols == 0 {
        return String::new();
    }
    let rows = m[0].len();

    // Pad matrix to uniform row count
    let mut s: Vec<Vec<i32>> = Vec::with_capacity(cols);
    for i in 0..cols {
        let mut col = m[i].clone();
        while col.len() < rows {
            col.push(0);
        }
        s.push(col);
    }

    let mut result = vec![1i32; cols];

    // Process rows from bottom to top
    for row in (0..rows).rev() {
        let mut stack: Vec<usize> = Vec::new();
        for col in 0..cols {
            while let Some(&back) = stack.last() {
                if s[back][row] >= s[col][row] {
                    stack.pop();
                } else {
                    break;
                }
            }
            if !stack.is_empty() {
                result[col] += result[*stack.last().unwrap()];
            }
            stack.push(col);
        }
    }

    // Format as comma-separated string
    let parts: Vec<String> = result.iter().map(|v| v.to_string()).collect();
    parts.join(",")
}

/// Build the mountain 2D structure from a 0-Y sequence.
pub fn build_mountain(seq: &[i32]) -> Mountain {
    let len = seq.len();
    let mut mountain: Mountain = Vec::new();

    let bottom: Vec<(i32, i32)> = seq.iter().map(|&v| (v, 0)).collect();
    mountain.push(bottom);

    loop {
        let last = mountain.len() - 1;
        let mut has_parent = false;

        for x in 1..len {
            if mountain[last][x].1 != 0 {
                continue;
            }
            let mut p = x as i32;
            while p >= 0 {
                let pu = p as usize;
                let has_upper_parent = mountain.len() == 1 || mountain[mountain.len() - 2][pu].1 != 0;
                if !has_upper_parent {
                    break;
                }
                if mountain[last][pu].0 < mountain[last][x].0 {
                    break;
                }
                p -= if mountain.len() == 1 { 1 } else { mountain[mountain.len() - 2][pu].1 };
            }
            if p >= 0 {
                let p_val = mountain[last][p as usize].0;
                if p_val != 0 && p_val < mountain[last][x].0 {
                    mountain[last][x].1 = x as i32 - p;
                    has_parent = true;
                }
            }
        }

        if !has_parent {
            break;
        }

        let mut next_layer = vec![(1i32, 0i32); len];
        for x in 1..len {
            if mountain[last][x].1 != 0 {
                let parent_idx = x - mountain[last][x].1 as usize;
                next_layer[x].0 = mountain[last][x].0 - mountain[last][parent_idx].0;
            }
        }
        mountain.push(next_layer);
    }
    mountain
}

/// Expand a 0-Y sequence by n steps (Mt. Fuji algorithm).
pub fn zero_y_expand(seq: &[i32], n: i32) -> Vec<i32> {
    if seq.is_empty() {
        return Vec::new();
    }
    let mountain = build_mountain(seq);

    let height = mountain.len();
    let cut_pos = mountain[0].len() - 1;

    // Find cut height: how many layers the last element has a parent
    let mut cut_height = 0;
    while cut_height + 1 < height && mountain[cut_height][cut_pos].1 != 0 {
        cut_height += 1;
    }

    if cut_height == 0 {
        // Last element is 0-height — simply remove it
        let mut result = seq.to_vec();
        result.pop();
        return result;
    }

    let bad_root_pos = cut_pos as i32 - mountain[cut_height - 1][cut_pos].1;
    let bad_len = cut_pos as i32 - bad_root_pos;

    let mut result = mountain.clone();

    // Remove last column from all layers
    for y in 0..height {
        result[y].pop();
    }

    // Create Mt. Fuji shell (copy bad part with offset adjustments)
    for i in 1..=n {
        for x in bad_root_pos..cut_pos as i32 {
            for y in 0..height {
                let orig_offset = mountain[y][x as usize].1;
                let has_parent = orig_offset != 0;

                if x == bad_root_pos && (y as i32) < cut_height as i32 - 1 {
                    // First new column in this iteration: copy the cut's offset
                    result[y].push((-1, mountain[y][cut_pos].1));
                } else if !has_parent {
                    // No parent: copy value as-is
                    result[y].push((mountain[y][x as usize].0, 0));
                } else if has_parent && x - orig_offset >= bad_root_pos && (x > bad_root_pos || (y as i32) < cut_height as i32) {
                    // Parent is within the bad part: keep original offset
                    result[y].push((-1, orig_offset));
                } else {
                    // Parent is outside: adjust offset by badLen * iteration
                    result[y].push((-1, orig_offset + bad_len * i));
                }
            }
        }
    }

    // Recompute NaN values from bottom to top, left to right
    let result_len = result[0].len();
    for x in 0..result_len {
        for y in (0..height).rev() {
            if result[y][x].0 == -1 {
                let offset = result[y][x].1;
                let parent_idx = x as i32 - offset;
                let upper_val = if y + 1 < height { result[y + 1][x].0 } else { 0 };
                let parent_val = if parent_idx >= 0 { result[y][parent_idx as usize].0 } else { 0 };
                result[y][x].0 = upper_val + parent_val;
            }
        }
    }

    // Extract top row as the result sequence
    result[0].iter().map(|&(v, _)| v).collect()
}
