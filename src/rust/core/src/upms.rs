//! UPMS (Unupgrading Projection Matrix System) expansion.
//!
//! Column-major layout: matrix[colIndex][rowIndex].

use crate::Matrix;

/// Pad/crop a matrix to a common row count (same as TS standardizeMatrix).
pub fn standardize_matrix(matrix: &Matrix) -> Matrix {
    if matrix.is_empty() {
        return Vec::new();
    }
    let mut rows = 1;
    for col in matrix {
        if col.len() > rows {
            rows = col.len();
        }
    }
    let mut result: Matrix = matrix
        .iter()
        .map(|col| {
            let mut out = col.clone();
            while out.len() < rows {
                out.push(0);
            }
            out
        })
        .collect();
    while rows > 1 && result.iter().all(|col| col[rows - 1] == 0) {
        for col in result.iter_mut() {
            col.pop();
        }
        rows -= 1;
    }
    result
}

// ── Context: parent/ancestor caches ──────────────────────────────

struct Context {
    m: Matrix,
    col_count: usize,
    row_count: usize,
    parent_cache: Vec<Vec<i32>>, // [b][col], -2 = uncached
    ancestor_cache: Vec<Vec<Option<(Vec<usize>, Vec<u8>)>>>, // [a][col]
}

impl Context {
    fn new(matrix: &Matrix) -> Context {
        let m = standardize_matrix(matrix);
        let col_count = m.len();
        let row_count = if col_count == 0 { 0 } else { m[0].len() };
        Context {
            parent_cache: vec![vec![-2; col_count]; row_count + 1],
            ancestor_cache: vec![vec![None; col_count]; row_count + 1],
            m,
            col_count,
            row_count,
        }
    }

    fn get_zero_parent(&self, col: usize) -> i32 {
        if col > 0 {
            col as i32 - 1
        } else {
            -1
        }
    }

    fn get_a_ancestors(&mut self, col: usize, a: usize) -> (Vec<usize>, Vec<u8>) {
        if a > self.row_count || col >= self.col_count {
            return (Vec::new(), vec![0; self.col_count]);
        }
        if let Some(cached) = &self.ancestor_cache[a][col] {
            return cached.clone();
        }
        let mut list: Vec<usize> = Vec::new();
        let mut mask = vec![0u8; self.col_count];
        let mut current: i32 = col as i32;
        let mut guard = 0usize;
        while current != -1 && mask[current as usize] == 0 && guard <= self.col_count + 2 {
            let cur = current as usize;
            list.push(cur);
            mask[cur] = 1;
            current = if a == 0 {
                self.get_zero_parent(cur)
            } else {
                self.get_b_parent(cur, a)
            };
            guard += 1;
        }
        let result = (list, mask);
        self.ancestor_cache[a][col] = Some(result.clone());
        result
    }

    fn get_b_parent(&mut self, col: usize, b: usize) -> i32 {
        if b < 1 || b > self.row_count || col >= self.col_count {
            return -1;
        }
        let cached = self.parent_cache[b][col];
        if cached != -2 {
            return cached;
        }
        let row = b - 1;
        let value = self.m[col][row];
        let (ancestors, _) = self.get_a_ancestors(col, b - 1);
        let mut best: i32 = -1;
        for &candidate in &ancestors {
            if candidate >= col {
                continue;
            }
            if self.m[candidate][row] < value {
                best = candidate as i32;
                break;
            }
        }
        self.parent_cache[b][col] = best;
        best
    }
}

// ── Expansion helpers ────────────────────────────────────────────

fn last_column_is_zero(matrix: &Matrix) -> bool {
    if matrix.is_empty() {
        return true;
    }
    matrix[matrix.len() - 1].iter().all(|&v| v == 0)
}

fn find_last_non_zero_row_label(matrix: &Matrix) -> i32 {
    if matrix.is_empty() {
        return -1;
    }
    let last = &matrix[matrix.len() - 1];
    for r in (0..last.len()).rev() {
        if last[r] != 0 {
            return r as i32 + 1;
        }
    }
    -1
}

fn find_bad_root(ctx: &mut Context) -> Option<(usize, usize)> {
    let last_col = ctx.col_count - 1;
    let t = find_last_non_zero_row_label(&ctx.m);
    if t == -1 {
        return None;
    }
    let root_col = ctx.get_b_parent(last_col, t as usize);
    if root_col == -1 {
        return None;
    }
    Some((root_col as usize, t as usize))
}

