//! IHSS Hydra notation (based on the SMS algorithm), with a Mahlo-BOCF
//! LaTeX display. The underlying representation is a parent matrix; the
//! Mahlo BOCF rendering is only a formatting scheme.

/// Parent matrix: `parent[c][r]` = parent column index, or `-1` for none.
pub type IHSSParent = Vec<Vec<i32>>;

pub struct IHSS {
    parent: IHSSParent,
}

impl IHSS {
    pub fn new(parent: IHSSParent) -> Self {
        IHSS { parent }
    }

    // ---------- factories ----------

    pub fn from_string(input: &str) -> Result<Self, String> {
        let matrix = Self::parse(input)?;
        Ok(Self::from_value(&matrix))
    }

    /// Convert a value matrix (BMS-style `(0,0)(1,1)` columns) to a parent matrix.
    pub fn from_value(value: &[Vec<i32>]) -> Self {
        let cols = value.len();
        let rows = value.first().map(|c| c.len()).unwrap_or(0);
        if rows == 0 || cols == 0 {
            return IHSS { parent: Vec::new() };
        }
        let mut parent: IHSSParent = vec![vec![-1; rows]; cols];
        let mut virtual_parent = vec![-1; cols];
        for c in 1..cols {
            virtual_parent[c] = (c - 1) as i32;
        }

        let get_ancestors = |col: usize, row: usize, parent: &IHSSParent| -> Vec<usize> {
            let mut ancestors = Vec::new();
            let mut p = parent[col][row];
            while p != -1 {
                ancestors.push(p as usize);
                p = parent[p as usize][row];
            }
            ancestors
        };

        for r in 0..rows {
            for c in 0..cols {
                let below_ancestors: Vec<usize> = if r == 0 {
                    let mut list = Vec::new();
                    let mut p = virtual_parent[c];
                    while p != -1 {
                        list.push(p as usize);
                        p = virtual_parent[p as usize];
                    }
                    list
                } else {
                    get_ancestors(c, r - 1, &parent)
                };
                let cur_val = value[c][r];
                let mut best: i32 = -1;
                for &anc in &below_ancestors {
                    if value[anc][r] < cur_val {
                        best = anc as i32;
                        break;
                    }
                }
                parent[c][r] = best;
            }
        }
        IHSS { parent }
    }

    pub fn from_worm(worm: &[i32]) -> Self {
        let n = worm.len();
        if n == 0 {
            return IHSS { parent: Vec::new() };
        }
        let mut aux: Vec<Vec<i32>> = vec![worm.to_vec()];
        let mut parent_rows: Vec<Vec<i32>> = Vec::new();

        let mut row0: Vec<i32> = Vec::with_capacity(n);
        for c in 0..n {
            let mut best: i32 = -1;
            for p in (0..c).rev() {
                if aux[0][p] < aux[0][c] {
                    best = p as i32;
                    break;
                }
            }
            row0.push(best);
        }
        parent_rows.push(row0);

        let mut i = 0;
        loop {
            let row = &parent_rows[i];
            let mut aux_next: Vec<i32> = Vec::with_capacity(n);
            for c in 0..n {
                if row[c] != -1 {
                    aux_next.push(aux[i][c] - aux[i][row[c] as usize]);
                } else {
                    aux_next.push(1);
                }
            }
            aux.push(aux_next);
            if aux[i + 1].iter().all(|&v| v == 1) {
                break;
            }

            let mut next_row: Vec<i32> = Vec::with_capacity(n);
            for c in 0..n {
                let mut ancestors = Vec::new();
                let mut p = parent_rows[i][c];
                while p != -1 {
                    ancestors.push(p as usize);
                    p = parent_rows[i][p as usize];
                }
                let mut max_col: i32 = -1;
                for &anc in &ancestors {
                    if aux[i + 1][anc] < aux[i + 1][c] {
                        if (anc as i32) > max_col {
                            max_col = anc as i32;
                        }
                    }
                }
                next_row.push(max_col);
            }
            parent_rows.push(next_row);
            i += 1;
        }

        let parent_cols: IHSSParent = (0..n)
            .map(|c| parent_rows.iter().map(|row| row[c]).collect())
            .collect();
        IHSS { parent: parent_cols }
    }

    // ---------- parsing & formatting ----------

