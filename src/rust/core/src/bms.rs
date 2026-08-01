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