fn compute_delta(ctx: &Context, root_col: usize, t: usize) -> Vec<i32> {
    let last_col = ctx.col_count - 1;
    let mut delta = vec![0i32; ctx.row_count];
    for r in 0..ctx.row_count {
        delta[r] = if r >= t - 1 {
            0
        } else {
            ctx.m[last_col][r] - ctx.m[root_col][r]
        };
    }
    delta
}

fn max_entry(matrix: &Matrix) -> i32 {
    let mut max = 0;
    for col in matrix {
        for &v in col {
            if v > max {
                max = v;
            }
        }
    }
    max
}

fn sequence_compare(s1: &[i32], s2: &[i32]) -> i32 {
    let len = s1.len().max(s2.len());
    for i in 0..len {
        let a = if i < s1.len() { s1[i] } else { 0 };
        let b = if i < s2.len() { s2[i] } else { 0 };
        if a < b {
            return -1;
        }
        if a > b {
            return 1;
        }
    }
    0
}

fn conv_matrix_compare(m1: &ConvMatrix, m2: &ConvMatrix) -> i32 {
    let len = m1.len().max(m2.len());
    for c in 0..len {
        if c >= m1.len() {
            return -1;
        }
        if c >= m2.len() {
            return 1;
        }
        let cmp = sequence_compare(&m1[c], &m2[c]);
        if cmp != 0 {
            return cmp;
        }
    }
    0
}

struct VerificationRoots {
    data: Vec<i8>,
    height: usize,
}

impl VerificationRoots {
    fn index(&self, col: usize, row: usize) -> usize {
        col * self.height + row
    }
}

/// The verification-root (VR) computation.
/// Computes BadRoot(i), Δ_i and VR(i).
fn compute_upms_verification_roots(ctx: &mut Context, root_col: usize, t: usize) -> VerificationRoots {
    let m = ctx.m.clone();
    let alpha = ctx.col_count - 1;
    let y = root_col;
    let height = ctx.row_count;
    let max_twice = max_entry(&m) * 2;
    let mut vr = vec![-1i8; ctx.col_count * height];

    let in_bad_part = |col: usize, row: usize| col >= y && col < alpha && row < t - 1;
    let get_vr = |vr: &Vec<i8>, col: usize, row: usize| {
        if in_bad_part(col, row) {
            vr[col * height + row]
        } else {
            -1
        }
    };
    let set_vr = |vr: &mut Vec<i8>, col: usize, row: usize, value: i8| {
        vr[col * height + row] = value;
    };

    let base_value = |col: usize, k: usize, r: usize| m[col][r] + if r < k { 1 } else { 0 };

    let column_less_than_base = |m: &Matrix, candidate: usize, col: usize, k: usize| {
        let limit = k + 1;
        for r in 0..limit {
            let a = if r < height { m[candidate][r] } else { 0 };
            let b = base_value(col, k, r);
            if a < b {
                return true;
            }
            if a > b {
                return false;
            }
        }
        false
    };

    let transformed_x_value = |m: &Matrix, vr: &Vec<i8>, source_col: usize, row: usize, i_col: usize, k: usize| {
        let mut value = m[source_col][row];
        if row + 1 < k && get_vr(vr, source_col, row) == 1 {
            value += max_twice - m[i_col][row];
        }
        value
    };

    let transformed_y_value = |m: &Matrix, ctx: &mut Context, source_col: usize, row: usize, j_col: usize, k: usize| {
        let mut value = m[source_col][row];
        if row + 1 < k {
            let col_is_j = source_col == j_col;
            let (_, mask) = ctx.get_a_ancestors(source_col, row + 1);
            let contains_j = mask[j_col] == 1;
            if col_is_j || contains_j {
                value += max_twice - m[j_col][row];
            }
        }
        value
    };

    let compare_transformed_parts = |m: &Matrix,
                                        ctx: &mut Context,
                                        vr: &Vec<i8>,
                                        x_start: usize,
                                        x_end: usize,
                                        y_start: usize,
                                        j_col: usize,
                                        i_col: usize,
                                        k: usize|
     -> i32 {
        let x_len = x_end - x_start + 1;
        let y_len = alpha - y_start + 1;
        let common_cols = x_len.min(y_len);
        for local in 0..common_cols {
            let x_col = x_start + local;
            let y_col = y_start + local;
            for row in 0..height {
                let xv = transformed_x_value(m, vr, x_col, row, i_col, k);
                let yv = transformed_y_value(m, ctx, y_col, row, j_col, k);
                if xv < yv {
                    return -1;
                }
                if xv > yv {
                    return 1;
                }
            }
        }
        if x_len < y_len {
            return -1;
        }
        if x_len > y_len {
            return 1;
        }
        0
    };

    for row in 0..t - 1 {
        let k = row + 1;
        for col in y..alpha {
            if col == y || row == 0 {
                set_vr(&mut vr, col, row, 1);
                continue;
            }
            let (k_ancestors, k_mask) = ctx.get_a_ancestors(col, k);
            let mut ancestor_has_vr0 = false;
            for &a in &k_ancestors {
                if get_vr(&vr, a, row) == 0 {
                    ancestor_has_vr0 = true;
                    break;
                }
            }
            let k_parent = ctx.get_b_parent(col, k);
            if k_mask[y] != 1 || ancestor_has_vr0 || k_parent == -1 {
                set_vr(&mut vr, col, row, 0);
                continue;
            }
            if k_parent as usize != y {
                set_vr(&mut vr, col, row, 1);
                continue;
            }
            let mut earlier_row_has_vr0 = false;
            for w_row in 0..row {
                if get_vr(&vr, col, w_row) == 0 {
                    earlier_row_has_vr0 = true;
                    break;
                }
            }
            if earlier_row_has_vr0 {
                set_vr(&mut vr, col, row, 0);
                continue;
            }
            let mut higher_parent_escapes_bad_root = false;
            for v_row in row + 1..t - 1 {
                if ctx.get_b_parent(col, v_row + 1) != y as i32 {
                    higher_parent_escapes_bad_root = true;
                    break;
                }
            }
            if higher_parent_escapes_bad_root {
                set_vr(&mut vr, col, row, 0);
                continue;
            }
            let mut u: i32 = -1;
            for candidate in col + 1..=alpha {
                if column_less_than_base(&m, candidate, col, k) {
                    u = candidate as i32;
                    break;
                }
            }
            if u == -1 {
                set_vr(&mut vr, col, row, 1);
                continue;
            }
            let a_yk = m[y][row];
            let (alpha_ancestors, _) = ctx.get_a_ancestors(alpha, k);
            let mut j: i32 = -1;
            for &a in &alpha_ancestors {
                if m[a][row] == a_yk + 1 {
                    j = a as i32;
                    break;
                }
            }
            if j == -1 {
                j = alpha as i32;
            }
            let cmp = compare_transformed_parts(&m, ctx, &vr, col, u as usize - 1, j as usize, j as usize, col, k);
            set_vr(&mut vr, col, row, if cmp < 0 { 0 } else { 1 });
        }
    }

    VerificationRoots { data: vr, height }
}

