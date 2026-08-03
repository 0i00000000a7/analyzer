//! BMS → BOCF conversion.

use crate::term::*;
use crate::Matrix;

pub fn ebo() -> Matrix {
    vec![
        vec![0, 0, 0],
        vec![1, 1, 1],
        vec![2, 1, 1],
        vec![3, 1, 0],
        vec![2, 0, 0],
    ]
}

/// Per-call caches (mirrors C++ file-scope statics, lifecycle = one bms_to_bocf call).
pub struct BmsContext {
    parent: Vec<Vec<i32>>,
    parent_ready: bool,
    children: Vec<Vec<usize>>,
    upgrader: Vec<i32>,
    column: Vec<Term>,
    column_cached: Vec<bool>,
    index: Vec<Term>,
    index_cached: Vec<bool>,
}

impl BmsContext {
    pub fn new() -> Self {
        BmsContext {
            parent: Vec::new(),
            parent_ready: false,
            children: Vec::new(),
            upgrader: Vec::new(),
            column: Vec::new(),
            column_cached: Vec::new(),
            index: Vec::new(),
            index_cached: Vec::new(),
        }
    }

    fn build_parent_cache(&mut self, m: &Matrix) {
        let l = m.len();
        let rows = m.iter().map(|c| c.len()).max().unwrap_or(0);

        // Pad matrix columns to uniform row count
        let mut s: Vec<Vec<i32>> = Vec::with_capacity(l);
        for i in 0..l {
            let mut col = m[i].clone();
            while col.len() < rows {
                col.push(0);
            }
            s.push(col);
        }

        self.parent = vec![Vec::new(); l];
        for row in 0..rows {
            if row == 0 {
                // Row 0: monotonic stack — O(cols)
                let mut stack: Vec<usize> = Vec::new();
                for col in 0..l {
                    while let Some(&back) = stack.last() {
                        if s[back][0] >= s[col][0] {
                            stack.pop();
                        } else {
                            break;
                        }
                    }
                    self.parent[col].push(if stack.is_empty() { -1 } else { stack.last().unwrap().clone() as i32 });
                    stack.push(col);
                }
            } else {
                // Rows > 0: follow parent chain from row above
                for col in 0..l {
                    let mut k = col as i32;
                    while k >= 0 && s[k as usize][row] >= s[col][row] {
                        k = self.parent[k as usize][row - 1];
                    }
                    self.parent[col].push(k);
                }
            }
        }
        self.parent_ready = true;

        // Build children cache from row-0 parents
        self.children = vec![Vec::new(); l];
        for i in 0..l {
            let p = self.parent[i][0];
            if p >= 0 {
                self.children[p as usize].push(i);
            }
        }
    }

    fn find_parent(&self, m: &Matrix, find_row: i32, relative_column: usize) -> i32 {
        if self.parent_ready {
            if find_row == -1 {
                return relative_column as i32 - 1;
            }
            return self.parent[relative_column][find_row as usize];
        }
        // Fallback (no cache)
        if find_row == -1 {
            return relative_column as i32 - 1;
        }
        let mut cur_column = self.find_parent(m, find_row - 1, relative_column);
        while cur_column > -1 && m[cur_column as usize][find_row as usize] >= m[relative_column][find_row as usize] {
            cur_column = self.find_parent(m, find_row - 1, cur_column as usize);
        }
        cur_column
    }

    fn children(&self, m: &Matrix, n: usize) -> Vec<usize> {
        if self.parent_ready {
            return self.children[n].clone();
        }
        // Fallback (no cache)
        let mut x = Vec::new();
        for i in 0..m.len() {
            if self.find_parent(m, 0, i) == n as i32 {
                x.push(i);
            }
        }
        x
    }