    /// Parse a `(0,0)(1,1)(2,1)` value-matrix string into columns.
    pub fn parse(input: &str) -> Result<Vec<Vec<i32>>, String> {
        let bytes = input.as_bytes();
        let mut columns: Vec<Vec<i32>> = Vec::new();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] != b'(' {
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut depth = 1;
            i += 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'(' {
                    depth += 1;
                } else if bytes[i] == b')' {
                    depth -= 1;
                }
                i += 1;
            }
            let content = &input[start..i - 1];
            if content.trim().is_empty() {
                columns.push(vec![0]);
            } else {
                let nums: Vec<i32> = content
                    .split(',')
                    .map(|s| s.trim().parse::<i32>().unwrap_or(0))
                    .collect();
                columns.push(nums);
            }
        }
        if columns.is_empty() {
            return Err("IHSS input must contain at least one (…) column".to_string());
        }
        let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        for col in columns.iter_mut() {
            while col.len() < max_rows {
                col.push(0);
            }
        }
        Ok(columns)
    }

    pub fn to_value(&self) -> Vec<Vec<i32>> {
        let mut value = self.parent.clone();
        for c in 0..self.parent.len() {
            for r in 0..self.parent[c].len() {
                let p = self.parent[c][r];
                if p == -1 {
                    value[c][r] = 0;
                } else {
                    value[c][r] = value[p as usize][r] + 1;
                }
            }
        }
        value
    }

    pub fn format(&self) -> String {
        Self::format_matrix(&self.to_value())
    }

    pub fn format_matrix(matrix: &[Vec<i32>]) -> String {
        if matrix.is_empty() {
            return String::new();
        }
        matrix
            .iter()
            .map(|col| {
                let mut trimmed = col.clone();
                while trimmed.len() > 1 && *trimmed.last().unwrap() == 0 {
                    trimmed.pop();
                }
                format!("({})", trimmed.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
            })
            .collect()
    }

    pub fn to_worm(&self) -> Vec<i32> {
        let parent = &self.parent;
        let cols = parent.len();
        if cols == 0 {
            return Vec::new();
        }
        let rows = parent[0].len();
        let mut val: Vec<Vec<i32>> = vec![vec![0; rows]; cols];
        let r_top = rows - 1;
        for c in 0..cols {
            let p = parent[c][r_top];
            if p == -1 {
                val[c][r_top] = 1;
            } else {
                val[c][r_top] = val[p as usize][r_top] + 1;
            }
        }
        for r in (0..r_top).rev() {
            for c in 0..cols {
                let p = parent[c][r];
                if p == -1 {
                    val[c][r] = 1;
                } else {
                    val[c][r] = val[p as usize][r] + val[c][r + 1];
                }
            }
        }
        (0..cols).map(|c| val[c][0]).collect()
    }

    pub fn is_limit(&self) -> bool {
        let parent = &self.parent;
        if parent.is_empty() {
            return false;
        }
        let last = parent.len() - 1;
        (0..parent[last].len()).any(|r| parent[last][r] != -1)
    }

    // ---------- core algorithm (SMS / IHSS) ----------

    fn get_generation_column(col_idx: usize, lnz_row: usize, parent: &IHSSParent, last_col: usize) -> Vec<i32> {
        let mut result = parent[last_col].clone();
        let p = parent[last_col][lnz_row];
        if p != -1 {
            result[lnz_row] = parent[p as usize][lnz_row];
        }
        for r in (lnz_row + 1)..result.len() {
            result[r] = parent[col_idx][r];
        }
        result
    }

    fn ancestors_at(&self, col: usize, row: usize) -> Vec<usize> {
        let mut ancestors = Vec::new();
        let mut p = self.parent[col][row];
        while p != -1 {
            ancestors.push(p as usize);
            p = self.parent[p as usize][row];
        }
        ancestors
    }

    fn trial_expand(&self, ref_col: usize, lnz_row: usize, last_col: usize, last_allow: &[std::collections::BTreeSet<usize>]) -> IHSSParent {
        let parent = &self.parent;
        let mut new_mat = parent.clone();
        new_mat.pop();
        let gen_col = Self::get_generation_column(ref_col, lnz_row, parent, last_col);
        let copy_width = (last_col - ref_col) as i32;

        if ref_col <= last_col {
            for c in ref_col..=last_col {
                let source_col = parent[c].clone();
                let mut new_col = Vec::with_capacity(source_col.len());
                for r in 0..source_col.len() {
                    let p = source_col[r];
                    let use_gen_col = if r <= lnz_row {
                        p == parent[ref_col][r] && last_allow[r].contains(&c)
                    } else {
                        c == ref_col
                    };
                    if use_gen_col {
                        new_col.push(gen_col[r]);
                    } else {
                        new_col.push(if p >= ref_col as i32 { p + copy_width } else { p });
                    }
                }
                new_mat.push(new_col);
            }
        }
        new_mat
    }

    fn compare_parent_matrices(a: &IHSSParent, b: &IHSSParent) -> i32 {
        let max_cols = a.len().max(b.len());
        for c in 0..max_cols {
            let col_a: &[i32] = if c < a.len() { &a[c] } else { &[] };
            let col_b: &[i32] = if c < b.len() { &b[c] } else { &[] };
            let max_r = col_a.len().max(col_b.len());
            for r in 0..max_r {
                let p_a = if r < col_a.len() { col_a[r] } else { -1 };
                let p_b = if r < col_b.len() { col_b[r] } else { -1 };
                if p_a != p_b {
                    if p_a == -1 {
                        return -1;
                    }
                    if p_b == -1 {
                        return 1;
                    }
                    return p_a - p_b;
                }
            }
        }
        (a.len() - b.len()) as i32
    }

    /// Expand the sequence `times` times. `k` and `pending_mode` mirror the
    /// SMS algorithm defaults (`k=2`, pending mode `HMS`).
    pub fn expand(&self, times: i32, k: i32, pending_mode: &str) -> (IHSS, i32) {
        let parent = &self.parent;
        let cols = parent.len();
        if cols == 0 {
            return (IHSS { parent: Vec::new() }, -1);
        }
        let rows = parent[0].len();
        let last_col = cols - 1;

        let mut lnz_row: i32 = -1;
        for r in (0..rows).rev() {
            if parent[last_col][r] != -1 {
                lnz_row = r as i32;
                break;
            }
        }
        if lnz_row == -1 {
            return (IHSS { parent: parent[..last_col].to_vec() }, -1);
        }

        let original_root = parent[last_col][lnz_row as usize];
        if original_root == -1 {
            return (IHSS { parent: parent.clone() }, -1);
        }

        let mut orig_elem_row: i32 = -1;
        for r in (0..rows).rev() {
            if parent[original_root as usize][r] != -1 {
                orig_elem_row = r as i32;
                break;
            }
        }
        if orig_elem_row == -1 || orig_elem_row < lnz_row {
            orig_elem_row = lnz_row;
        }

        let use_trial = lnz_row >= k - 1;

        let mut bad_root = original_root;

        let compute_s = |c: usize, r: usize| -> std::collections::BTreeSet<usize> {
            let mut ancestors = Vec::new();
            let mut p = parent[c][r];
            while p != -1 {
                ancestors.push(p as usize);
                p = parent[p as usize][r];
            }
            if ancestors.is_empty() && parent[c][r] == -1 {
                return std::collections::BTreeSet::new();
            }
            let mut s = std::collections::BTreeSet::new();
            for &a in &ancestors {
                s.insert(a);
            }
            let direct_parent = if ancestors.is_empty() { -1 } else { ancestors[0] as i32 };
            for &a in &ancestors {
                if a as i32 == direct_parent {
                    continue;
                }
                for col in 0..cols {
                    if parent[col][r] == a as i32 {
                        s.insert(col);
                    }
                }
            }
            for col in 0..cols {
                if parent[col][r] == -1 {
                    s.insert(col);
                }
            }
            s
        };

        let mut allowable: Vec<Vec<std::collections::BTreeSet<usize>>> = vec![Vec::new(); cols];
        for c in 0..cols {
            allowable[c] = vec![std::collections::BTreeSet::new(); rows];
        }
        for r in 0..rows {
            for c in 0..cols {
                if parent[c][r] == -1 {
                    allowable[c][r] = std::collections::BTreeSet::new();
                } else {
                    let s = compute_s(c, r);
                    if r == 0 {
                        allowable[c][r] = s;
                    } else {
                        let prev = &allowable[c][r - 1];
                        let intersect: std::collections::BTreeSet<usize> =
                            s.iter().copied().filter(|x| prev.contains(x)).collect();
                        allowable[c][r] = intersect;
                    }
                }
            }
        }

        let last_allow: Vec<std::collections::BTreeSet<usize>> =
            (0..rows).map(|r| allowable[last_col][r].clone()).collect();

        if use_trial {
            let orig_root_trial = self.trial_expand(original_root as usize, lnz_row as usize, last_col, &last_allow);

            let cond1: std::collections::BTreeSet<usize> = allowable[last_col][lnz_row as usize].clone();

            let gen_col = Self::get_generation_column(original_root as usize, lnz_row as usize, parent, last_col);
            let mut cond3_cols: Vec<usize> = Vec::new();
            for c in 0..cols {
                if c == last_col {
                    continue;
                }
                let gen_col_c = Self::get_generation_column(c, lnz_row as usize, parent, last_col);
                let mut contains = true;
                for r in 0..rows {
                    let a = gen_col_c[r];
                    let b = gen_col[r];
                    if a == -1 || a == b {
                        continue;
                    }
                    let mut is_ancestor = false;
                    let mut pp = b;
                    while pp != -1 {
                        pp = parent[pp as usize][r];
                        if pp == a {
                            is_ancestor = true;
                            break;
                        }
                    }
                    if !is_ancestor {
                        contains = false;
                        break;
                    }
                }
                if contains {
                    cond3_cols.push(c);
                }
            }
            let set3: std::collections::BTreeSet<usize> = cond3_cols.into_iter().collect();

            let mut candidate_set: std::collections::BTreeSet<usize> =
                cond1.iter().copied().filter(|c| set3.contains(c)).collect();
            candidate_set.insert(original_root as usize);
            let candidate_roots: Vec<usize> = candidate_set.iter().copied().collect();

            let candidate_set_for_pending: std::collections::BTreeSet<usize> = candidate_roots.iter().copied().collect();
            let mut pending_set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

            match pending_mode {
                "ISMS" => {
                    for a in self.ancestors_at(last_col, lnz_row as usize) {
                        pending_set.insert(a);
                    }
                }
                _ => {
                    // default ("HMS")
                    let grand_parent = parent[original_root as usize][orig_elem_row as usize];
                    if grand_parent == -1 {
                        for c in 0..cols {
                            if parent[c][orig_elem_row as usize] == -1 {
                                pending_set.insert(c);
                            }
                        }
                    } else {
                        for c in 0..cols {
                            if parent[c][orig_elem_row as usize] == grand_parent {
                                pending_set.insert(c);
                            }
                        }
                    }
                }
            }

            let final_pending: std::collections::BTreeSet<usize> = pending_set
                .iter()
                .copied()
                .filter(|c| candidate_set_for_pending.contains(c))
                .collect();
            let pending_roots: Vec<i32> = final_pending.iter().copied().map(|v| v as i32).collect();

            let mut small_root: i32 = -1;
            for &cr in candidate_roots.iter().rev() {
                if cr == original_root as usize {
                    continue;
                }
                let trial = self.trial_expand(cr, lnz_row as usize, last_col, &last_allow);
                let cmp = Self::compare_parent_matrices(&trial, &orig_root_trial);
                if cmp < 0 {
                    small_root = cr as i32;
                    break;
                }
            }

            if small_root != -1 {
                let mut min_right = i32::MAX;
                for &pr in &pending_roots {
                    if pr > small_root && pr < min_right {
                        min_right = pr;
                    }
                }
                bad_root = if min_right != i32::MAX {
                    min_right
                } else if !pending_roots.is_empty() {
                    *pending_roots.iter().min().unwrap()
                } else {
                    -1
                };
            } else {
                bad_root = if !pending_roots.is_empty() {
                    *pending_roots.iter().min().unwrap()
                } else {
                    -1
                };
            }
            if bad_root == -1 {
                bad_root = original_root;
            }
        }

        let final_gen_col = Self::get_generation_column(bad_root as usize, lnz_row as usize, parent, last_col);

        let mut new_parent = parent.clone();
        new_parent.pop();
        let copy_width = (last_col - bad_root as usize) as i32;

        if bad_root as usize <= last_col - 1 {
            for t in 1..=times {
                for c in bad_root as usize..last_col {
                    let source_col = parent[c].clone();
                    let mut new_col = Vec::with_capacity(source_col.len());
                    for r in 0..source_col.len() {
                        let p = source_col[r];
                        let use_gen_col = if r <= lnz_row as usize {
                            p == parent[bad_root as usize][r] && last_allow[r].contains(&c)
                        } else {
                            c == bad_root as usize
                        };
                        if use_gen_col {
                            new_col.push(if final_gen_col[r] >= bad_root {
                                final_gen_col[r] + (t - 1) * copy_width
                            } else {
                                final_gen_col[r]
                            });
                        } else {
                            new_col.push(if p >= bad_root { p + t * copy_width } else { p });
                        }
                    }
                    new_parent.push(new_col);
                }
            }
        }

        (IHSS { parent: new_parent }, bad_root)
    }

    // ---------- Mahlo BOCF LaTeX rendering ----------

    pub fn to_latex(&self) -> String {
        Self::render_latex(&self.parent)
    }

    fn render_latex(parent: &IHSSParent) -> String {
        if parent.is_empty() {
            return String::new();
        }
        let mut working = parent.clone();
        let rows = working[0].len();
        if rows == 0 {
            return String::new();
        }
        if rows == 1 {
            for col in working.iter_mut() {
                col.push(-1);
            }
        }

        let roots = Self::find_roots(&working);
        if roots.is_empty() {
            return String::new();
        }

        if roots.len() > 1 {
            let mut parts: Vec<String> = Vec::new();
            for &root in &roots {
                let sub_parent = Self::extract_subtree(&working, root);
                if sub_parent.is_empty() {
                    continue;
                }
                if let Ok(result) = Self::render_single_tree(&sub_parent, 0) {
                    parts.push(result);
                }
            }
            if parts.is_empty() {
                return String::new();
            }
            return Self::merge_parts(&parts).join(" + ");
        }

        Self::render_single_tree(&working, roots[0]).unwrap_or_default()
    }

    fn find_roots(parent: &IHSSParent) -> Vec<usize> {
        (0..parent.len()).filter(|&c| parent[c][0] == -1).collect()
    }

    fn get_children(parent: &IHSSParent, node: usize) -> Vec<usize> {
        let mut children: Vec<usize> = (0..parent.len())
            .filter(|&c| c != node && parent[c][0] == node as i32)
            .collect();
        children.sort_unstable();
        children
    }

    fn extract_subtree(parent: &IHSSParent, root: usize) -> IHSSParent {
        if parent.is_empty() {
            return Vec::new();
        }
        let rows = parent[0].len();
        if rows == 0 {
            return Vec::new();
        }
        let mut subtree_cols: Vec<usize> = Vec::new();
        let mut visited = vec![false; parent.len()];
        fn dfs(c: usize, parent: &IHSSParent, subtree: &mut Vec<usize>, visited: &mut [bool]) {
            if visited[c] {
                return;
            }
            visited[c] = true;
            subtree.push(c);
            for i in 0..parent.len() {
                if parent[i][0] == c as i32 {
                    dfs(i, parent, subtree, visited);
                }
            }
        }
        dfs(root, parent, &mut subtree_cols, &mut visited);
        subtree_cols.sort_unstable();

        let col_map: std::collections::HashMap<usize, usize> = subtree_cols
            .iter()
            .enumerate()
            .map(|(idx, &col)| (col, idx))
            .collect();

        let mut sub_parent: IHSSParent = subtree_cols
            .iter()
            .map(|&col| {
                let mut new_col = Vec::with_capacity(rows);
                for r in 0..rows {
                    let p = parent[col][r];
                    if p == -1 {
                        new_col.push(-1);
                    } else if let Some(&mapped) = col_map.get(&(p as usize)) {
                        new_col.push(mapped as i32);
                    } else {
                        new_col.push(-1);
                    }
                }
                new_col
            })
            .collect();

        if !sub_parent.is_empty() {
            sub_parent[0][0] = -1;
        }
        sub_parent
    }

    fn compute_level(parent: &IHSSParent, c: usize) -> i32 {
        if c >= parent.len() || parent[0].len() < 2 {
            return 0;
        }
        let mut level = 0;
        let mut cur = c;
        loop {
            let p = parent[cur][1];
            if p == -1 {
                break;
            }
            cur = p as usize;
            level += 1;
        }
        level
    }

    fn format_m(k: i32) -> String {
        if k == 1 {
            "M".to_string()
        } else {
            format!("M_{{{}}}", k)
        }
    }

    /// Merge like terms, preserving order. Ordinal addition is not
    /// commutative, so only *consecutive* identical summands are combined and
    /// the original left-to-right order is kept (e.g. A+B+A → A+B+A, never
    /// 2A+B).
    fn merge_parts(parts: &[String]) -> Vec<String> {
        if parts.is_empty() {
            return Vec::new();
        }
        let mut result: Vec<String> = Vec::new();
        let mut i = 0;
        while i < parts.len() {
            let mut j = i;
            while j < parts.len() && parts[j] == parts[i] {
                j += 1;
            }
            let count = j - i;
            if count == 1 {
                result.push(parts[i].clone());
            } else if parts[i] == "1" {
                result.push(count.to_string());
            } else {
                result.push(format!("{} \\times {}", parts[i], count));
            }
            i = j;
        }
        result
    }

    fn join_with_merge(parts: &[String]) -> String {
        if parts.is_empty() {
            return "0".to_string();
        }
        Self::merge_parts(parts).join(" + ")
    }

    fn render_single_tree(parent: &IHSSParent, root: usize) -> Result<String, String> {
        if parent.is_empty() {
            return Err("empty matrix".to_string());
        }
        if root >= parent.len() {
            return Err("root index out of range".to_string());
        }
        let rows = parent[0].len();
        if rows < 2 {
            return Err("matrix needs at least 2 rows".to_string());
        }

        let k = Self::compute_level(parent, root);
        let children = Self::get_children(parent, root);

        if children.is_empty() {
            return if k == 0 { Ok("1".to_string()) } else { Ok(Self::format_m(k)) };
        }

        let mut s: Vec<usize> = Vec::new();
        let mut t: Vec<usize> = Vec::new();
        for &child in &children {
            let level = Self::compute_level(parent, child);
            if level == k + 1 {
                s.push(child);
            } else if level <= k {
                t.push(child);
            } else {
                s.push(child);
            }
        }

        let s_strings: Vec<String> = s.iter().map(|&c| Self::render_single_tree(parent, c)).collect::<Result<_, _>>()?;
        let t_strings: Vec<String> = t.iter().map(|&c| Self::render_single_tree(parent, c)).collect::<Result<_, _>>()?;

        let s_sum = Self::join_with_merge(&s_strings);
        let t_sum = Self::join_with_merge(&t_strings);

        let mk1 = Self::format_m(k + 1);

        if t.is_empty() {
            return Ok(format!("\\psi_{{{}}}(", mk1) + &if s_sum == "0" { "0".to_string() } else { s_sum.clone() } + ")");
        }
        if s.is_empty() {
            let r = format!("\\psi_{{{}}}({})", mk1, mk1);
            return Ok(format!("\\psi_{{{}}}({})", r, t_sum));
        }
        let mut all_s = s_strings;
        all_s.push(mk1.clone());
        let s_sum_with_m = Self::join_with_merge(&all_s);
        let r = format!("\\psi_{{{}}}({})", mk1, s_sum_with_m);
        Ok(format!("\\psi_{{{}}}({})", r, t_sum))
    }

    // ---------- standardness (chkStd, IHSS expansion) ----------

    /// Whether the value matrix is IHSS-standard, mirroring BMS chkStd but
    /// expanding with the IHSS fundamental-sequence algorithm. When `triangular`
    /// is true the identity is the triangular sequence `(0)(1)(2,1)(3,2,1)...`;
    /// otherwise it is the normal identity `(0)(1,1,1)(2,2,2)...`.
    pub fn is_standard_matrix(value: &[Vec<i32>], triangular: bool) -> bool {
        let nc = value.len();
        if nc == 0 {
            return true;
        }
        let nr = value.iter().map(|c| c.len()).max().unwrap_or(0);
        let mut s: Vec<Vec<i32>> = Vec::with_capacity(nc);
        for i in 0..nc {
            let mut col = value[i].clone();
            while col.len() < nr {
                col.push(0);
            }
            s.push(col);
        }

        // Highest non-zero row.
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

        // Find the first column p that differs from the identity.
        let mut p: Option<usize> = None;
        'outer: for i in 0..nc {
            for j in 0..=row {
                let id_val = if triangular {
                    if j <= i { (i - j) as i32 } else { 0 }
                } else {
                    i as i32
                };
                if s[i][j] > id_val {
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

        // SA = the identity prefix up to p, as an IHSS parent matrix.
        let mut sa_value: Vec<Vec<i32>> = Vec::with_capacity(p + 1);
        for i in 0..=p {
            let mut col = vec![0i32; row + 1];
            for j in 0..=row {
                if triangular {
                    col[j] = if j <= i { (i - j) as i32 } else { 0 };
                } else {
                    col[j] = i as i32;
                }
            }
            sa_value.push(col);
        }
        let mut sa = Self::from_value(&sa_value);
        let mut pp = p;

        loop {
            let (expanded, _) = sa.expand(1, 2, "HMS");
            let ev = expanded.to_value();

            let mut newp: Option<usize> = None;
            for i in pp..nc {
                let ev_col = ev.get(i).cloned().unwrap_or_default();
                let mut larger = false;
                let mut smaller = false;
                for j in 0..=row {
                    let a = ev_col.get(j).copied().unwrap_or(0);
                    if s[i][j] > a {
                        larger = true;
                    }
                    if s[i][j] < a {
                        smaller = true;
                    }
                }
                if larger && !smaller {
                    return false;
                }
                if larger || smaller {
                    newp = Some(i);
                    break;
                }
            }

            match newp {
                None => return true,
                Some(n) => {
                    // Trim the expanded matrix to the new divergence prefix and continue.
                    sa = Self::from_value(&ev[..=n]);
                    pp = n;
                }
            }
        }
    }

    /// IHSS-standard against the normal identity `(0)(1,1,1)(2,2,2)...`.
    pub fn is_standard(value: &[Vec<i32>]) -> bool {
        Self::is_standard_matrix(value, false)
    }

    /// IHSS-standard against the triangular identity `(0)(1)(2,1)(3,2,1)...`.
    pub fn is_standard_triangular(value: &[Vec<i32>]) -> bool {
        Self::is_standard_matrix(value, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn val(s: &str) -> Vec<Vec<i32>> {
        let mut out = Vec::new();
        for tok in s.split(')').filter(|t| t.contains('(')) {
            let inner = tok.trim_start_matches('(');
            let col: Vec<i32> = inner
                .split(',')
                .map(|x| x.trim().parse().unwrap_or(0))
                .collect();
            out.push(col);
        }
        out
    }

    #[test]
    fn normal_identity_prefix_is_standard() {
        assert!(IHSS::is_standard(&val("(0)(1)(2)")));
        assert!(IHSS::is_standard(&val("(0)(1,1,1)(2,2,2)")));
        // (0)(1,1)(2,2,2) (=0 11 222) is NOT standard.
        assert!(!IHSS::is_standard(&val("(0)(1,1)(2,2,2)")));
        // (0,0,0)(1,1,1)(2,1,1) is standard (normal).
        assert!(IHSS::is_standard(&val("(0,0,0)(1,1,1)(2,1,1)")));
    }

    #[test]
    fn triangular_identity_prefix_is_standard() {
        assert!(IHSS::is_standard_triangular(&val("(0)(1)(2,1)(3,2,1)")));
        // (0)(1)(2,1) (=0 1 21) is standard but triangular, not normal.
        assert!(IHSS::is_standard_triangular(&val("(0)(1)(2,1)")));
        assert!(!IHSS::is_standard(&val("(0)(1)(2,1)")));
    }

    #[test]
    fn known_standard_matrices() {
        // (0,0)(1,1)(2,1) ↔ ψ_M(ψ_{ψ_{M_2}(M_2)}(M)) — normal standard.
        assert!(IHSS::is_standard(&val("(0,0)(1,1)(2,1)")));
        // (0,0)(1,0)(2,1) ↔ ψ_{ψ_M(M)}(ψ_M(M)) — triangular standard.
        assert!(IHSS::is_standard_triangular(&val("(0,0)(1,0)(2,1)")));
        // (0)(1)(0) ↔ ψ_{ψ_M(M)}(1) + 1 — standard both.
        assert!(IHSS::is_standard(&val("(0)(1)(0)")));
        assert!(IHSS::is_standard_triangular(&val("(0)(1)(0)")));
    }

    #[test]
    fn standardness_on_raw_parse() {
        // Standardness must reflect the RAW input, not the from_value-reconstructed
        // matrix (which normalizes). (0)(1,1)(2,2,2) is genuinely non-standard.
        let raw = IHSS::parse("(0)(1,1)(2,2,2)").unwrap();
        assert_eq!(raw, vec![vec![0,0,0], vec![1,1,0], vec![2,2,2]]);
        assert!(!IHSS::is_standard_matrix(&raw, false));
        assert!(!IHSS::is_standard_matrix(&raw, true));
    }
}