fn generate_bh(
    ctx: &Context,
    b: &Matrix,
    delta: &[i32],
    t: usize,
    h: i32,
    root_col: usize,
    vr: &VerificationRoots,
) -> Matrix {
    b.iter()
        .enumerate()
        .map(|(local_col, col)| {
            let original_col = root_col + local_col;
            let mut next = vec![0i32; ctx.row_count];
            for r in 0..ctx.row_count {
                let has_vr = r < t - 1 && vr.data[vr.index(original_col, r)] == 1;
                next[r] = col[r] + h * delta[r] * if has_vr { 1 } else { 0 };
            }
            next
        })
        .collect()
}

/// Expand a UPMS matrix by `index` steps. Returns the expanded matrix.
/// Mirrors the reference UPMS.ts expandUPMS.
pub fn expand_upms(matrix: &Matrix, index: i32) -> Matrix {
    if !is_legal_upms_matrix(matrix) {
        return Vec::new();
    }
    let mut ctx = Context::new(matrix);
    let n = index.max(0);
    if ctx.m.is_empty() {
        return Vec::new();
    }
    if last_column_is_zero(&ctx.m) {
        let mut m = ctx.m.clone();
        m.pop();
        return standardize_matrix(&m);
    }
    let Some((root_col, t)) = find_bad_root(&mut ctx) else {
        return Vec::new();
    };
    let g: Matrix = ctx.m[..root_col].to_vec();
    let b: Matrix = ctx.m[root_col..ctx.col_count - 1].to_vec();
    let delta = compute_delta(&ctx, root_col, t);
    let vr = compute_upms_verification_roots(&mut ctx, root_col, t);
    let mut result: Matrix = Vec::new();
    result.extend(g.iter().cloned());
    result.extend(b.iter().cloned());
    for h in 1..=n {
        let bh = generate_bh(&ctx, &b, &delta, t, h, root_col, &vr);
        for col in bh {
            result.push(col);
        }
    }
    standardize_matrix(&result)
}