    fn get_upgrader(&mut self, m: &Matrix, n: usize) -> i32 {
        // Check cache
        if n < self.upgrader.len() {
            let cached = self.upgrader[n];
            if cached != -2 {
                return cached;
            }
        }
        // Compute
        let mut result: i32 = -1;
        if m[n].len() < 3 || m[n][1] == 0 || m[n][2] == 1 || n + 1 >= m.len() {
            result = -1;
        } else {
            let p = self.find_parent(m, 1, n);
            if p < 0 || p as usize >= m.len() {
                result = -1;
            } else {
                let pu = p as usize;
                let l = vec![m[pu][0] + 1, m[n][1], m[pu][2] + 1];

                // C++: the first branch only returns on match; otherwise
                // control falls through to the ancestor search below.
                let mut matched = false;
                if self.find_parent(m, 1, n) == self.find_parent(m, 1, n + 1) {
                    let match_ = m[n + 1].len() >= 3 && m[n + 1][0] == l[0] && m[n + 1][1] == l[1] && m[n + 1][2] == l[2];
                    if match_ {
                        result = n as i32 + 1;
                        matched = true;
                    }
                }
                if !matched {
                    let mut q = n as i32;
                    let mut found = -1;
                    loop {
                        q = self.find_parent(m, 0, q as usize);
                        if q == -1 {
                            break;
                        }
                        if self.find_parent(m, 1, n) == self.find_parent(m, 1, q as usize) {
                            let match_ = m[q as usize].len() >= 3 && m[q as usize][0] == l[0] && m[q as usize][1] == l[1] && m[q as usize][2] == l[2];
                            if match_ && n + 1 < m.len() && m[n + 1][0] > m[q as usize][0] {
                                found = q;
                                break;
                            }
                        }
                    }
                    result = found;
                }
            }
        }
        if self.upgrader.is_empty() {
            self.upgrader = vec![-2; m.len()];
        }
        if n >= self.upgrader.len() {
            self.upgrader.resize(n + 1, -2);
        }
        self.upgrader[n] = result;
        result
    }

    fn get_index_of_column(&mut self, m: &Matrix, n: usize) -> Term {
        if m[n].len() < 2 || m[n][1] == 0 {
            return zero();
        }
        if m[n].len() < 3 || m[n][2] == 0 {
            let upgrade_idx = self.get_upgrader(m, n);
            let upgrading_term_adm = if upgrade_idx >= 0 {
                last_term(&self.get_cached_index_of_column(m, upgrade_idx as usize))
            } else {
                one()
            };
            return add(&self.get_cached_index_of_column(m, self.find_parent(m, 1, n) as usize), &upgrading_term_adm);
        }

        let mut omega_power_x_counter = one();
        for i in self.children(m, n) {
            if m[i].len() < 3 {
                continue;
            }
            if !row_eq3(&m[i], m[n][0] + 1, m[n][1], 1) {
                continue;
            }
            let mut q = zero();
            for j in self.children(m, i) {
                q = add(&q, &self.get_cached_not_standard_expr(m, j));
            }
            omega_power_x_counter = add(&omega_power_x_counter, &exp(&q));
        }
        add(&self.get_cached_index_of_column(m, self.find_parent(m, 1, n) as usize), &exp(&omega_power_x_counter))
    }

