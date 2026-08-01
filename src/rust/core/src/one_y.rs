//! 1-Y mountain building.

use crate::wy::Ord;
use crate::Mountain;

fn normalize(mut a: Ord) -> Ord {
    while a.len() > 1 && *a.last().unwrap() == 0 {
        a.pop();
    }
    a
}

/// Row ordinal: base at ext k = ω·k, diff depth d = ω·k + d
fn row_ordinal(ext: i32, diff_depth: i32) -> Ord {
    if ext == 0 && diff_depth == 0 {
        return vec![0];
    }
    if ext == 0 {
        return vec![diff_depth];
    }
    let mut ord = vec![0i32, 0];
    ord[1] = ext;
    ord[0] = diff_depth;
    normalize(ord)
}

/// Definition (1): first smaller to left
fn find_parent(seq: &[i32], i: usize) -> i32 {
    for j in (0..i).rev() {
        if seq[j] < seq[i] {
            return j as i32;
        }
    }
    -1
}

/// Check ancestor relationship in a parent array
fn is_ancestor_of(parents: &[i32], mut col: i32, anc: i32) -> bool {
    while col >= 0 {
        if col == anc {
            return true;
        }
        col = parents[col as usize];
    }
    false
}

/// Build the 1-Y mountain layers with row labels.
pub fn build_1y_mountain_with_rows(seq: &[i32]) -> (Mountain, Vec<Ord>) {
    let mut result: Mountain = Vec::new();
    let mut row_labels: Vec<Ord> = Vec::new();

    let mut cur = seq.to_vec();
    let mut extracted_parents: Vec<i32> = Vec::new(); // def(6) parents from previous round

    for ext in 0.. {
        let n = cur.len();

        // ── Base layer ──
        let mut base_parent = vec![-1i32; n];
        if ext == 0 || extracted_parents.is_empty() {
            // Original seq: definition (1)
            for i in 0..n {
                if cur[i] > 1 {
                    base_parent[i] = find_parent(&cur, i);
                }
            }
        } else {
            // Extracted seq: def(6) parents, items = 1 have no parent
            for i in 0..n {
                if cur[i] > 1 {
                    base_parent[i] = extracted_parents[i];
                }
            }
        }

        let mut base = vec![(0i32, 0i32); n];
        for i in 0..n {
            base[i] = (cur[i], if base_parent[i] >= 0 { i as i32 - base_parent[i] } else { 0 });
        }
        result.push(base);
        row_labels.push(row_ordinal(ext, 0));
        let first_layer_idx = result.len() - 1;

        let mut parent_in_layers: Vec<Vec<i32>> = Vec::new();
        parent_in_layers.push(base_parent.clone());

        let mut prev_vals = cur.clone();

        // ── Inner loop: diff layers ──
        for diff_depth in 1.. {
            let mut cur_diffs = vec![-1i32; n];
            let mut layer_parents = vec![-1i32; n];

            for i in 0..n {
                if prev_vals[i] <= 0 {
                    continue; // sentinel
                }
                if prev_vals[i] == 1 {
                    continue; // =1 → no parent, no diff
                }

                let mut p = -1;
                if diff_depth == 1 {
                    // First diff: diff value from def(1) parent, ancestry def(3)
                    let base_p = base_parent[i];
                    if base_p >= 0 {
                        let diff_val = prev_vals[i] - prev_vals[base_p as usize];
                        for j in (0..i).rev() {
                            if prev_vals[j] <= 0 || base_parent[j] < 0 {
                                continue;
                            }
                            let j_diff_val = prev_vals[j] - prev_vals[base_parent[j] as usize];
                            if j_diff_val >= diff_val {
                                continue;
                            }
                            if is_ancestor_of(&base_parent, i as i32, j as i32) {
                                p = j as i32;
                                break;
                            }
                        }
                        if p < 0 {
                            p = base_p;
                        }
                        cur_diffs[i] = prev_vals[i] - prev_vals[base_p as usize];
                        layer_parents[i] = p;
                    }
                } else {
                    // L2+: def(3) for parent and diff value
                    let below_parents = parent_in_layers.last().unwrap().clone();
                    for j in (0..i).rev() {
                        if prev_vals[j] <= 0 {
                            continue;
                        }
                        if prev_vals[j] >= prev_vals[i] {
                            continue;
                        }
                        if is_ancestor_of(&below_parents, i as i32, j as i32) {
                            p = j as i32;
                            break;
                        }
                    }
                    if p >= 0 {
                        cur_diffs[i] = prev_vals[i] - prev_vals[p as usize];
                        layer_parents[i] = p;
                    }
                }
            }

            // Check for non-sentinel values
            let mut has_value = false;
            for i in 0..n {
                if cur_diffs[i] >= 0 {
                    has_value = true;
                    break;
                }
            }
            if !has_value {
                break;
            }

            // Convergence: stop when ALL non-sentinel diffs = 1.
            let mut all_converged = true;
            for i in 0..n {
                if cur_diffs[i] >= 0 && cur_diffs[i] != 1 {
                    all_converged = false;
                    break;
                }
            }

            // Push diff layer with left-leg parent distances for display
            let mut diff_layer = vec![(-1i32, -1i32); n];
            for i in 0..n {
                if cur_diffs[i] < 0 {
                    continue;
                }
                let leg_p = if diff_depth == 1 {
                    base_parent[i]
                } else {
                    parent_in_layers.last().unwrap()[i]
                };
                diff_layer[i] = (cur_diffs[i], if leg_p >= 0 { i as i32 - leg_p } else { 0 });
            }

            result.push(diff_layer);
            row_labels.push(row_ordinal(ext, diff_depth as i32));
            parent_in_layers.push(layer_parents);

            if all_converged {
                break;
            }

            prev_vals = cur_diffs;
        }

        // ── Check topmost values ──
        let mut topmost = vec![-1i32; n];
        for col in 0..n {
            for l in (first_layer_idx..result.len()).rev() {
                if result[l][col].0 >= 0 {
                    topmost[col] = result[l][col].0;
                    break;
                }
            }
        }

        let mut all_one = true;
        for i in 0..n {
            if topmost[i] >= 0 && topmost[i] != 1 {
                all_one = false;
                break;
            }
        }
        if all_one {
            break;
        }

        // ── Extract: form the new base sequence ──
        let mut next = vec![0i32; n];
        for col in 0..n {
            for l in (first_layer_idx..result.len()).rev() {
                if result[l][col].0 >= 0 {
                    next[col] = result[l][col].0;
                    break;
                }
            }
        }

        if next == cur {
            break;
        }

        // Relative layer of each column's topmost (for def(6) traversal)
        let mut top_layer_rel = vec![0usize; n];
        for col in 0..n {
            for l in (first_layer_idx..result.len()).rev() {
                if result[l][col].0 >= 0 {
                    top_layer_rel[col] = l - first_layer_idx;
                    break;
                }
            }
        }

        // ── Definition (6): extraction parents (quasi-parents) ──
        let mut quasi_parent = vec![-1i32; n];
        for col in 1..n {
            if next[col] <= 1 {
                continue;
            }
            let rel_layer = top_layer_rel[col];
            if rel_layer == 0 {
                continue;
            }
            let mut cur_col = col;
            while rel_layer > 0 {
                let below_parents = parent_in_layers[rel_layer - 1].clone();
                let p_col = below_parents[cur_col];
                if p_col < 0 {
                    break;
                }
                if top_layer_rel[p_col as usize] == rel_layer - 1 {
                    quasi_parent[col] = p_col;
                    break;
                }
                cur_col = p_col as usize;
                if top_layer_rel[cur_col] == rel_layer {
                    quasi_parent[col] = cur_col as i32;
                    break;
                }
            }
        }

        // Find the actual extraction parent: scan left from j-1 to 0
        // for the rightmost k where next[k] < next[j] AND k is a
        // quasi-ancestor of j.
        extracted_parents = vec![-1i32; n];
        for col in 1..n {
            if next[col] <= 1 {
                continue;
            }
            // Build quasi-ancestor chain for this column
            let mut in_chain = vec![false; n];
            let mut c = col as i32;
            while c >= 0 {
                in_chain[c as usize] = true;
                if quasi_parent[c as usize] < 0 {
                    break;
                }
                c = quasi_parent[c as usize];
            }
            // Scan left for rightmost k with smaller value AND in chain
            for k in (0..col).rev() {
                if next[k] >= 0 && next[k] < next[col] && in_chain[k] {
                    extracted_parents[col] = k as i32;
                    break;
                }
            }
        }

        cur = next;
    }

    (result, row_labels)
}

pub fn build_1y_mountain(seq: &[i32]) -> Mountain {
    build_1y_mountain_with_rows(seq).0
}