/// Validation of isLegalUPMSMatrix.
pub fn is_legal_upms_matrix(matrix: &Matrix) -> bool {
    if matrix.is_empty() {
        return true;
    }
    for col in matrix {
        for &v in col {
            if v < 0 {
                return false;
            }
        }
    }
    let m = standardize_matrix(matrix);
    if m.is_empty() {
        return true;
    }
    let rows = m[0].len();
    for r in 0..rows {
        if m[0][r] != 0 {
            return false;
        }
    }
    for c in 0..m.len() {
        let col = &m[c];
        for r in 1..rows {
            if col[r] > col[r - 1] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_basic() {
        let m = vec![vec![0], vec![1, 1]];
        let r = expand_upms(&m, 3);
        assert_eq!(r, vec![vec![0], vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn upms_to_bms_boundary() {
        // UPMS boundary (0,0,0)(1,1,1)(2,1,1) should map to BMS boundary (0,0,0)(1,1,1)(2,1,0)(1,1,1)
        let upms = vec![[0, 0, 0], [1, 1, 1], [2, 1, 1]];
        let result = upms_to_bms_raw(&upms).unwrap();
        assert_eq!(result, vec![[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 1, 1]]);
    }

    #[test]
    fn bms_to_upms_boundary() {
        // BMS boundary (0,0,0)(1,1,1)(2,1,0)(1,1,1) should map to UPMS boundary (0,0,0)(1,1,1)(2,1,1)
        let bms = vec![[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 1, 1]];
        let result = bms_to_upms_squeeze(&bms).unwrap();
        assert_eq!(result, vec![[0, 0, 0], [1, 1, 1], [2, 1, 1]]);
    }
}

// ════════════════════════════════════════════════════════════════
// BMS↔UPMS conversion
// ════════════════════════════════════════════════════════════════

type ConvColumn = [i32; 3];
type ConvMatrix = Vec<ConvColumn>;

const EMPTY_COLUMN: ConvColumn = [-1, -1, -1];

fn bms_boundary() -> ConvMatrix {
    vec![[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 1, 1]]
}

fn upms_boundary() -> ConvMatrix {
    vec![[0, 0, 0], [1, 1, 1], [2, 1, 1]]
}

fn column_at(matrix: &ConvMatrix, index: usize) -> ConvColumn {
    if index < matrix.len() {
        matrix[index]
    } else {
        EMPTY_COLUMN
    }
}

fn highest_non_zero_row(col: &ConvColumn) -> usize {
    for r in (0..3).rev() {
        if col[r] != 0 {
            return r + 1;
        }
    }
    0
}

type ParentCache = std::collections::HashMap<(usize, usize), Option<usize>>;

fn parent_index(matrix: &ConvMatrix, index: usize, level: usize, cache: &mut ParentCache) -> Option<usize> {
    if let Some(&cached) = cache.get(&(index, level)) {
        return cached;
    }
    let col = matrix[index];
    let result: Option<usize>;
    if level == 1 {
        result = (0..index).rev().find(|&c| col[0] > matrix[c][0]);
    } else {
        let mut result_inner: Option<usize> = None;
        let mut candidate = parent_index(matrix, index, level - 1, cache);
        while let Some(c) = candidate {
            let col_slice = &col[level - 1..];
            let cand_slice = &matrix[c][level - 1..];
            if sequence_compare(col_slice, cand_slice) > 0 {
                result_inner = Some(c);
                break;
            }
            candidate = parent_index(matrix, c, level - 1, cache);
        }
        result = result_inner;
    }
    cache.insert((index, level), result);
    result
}

fn ancestor_indices(matrix: &ConvMatrix, index: usize, level: usize, cache: &mut ParentCache) -> Vec<usize> {
    let mut result: Vec<usize> = Vec::new();
    let mut candidate = parent_index(matrix, index, level, cache);
    while let Some(c) = candidate {
        result.push(c);
        candidate = parent_index(matrix, c, level, cache);
    }
    result
}

fn is_ancestor(matrix: &ConvMatrix, ancestor: usize, index: usize, level: usize, cache: &mut ParentCache) -> bool {
    ancestor_indices(matrix, index, level, cache).contains(&ancestor)
}

fn child_above_parent(matrix: &ConvMatrix, index: usize, level: usize, parent: usize, cache: &mut ParentCache) -> Option<usize> {
    if parent_index(matrix, index, level, cache) == Some(parent) {
        return Some(index);
    }
    for anc in ancestor_indices(matrix, index, level, cache) {
        if parent_index(matrix, anc, level, cache) == Some(parent) {
            return Some(anc);
        }
    }
    None
}

fn add_matrices(left: &ConvMatrix, right: &ConvMatrix) -> ConvMatrix {
    left.iter()
        .enumerate()
        .map(|(i, lc)| {
            let rc = right[i];
            [lc[0] + rc[0], lc[1] + rc[1], lc[2] + rc[2]]
        })
        .collect()
}

fn scale_matrix(factor: i32, matrix: &ConvMatrix) -> ConvMatrix {
    matrix.iter().map(|col| [col[0] * factor, col[1] * factor, col[2] * factor]).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum ConvSystem {
    Bms,
    Upms,
}

fn conv_fundamental_sequence(matrix: &ConvMatrix, number: i32, system: ConvSystem) -> Result<ConvMatrix, String> {
    if number < 0 {
        return Err("Fundamental-sequence index must be nonnegative".to_string());
    }
    let source = matrix.clone();
    if source.is_empty() {
        return Err("Empty expression".to_string());
    }
    let last = source[source.len() - 1];
    if last[0] == 0 && last[1] == 0 && last[2] == 0 {
        return Err("Successor expression".to_string());
    }

    let last_index = source.len() - 1;
    let last_column = source[last_index];
    let m = highest_non_zero_row(&last_column);
    let mut cache: ParentCache = std::collections::HashMap::new();
    let parent = parent_index(&source, last_index, m, &mut cache)
        .ok_or_else(|| format!("Last column has no {}-parent", m))?;

    let parent_column = source[parent];
    let mut d: ConvColumn = [0; 3];
    for r in 0..3 {
        d[r] = if r >= m - 1 { 0 } else { last_column[r] - parent_column[r] };
    }
    let k = highest_non_zero_row(&d);
    let prefix = source[..last_index].to_vec();

    if number == 0 {
        return Ok(prefix);
    }
    if number == 1 {
        let mut r = prefix;
        r.push([parent_column[0] + d[0], parent_column[1] + d[1], parent_column[2] + d[2]]);
        return Ok(r);
    }

    let base = source[parent..last_index].to_vec();

    let correction: ConvMatrix;
    if system == ConvSystem::Bms || k <= 1 {
        correction = (parent..last_index)
            .map(|pos| {
                let mut values = [0i32; 3];
                for row in 0..3 {
                    let active = row + 1 <= k && (pos == parent || is_ancestor(&source, parent, pos, row + 1, &mut cache));
                    values[row] = if active { d[row] } else { 0 };
                }
                values
            })
            .collect();
    } else {
        let h = 2 * source.iter().flat_map(|c| c.iter()).copied().max().unwrap_or(0);

        let mut targets: std::collections::HashMap<usize, ConvMatrix> = std::collections::HashMap::new();
        for level in 2..=k {
            let z = child_above_parent(&source, last_index, level, parent, &mut cache)
                .ok_or_else(|| format!("Could not define Y_{}", level))?;
            let y_prime = source[z..].to_vec();
            let mut d_i: ConvMatrix = Vec::new();
            for pos in z..source.len() {
                let mut values = [0i32; 3];
                for row in 0..3 {
                    let active = row + 1 < level && (pos == z || is_ancestor(&source, z, pos, row + 1, &mut cache));
                    values[row] = if active { h - source[z][row] } else { 0 };
                }
                d_i.push(values);
            }
            targets.insert(level, add_matrices(&y_prime, &d_i));
        }

        let full_length = source.len() - parent;
        let mut vectors: Vec<Vec<i32>> = vec![Vec::new(); k + 1];
        vectors[1] = vec![1; full_length];

        for level in 2..=k {
            let mut vector = vec![0i32; full_length];
            vector[0] = 1;
            vector[full_length - 1] = 1;
            for local_pos in 1..full_length - 1 {
                let pos = parent + local_pos;
                if !is_ancestor(&source, parent, pos, level, &mut cache) {
                    continue;
                }
                if vectors[level - 1][local_pos] == 0 {
                    continue;
                }
                let Some(z_prime) = child_above_parent(&source, pos, level, parent, &mut cache) else {
                    continue;
                };
                let x_prime = source[z_prime..].to_vec();
                let mut d_z: ConvMatrix = Vec::new();
                for matrix_pos in z_prime..source.len() {
                    let from_end = source.len() - matrix_pos;
                    let mut values = [0i32; 3];
                    for row in 0..3 {
                        let active = row + 1 < level && {
                            let v = &vectors[row + 1];
                            let vlen = v.len();
                            vlen > from_end && v[vlen - from_end - 1] == 1
                        };
                        values[row] = if active { h - source[z_prime][row] } else { 0 };
                    }
                    d_z.push(values);
                }
                let x_value = add_matrices(&x_prime, &d_z);
                let target = targets.get(&level).unwrap();
                vector[local_pos] = if conv_matrix_compare(&x_value, target) < 0 { 0 } else { 1 };
            }
            vectors[level] = vector;
        }

        correction = (0..base.len())
            .map(|local_pos| {
                let mut values = [0i32; 3];
                for row in 0..3 {
                    values[row] = if row + 1 <= k { d[row] * vectors[row + 1][local_pos] } else { 0 };
                }
                values
            })
            .collect();
    }

    let block = add_matrices(&base, &correction);
    let mut result = prefix.clone();
    result.extend(block);
    for factor in 2..number {
        result.extend(add_matrices(&base, &scale_matrix(factor, &correction)));
    }
    Ok(result)
}

// ── UPMS → BMS rewrite ──

fn upms_to_bms_rewrite(matrix: &ConvMatrix) -> Result<ConvMatrix, String> {
    let mut current = matrix.clone();
    let mut pointer = 0usize;
    let mut loop_guard = 0usize;

    loop {
        let cur_col = column_at(&current, pointer);
        if cur_col == EMPTY_COLUMN {
            break;
        }
        loop_guard += 1;
        if loop_guard > 100_000 {
            return Err("UPMS → BMS pointer loop".to_string());
        }

        if current[pointer][2] == 1 {
            pointer += 1;
            continue;
        }

        let x = current[pointer];
        let a = x[0];
        let b = x[1];
        let low: ConvColumn = [a + 1, b + 1, 1];
        let case1_pattern: [ConvColumn; 4] = [
            low,
            [a + 2, b, 0],
            [a + 3, b + 1, 1],
            [a + 4, b + 1, 0],
        ];

        let offsets: [ConvColumn; 4] = [
            column_at(&current, pointer + 1),
            column_at(&current, pointer + 2),
            column_at(&current, pointer + 3),
            column_at(&current, pointer + 4),
        ];
        if offsets == case1_pattern {
            let high: ConvColumn = [a + 3, b + 1, 0];
            let following_pair: [ConvColumn; 2] = [[a + 4, b + 2, 1], [a + 5, b + 1, 0]];

            let mut scan = pointer + 4;
            let x_end: usize;
            loop {
                let col = column_at(&current, scan);
                let seq_lt = sequence_compare(&col, &high) < 0;
                let eq_high_then_pair_lt = col == high
                    && conv_matrix_compare(
                        &vec![column_at(&current, scan + 1), column_at(&current, scan + 2)],
                        &following_pair.to_vec(),
                    ) < 0;
                if seq_lt || eq_high_then_pair_lt {
                    x_end = scan - 1;
                    break;
                }
                scan += 1;
            }

            scan = x_end + 1;
            while sequence_compare(&column_at(&current, scan), &low) >= 0 {
                scan += 1;
            }
            let y_end = scan - 1;

            let x_block = current[pointer + 1..=x_end].to_vec();
            let y_block = current[x_end + 1..=y_end].to_vec();
            let mut z_block: ConvMatrix = Vec::new();
            let mut insert_z = false;

            if !y_block.is_empty() {
                let prefix_part = current[pointer..=y_end].to_vec();
                let mut fs_input = prefix_part.clone();
                fs_input.push(low);
                let expanded = conv_fundamental_sequence(&fs_input, 2, ConvSystem::Upms)?;
                if expanded[..prefix_part.len()] != prefix_part[..] {
                    return Err("FS(A,2) did not preserve xXY".to_string());
                }
                z_block = expanded[prefix_part.len()..].to_vec();
                let mut comp = expanded.clone();
                comp.push([a + 2, 0, 0]);
                insert_z = conv_matrix_compare(&current[pointer..].to_vec(), &comp) < 0;
            }

            let x_prime: ConvMatrix = x_block[1..]
                .iter()
                .map(|c| [c[0] - 2, c[1], c[2]])
                .collect();
            let mut replacement = x_prime;
            replacement.extend(if insert_z { z_block } else { y_block });
            let mut new_current: ConvMatrix = current[..pointer].to_vec();
            new_current.extend(replacement);
            new_current.extend(current[y_end + 1..].to_vec());
            current = new_current;
            continue;
        }

        let case2 = column_at(&current, pointer + 1) == low
            && column_at(&current, pointer + 2) == [a + 2, b + 1, 0]
            && sequence_compare(&column_at(&current, pointer + 3), &low) >= 0;

        if case2 {
            let mut scan = pointer + 2;
            while sequence_compare(&column_at(&current, scan), &low) >= 0 {
                scan += 1;
            }
            let x_end = scan - 1;
            let x_x = current[pointer..=x_end].to_vec();
            let mut fs_input = x_x.clone();
            fs_input.push(low);
            let expanded = conv_fundamental_sequence(&fs_input, 2, ConvSystem::Upms)?;
            if expanded[..x_x.len()] != x_x[..] {
                return Err("FS(A,2) did not preserve xX".to_string());
            }
            let y_block = expanded[x_x.len()..].to_vec();
            let mut comp = expanded.clone();
            comp.push([a + 2, 0, 0]);
            let insert_y = conv_matrix_compare(&current[pointer..].to_vec(), &comp) < 0;
            let tail = current[x_end + 1..].to_vec();
            let mut new_current: ConvMatrix = current[..pointer].to_vec();
            new_current.extend(x_x[..3.min(x_x.len())].to_vec());
            if insert_y {
                new_current.extend(y_block);
            }
            new_current.extend(tail);
            current = new_current;
            pointer += 1;
            continue;
        }

        pointer += 1;
    }

    Ok(current)
}

// ── Squeeze search ──

fn smallest_squeezing_index(y: &ConvMatrix, x: &ConvMatrix, cap: usize) -> Result<usize, String> {
    let converted = |index: i32| -> Result<ConvMatrix, String> {
        let candidate = conv_fundamental_sequence(y, index, ConvSystem::Upms)?;
        upms_to_bms_raw(&candidate)
    };

    if conv_matrix_compare(&converted(0)?, x) >= 0 {
        return Ok(0);
    }

    let mut lower = 0usize;
    let mut upper = 1usize;
    while upper <= cap && conv_matrix_compare(&converted(upper as i32)?, x) < 0 {
        lower = upper;
        if upper == cap {
            break;
        }
        upper = (cap).min(upper * 2);
    }

    if conv_matrix_compare(&converted(upper as i32)?, x) < 0 {
        for i in 0..=cap {
            if conv_matrix_compare(&converted(i as i32)?, x) >= 0 {
                return Ok(i);
            }
        }
        return Err(format!("No n in 0..{} satisfies U2B(y[n]) >= x", cap));
    }

    let mut low = lower + 1;
    let mut high = upper;
    while low < high {
        let mid = (low + high) >> 1;
        if conv_matrix_compare(&converted(mid as i32)?, x) >= 0 {
            high = mid;
        } else {
            low = mid + 1;
        }
    }

    if low > 0 && conv_matrix_compare(&converted(low as i32 - 1)?, x) >= 0 {
        for i in lower + 1..=low {
            if conv_matrix_compare(&converted(i as i32)?, x) >= 0 {
                return Ok(i);
            }
        }
    }
    Ok(low)
}

fn upms_to_bms_raw(matrix: &ConvMatrix) -> Result<ConvMatrix, String> {
    if *matrix == upms_boundary() {
        return Ok(bms_boundary());
    }
    upms_to_bms_rewrite(matrix)
}

fn bms_to_upms_squeeze(matrix: &ConvMatrix) -> Result<ConvMatrix, String> {
    let x = matrix;
    if *x == bms_boundary() {
        return Ok(upms_boundary());
    }
    if conv_matrix_compare(x, &bms_boundary()) > 0 {
        return Err("BMS input above boundary".to_string());
    }

    let mut y: ConvMatrix = upms_boundary();
    let mut visited: std::collections::HashSet<ConvMatrix> = std::collections::HashSet::new();
    let max_iter = 1000.max(50 * (x.len() + 1));

    for _ in 0..max_iter {
        if !visited.insert(y.clone()) {
            return Err("Squeeze search entered a cycle".to_string());
        }

        let converted_y = upms_to_bms_raw(&y)?;
        if converted_y == *x {
            return Ok(y);
        }
        if conv_matrix_compare(&converted_y, x) < 0 {
            return Err("Squeeze descended below target".to_string());
        }

        let last_is_zero = y.last().map(|c| c[0] == 0 && c[1] == 0 && c[2] == 0).unwrap_or(false);
        if last_is_zero {
            let candidate = y[..y.len() - 1].to_vec();
            let converted_candidate = upms_to_bms_raw(&candidate)?;
            if conv_matrix_compare(&converted_candidate, x) < 0 {
                return Err("BMS target between successor and predecessor".to_string());
            }
            y = candidate;
            continue;
        }

        let cap = 5 * x.len();
        let index = smallest_squeezing_index(&y, x, cap)?;
        let candidate = conv_fundamental_sequence(&y, index as i32, ConvSystem::Upms)?;
        if candidate == y {
            return Err("Squeeze step did not decrease".to_string());
        }
        y = candidate;
    }

    Err("Squeeze exceeded iteration limit".to_string())
}

// ════════════════════════════════════════════════════════════════
// Public conversion API
// ════════════════════════════════════════════════════════════════

fn matrix_to_conv(matrix: &Matrix) -> ConvMatrix {
    standardize_matrix(matrix)
        .iter()
        .map(|col| [col.first().copied().unwrap_or(0), col.get(1).copied().unwrap_or(0), col.get(2).copied().unwrap_or(0)])
        .collect()
}

fn conv_to_matrix(matrix: &ConvMatrix) -> Matrix {
    matrix.iter().map(|c| vec![c[0], c[1], c[2]]).collect()
}

/// Convert UPMS expression to BMS. Input/output are column-major.
pub fn upms_to_bms(input: &Matrix) -> Result<Matrix, String> {
    let source = standardize_matrix(input);
    if source.iter().all(|col| col.len() <= 2 || col[2] == 0) {
        return Ok(source);
    }
    let conv_matrix = matrix_to_conv(&source);
    if conv_matrix_compare(&conv_matrix, &upms_boundary()) > 0 {
        return Err("UPMS input above boundary 0 111 211".to_string());
    }
    let result = upms_to_bms_raw(&conv_matrix)?;
    let round_trip = bms_to_upms_squeeze(&result)?;
    if round_trip != conv_matrix {
        return Err("Converted BMS failed standardness round trip".to_string());
    }
    Ok(conv_to_matrix(&result))
}

/// Convert BMS expression to UPMS. Input/output are column-major.
pub fn bms_to_upms(input: &Matrix) -> Result<Matrix, String> {
    let source = standardize_matrix(input);
    if source.iter().all(|col| col.len() <= 2 || col[2] == 0) {
        return Ok(source);
    }
    let conv_matrix = matrix_to_conv(&source);
    let result = bms_to_upms_squeeze(&conv_matrix)?;
    let round_trip = upms_to_bms_raw(&result)?;
    if round_trip != conv_matrix {
        return Err("Converted UPMS failed standardness round trip".to_string());
    }
    Ok(conv_to_matrix(&result))
}

// ════════════════════════════════════════════════════════════════
// Parsing & formatting
// ════════════════════════════════════════════════════════════════

/// Parse a UPMS expression string ("0 11" or "(0)(1,1)" style) into a matrix.
pub fn parse_upms(input: &str) -> Result<Matrix, String> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut normalized = input.trim().to_string();
    if normalized.contains('(') && normalized.contains(')') {
        normalized = normalized.replace(")(", ") (");
    }
    let mut result: Matrix = Vec::new();
    for raw in normalized.split_whitespace() {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        let parts: Vec<i32>;
        if token.starts_with('(') && token.ends_with(')') {
            let inner = &token[1..token.len() - 1];
            if inner.is_empty() {
                parts = Vec::new();
            } else {
                let mut parsed = Vec::new();
                for s in inner.split(',') {
                    let v = s.trim().parse::<i32>()
                        .map_err(|_| format!("Invalid UPMS column: {}", token))?;
                    parsed.push(v);
                }
                parts = parsed;
            }
        } else if token.chars().all(|c| c.is_ascii_digit()) {
            parts = token.chars().map(|c| c.to_digit(10).unwrap() as i32).collect();
        } else {
            return Err(format!("Invalid UPMS column: {}", token));
        }
        if parts.len() > 3 {
            return Err(format!("Column {} has more than 3 entries", token));
        }
        let mut col = parts;
        while col.len() < 3 {
            col.push(0);
        }
        result.push(col);
    }
    Ok(result)
}

/// Format a matrix as a compact UPMS string ("0 11" style).
pub fn format_upms(matrix: &Matrix) -> String {
    matrix
        .iter()
        .map(|col| {
            let mut values = col.clone();
            while values.len() > 1 && values[values.len() - 1] == 0 {
                values.pop();
            }
            if values.iter().any(|&v| v < 0 || v > 9) {
                format!("({})", col.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
            } else {
                values.iter().map(|v| v.to_string()).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