    fn not_standard_expr_from_column(&mut self, m: &Matrix, n: usize) -> Term {
        let mut omega_multiplication = zero();
        // upgrader section
        let upgrader_snapshot: Vec<i32> = self.upgrader.clone();
        for i in self.children(m, n) {
            if m[i].len() >= 3 && row_eq3(&m[i], m[n][0] + 1, m[n][1], 1) {
                continue;
            }
            // C++ quirk: std::find over the cache checks whether the VALUE i
            // appears anywhere (i.e. i is some column's upgrader).
            let is_upgrader = upgrader_snapshot.contains(&(i as i32));
            if is_upgrader {
                let c = self.children(m, i);
                if !c.is_empty() {
                    let last = *c.last().unwrap();
                    if m[last].len() >= 3 && row_eq3(&m[last], m[i][0] + 1, m[i][1], 1) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            omega_multiplication = add(&omega_multiplication, &self.get_cached_not_standard_expr(m, i));
        }
        t(self.get_cached_index_of_column(m, n), omega_multiplication, zero())
    }

    fn get_cached_not_standard_expr(&mut self, m: &Matrix, n: usize) -> Term {
        if n < self.column_cached.len() && self.column_cached[n] {
            return self.column[n].clone();
        }
        let result = self.not_standard_expr_from_column(m, n);
        if self.column_cached.len() <= n || !self.column_cached[n] {
            if self.column_cached.len() <= n {
                self.column.resize(n + 1, zero());
                self.column_cached.resize(n + 1, false);
            }
            self.column[n] = result.clone();
            self.column_cached[n] = true;
        }
        result
    }

    fn get_cached_index_of_column(&mut self, m: &Matrix, n: usize) -> Term {
        if n < self.index_cached.len() && self.index_cached[n] {
            return self.index[n].clone();
        }
        let result = self.get_index_of_column(m, n);
        if self.index_cached.len() <= n || !self.index_cached[n] {
            if self.index_cached.len() <= n {
                self.index.resize(n + 1, zero());
                self.index_cached.resize(n + 1, false);
            }
            self.index[n] = result.clone();
            self.index_cached[n] = true;
        }
        result
    }

    pub fn bms_to_bocf(&mut self, m: &Matrix) -> Term {
        self.build_parent_cache(m);

        // Precompute upgrader cache for the entire matrix
        let l = m.len();
        self.upgrader = vec![-2; l];
        for x in 0..l {
            self.upgrader[x] = self.get_upgrader(m, x);
        }

        let mut s = zero();
        for i in 0..m.len() {
            if m[i].len() >= 1 && m[i][0] == 0 && (m[i].len() < 2 || m[i][1] == 0) && (m[i].len() < 3 || m[i][2] == 0) {
                s = add(&s, &self.get_cached_not_standard_expr(m, i));
            }
        }
        self.parent_ready = false;
        self.parent.clear();
        self.children.clear();
        self.upgrader.clear();
        self.column.clear();
        self.column_cached.clear();
        self.index.clear();
        self.index_cached.clear();
        standard_form(&s)
    }
}

fn row_eq3(r: &[i32], a: i32, b: i32, c: i32) -> bool {
    r.len() >= 3 && r[0] == a && r[1] == b && r[2] == c
}

/// ψ(I) checks via lexicographic matrix order.
pub fn is_eq_ebo(m: &Matrix) -> bool {
    crate::expand::matrix_lex_order(m, &ebo()) == 0
}

pub fn is_gt_ebo(m: &Matrix) -> bool {
    crate::expand::matrix_lex_order(m, &ebo()) > 0
}

pub fn is_gte_ebo(m: &Matrix) -> bool {
    is_eq_ebo(m) || is_gt_ebo(m)
}

/// Convenience: one-shot BMS → BOCF without managing a context.
pub fn bms_to_bocf(m: &Matrix) -> Term {
    let mut ctx = BmsContext::new();
    ctx.bms_to_bocf(m)
}

// ────────────────────────────────────────────────────────────────
// Standard-form check (chkStd from basmat.c, BM4 only)
// ────────────────────────────────────────────────────────────────

/// BM4 getBadSequence. Returns bad-part length (0 = no bad part), filling
/// `delta` and the C matrix (column-major, `c[col * nr + row]`).
fn get_bad_sequence_bm4(s: &[Vec<i32>], delta: &mut [i32], c: &mut [i32], n: usize, nr: usize) -> usize {
    let row = nr - 1;
    let mut bad = 0usize;

    if s[n][0] == 0 {
        return 0;
    }

    // Clear Delta
    for m in 0..nr {
        delta[m] = 0;
    }
    // Determine the bad sequence and calculate Delta (same as BM2)
    let mut k = 0usize;
    'outer: while k <= n {
        // For each k, we check rows l=0..row. If ANY row fails the < check,
        // we immediately skip to the next k (matching C code's `l = row;`).
        let mut l = 0usize;
        while l <= row {
            if s[n - k][l] < s[n][l] - delta[l] {
                if l == row || (l < row && s[n][l + 1] == 0) {
                    bad = k;
                    break 'outer;
                } else {
                    delta[l] = s[n][l] - s[n - k][l];
                }
                l += 1;
            } else {
                // Row l does NOT satisfy the < condition; skip to next k
                // (equivalent to C code's `l = row;` which exits the for loop)
                break;
            }
        }
        k += 1;
    }
    if bad == 0 {
        return 0;
    }

    // Calculate C matrix (BM4)
    let mut e = vec![0i32; nr];
    let mut l = bad;
    while l >= 2 {
        for m in 0..=row {
            let mut q = 0usize;
            for j in 0..=row {
                e[j] = 0;
            }
            let mut p = usize::MAX;
            let mut n2 = l;
            'mid: while n2 <= bad {
                let mut o = 0usize;
                while o <= m {
                    if s[n - n2][o] < s[n - l + 1][o] - e[o] {
                        // C reads S[(n-l+1)*nr+o+1] even when o == row, spilling
                        // into the next column; short-circuit to keep it in bounds.
                        if o == m || s[n - l + 1][o + 1] == 0 {
                            p = n2;
                            q = o;
                            break 'mid;
                        } else {
                            e[o] = s[n - l + 1][o] - s[n - n2][o];
                        }
                        o += 1;
                    } else {
                        // Row o does NOT satisfy the < condition; skip to next n2
                        break;
                    }
                }
                n2 += 1;
            }
            if p == usize::MAX {
                c[(bad - l + 2) * nr + m] = 0;
            } else if c[(bad - p + 1) * nr + m] == 1 && q == m {
                c[(bad - l + 2) * nr + m] = 1;
            } else {
                c[(bad - l + 2) * nr + m] = 0;
            }
        }
        l -= 1;
    }
    bad
}

/// BM4 copyBadSequence: extend the sequence from column `n` (inclusive) until
/// column `nn` (exclusive) by copying the bad part with Delta·C added.
fn copy_bad_sequence_bm4(s: &mut Vec<Vec<i32>>, delta: &[i32], c: &[i32], n: usize, nn: usize, nr: usize, bad: usize) {
    let mut cur = n;
    let mut m = 1usize;
    while cur < nn {
        let mut col = s[cur - bad].clone();
        for l in 0..nr {
            col[l] += delta[l] * c[l + m * nr];
        }
        if cur < s.len() {
            s[cur] = col;
        } else {
            s.push(col);
        }
        cur += 1;
        m += 1;
        if m > bad {
            m = 1;
        }
    }
}

/// chkStd from basmat.c (BM4). Returns true iff the matrix is standard, i.e.
/// reachable from a BMS limit expression by repeatedly taking fundamental
/// sequences. Detail output follows debug builds (`cfg!(debug_assertions)`).
pub fn is_standard_matrix(m: &Matrix) -> bool {
    is_standard_matrix_impl(m, false)
}

/// Same as `is_standard_matrix` but uses the triangular standard sequence
/// `(0)(1)(2,1)(3,2,1)(4,3,2,1)...` (column i = (i, i-1, …, 1, 0, …))
/// instead of the BMS identity `(0)(1,1)(2,2,2)...`.
pub fn is_standard_triangular_matrix(m: &Matrix) -> bool {
    is_standard_matrix_impl(m, true)
}

fn is_standard_matrix_impl(m: &Matrix, triangular: bool) -> bool {
    let nc = m.len();
    if nc == 0 {
        return true;
    }
    let nr = m.iter().map(|c| c.len()).max().unwrap_or(0);
    // Pad columns to a uniform row count
    let mut s: Vec<Vec<i32>> = Vec::with_capacity(nc);
    for i in 0..nc {
        let mut col = m[i].clone();
        while col.len() < nr {
            col.push(0);
        }
        s.push(col);
    }

    // Check real numbers of row
    let mut row = 0usize;
    for i in 0..nc {
        if row + 1 == nr {
            break;
        }
        for j in (row + 1)..nr {
            if s[i][j] > 0 {
                row = j;
            }
        }
    }

    // Find smaller column p
    let mut p: Option<usize> = None;
    'outer: for i in 0..nc {
        for j in 0..=row {
            let id_val = if triangular {
                if j <= i { (i - j) as i32 } else { 0 }
            } else {
                i as i32
            };
            if s[i][j] > id_val {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "is_standard: not starting from the standard sequence (S[{}][{}]={} > {})",
                        i, j, s[i][j], id_val
                    );
                }
                return false;
            }
            if s[i][j] < id_val {
                p = Some(i);
                break 'outer;
            }
        }
    }
    let p = match p {
        Some(p) => p,
        None => return true,
    };

    if cfg!(debug_assertions) {
        eprintln!("is_standard: checking whether the input lies on the fundamental sequence of the standard sequence above it");
    }

    // Make a sequence above S (SA).
    let mut sa: Vec<Vec<i32>> = Vec::with_capacity(nc * 2);
    for i in 0..=p {
        if triangular {
            // Triangular identity: column i = (i, i-1, ..., 1, 0, ..., 0)
            let mut col = vec![0i32; row + 1];
            for j in 0..=row.min(i) {
                col[j] = (i - j) as i32;
            }
            sa.push(col);
        } else {
            // BMS identity: column i = (i, i, ..., i)
            sa.push(vec![i as i32; row + 1]);
        }
    }
    let mut delta = vec![0i32; row + 1];
    let mut c = vec![0i32; (row + 1) * nc * 2];
    for i in 0..=row {
        c[i + row + 1] = 1; // C column 1 = all 1s
    }

    let mut pp = p;
    loop {
        let bad = get_bad_sequence_bm4(&sa, &mut delta, &mut c, pp, row + 1);
        if bad == 0 {
            if cfg!(debug_assertions) {
                eprintln!("is_standard: not standard (bad sequence exhausted)");
            }
            return false;
        }
        if pp < bad {
            if cfg!(debug_assertions) {
                eprintln!("is_standard: not standard (bad part exceeds prefix)");
            }
            return false;
        }
        let num = (nc + 1 - pp) / bad + 1;
        let nn = pp + bad * num;
        copy_bad_sequence_bm4(&mut sa, &delta, &c, pp, nn, row + 1, bad);

        let mut newp: Option<usize> = None;
        for i in pp..nc {
            let mut smaller = false;
            let mut larger = false;
            for j in 0..=row {
                if s[i][j] > sa[i][j] {
                    larger = true;
                }
                if s[i][j] < sa[i][j] {
                    smaller = true;
                }
            }
            if larger && !smaller {
                if cfg!(debug_assertions) {
                    eprintln!("is_standard: not standard (input exceeds the fundamental sequence at column {})", i);
                }
                return false;
            }
            if larger || smaller {
                newp = Some(i);
                break;
            }
        }
        match newp {
            None => return true,
            Some(n) => pp = n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chkstd_standard() {
        let m = vec![
            vec![0, 0, 0],
            vec![1, 1, 1],
            vec![2, 1, 1],
            vec![3, 1, 0],
            vec![1, 1, 1],
        ];
        assert!(is_standard_matrix(&m));
    }

    #[test]
    fn test_chkstd_non_standard() {
        // (0)(1)(2,1)(3,2,1,1) — not standard triangular BMS
        let m = vec![
            vec![0],
            vec![1],
            vec![2, 1],
            vec![3, 2, 1, 1],
        ];
        assert!(!is_standard_matrix(&m));
    }
}
