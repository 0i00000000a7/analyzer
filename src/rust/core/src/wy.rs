#![allow(unused_assignments, unused_mut)]
//! ω-Y / n-Y mountain building and expansion.

use crate::Mountain;

// ════════════════════════════════════════════════════════════════
// ω-base digit representation: ord(n) = [0,...,0,1] (ω^n)
// Little-endian: index 0 = ω^0, index 1 = ω^1, ...
// ════════════════════════════════════════════════════════════════

pub type Ord = Vec<i32>;

pub fn normalize(mut a: Ord) -> Ord {
    while a.len() > 1 && *a.last().unwrap() == 0 {
        a.pop();
    }
    a
}

fn ord_plus(a: &Ord, b: &Ord) -> Ord {
    let len = a.len().max(b.len());
    let mut res = vec![0i32; len];
    let b_len = b.len() as i32;
    for i in (0..len as i32).rev() {
        if i >= b_len - 1 && (i as usize) < a.len() {
            res[i as usize] += a[i as usize];
        }
        if i <= b_len - 1 {
            res[i as usize] += b[i as usize];
        }
    }
    res
}

fn ord_minus(a: &Ord, b: &Ord) -> Ord {
    if a.len() < b.len() {
        return vec![-1];
    }
    let mut borrow = true;
    let mut res = a.clone();
    for i in (0..a.len() as i32).rev() {
        let bi = if (i as usize) < b.len() { b[i as usize] } else { 0 };
        if borrow {
            if a[i as usize] > bi {
                borrow = false;
                res[i as usize] = a[i as usize] - bi;
            } else {
                res.pop();
            }
        } else {
            res[i as usize] = a[i as usize];
        }
    }
    res
}

fn ord(n: usize) -> Ord {
    let mut res = vec![0i32; n + 1];
    res[n] = 1;
    res
}

fn ord_cmp(a: &Ord, b: &Ord) -> i32 {
    if a.len() > b.len() {
        return 1;
    }
    for i in (0..b.len() as i32).rev() {
        let va = if (i as usize) < a.len() { a[i as usize] } else { 0 };
        let vb = b[i as usize];
        if vb > va {
            return -1;
        }
        if vb < va {
            return 1;
        }
    }
    0
}

fn ord_min(a: &Ord, b: &Ord) -> Ord {
    if ord_cmp(a, b) >= 0 {
        b.clone()
    } else {
        a.clone()
    }
}

#[allow(dead_code)]
fn ord_length(a: &Ord) -> usize {
    for (i, &v) in a.iter().enumerate() {
        if v != 0 {
            return i + 1;
        }
    }
    a.len()
}

// ════════════════════════════════════════════════════════════════
// Mountain graph node (arena with indices)
// ════════════════════════════════════════════════════════════════

pub struct Node {
    pub value: i32,
    pub x: usize, // column index
    pub y: Ord,   // row label
    pub up: Option<usize>,
    pub down: Option<usize>,
    pub left: Option<usize>,
    pub right: Vec<usize>,
    pub is_magma: bool,
}

pub struct Graph {
    pub nodes: Vec<Node>,
}

impl Graph {
    pub fn new() -> Self {
        Graph { nodes: Vec::new() }
    }

    fn new_node(&mut self, value: i32, x: usize, y: Ord) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(Node {
            value,
            x,
            y,
            up: None,
            down: None,
            left: None,
            right: Vec::new(),
            is_magma: false,
        });
        idx
    }

    fn connect_h(&mut self, n1: usize, n2: usize) {
        self.nodes[n1].right.push(n2);
        self.nodes[n2].left = Some(n1);
    }

    fn connect_v(&mut self, n1: usize, n2: usize) {
        if let Some(up) = self.nodes[n2].up {
            self.nodes[up].down = Some(n1);
        }
        self.nodes[n1].down = Some(n2);
        self.nodes[n2].up = Some(n1);
    }

    fn magma_op(&mut self, node: usize) {
        let rights = self.nodes[node].right.clone();
        for nd in rights {
            let target = self.nodes[nd].down;
            if let Some(t) = target {
                let same_y = ord_cmp(&self.nodes[t].y, &self.nodes[node].y) == 0;
                if same_y {
                    self.nodes[t].is_magma = true;
                    self.magma_op(t);
                }
            }
        }
    }

    fn find_node(&self, mut x: usize, y: &Ord, eq: bool) -> usize {
        loop {
            match self.nodes[x].up {
                Some(up) => {
                    let c = ord_cmp(&self.nodes[up].y, y) + if eq { 0 } else { 1 };
                    if c <= 0 {
                        x = up;
                        continue;
                    }
                    break;
                }
                None => break,
            }
        }
        x
    }

    fn generate_mountain(&mut self, seq: &[i32]) -> Vec<usize> {
        let len = seq.len();
        let mut mt = vec![0usize; len];
        for i in 0..len {
            let nd = self.new_node(seq[i], i, vec![0]);
            let base = self.new_node(-1, i, vec![-1]);
            mt[i] = nd;
            self.connect_v(nd, base);
            if i > 0 {
                let prev = mt[i - 1];
                let prev_down = self.nodes[prev].down.unwrap();
                self.connect_h(prev_down, nd);
            }
        }
        mt
    }

    fn copy_mountain(&mut self, seq: &[usize]) -> Vec<usize> {
        let len = seq.len();
        let mut mt = vec![0usize; len];
        for i in 0..len {
            let nd = self.new_node(self.nodes[seq[i]].value, i, vec![0]);
            let base = self.new_node(-1, i, vec![-1]);
            mt[i] = nd;
            self.connect_v(nd, base);
            if i > 0 {
                // Find the correct parent (matching reference copyMountain)
                let mut parent = self.nodes[seq[i]].left;
                loop {
                    let pi = match parent {
                        Some(p) => p,
                        None => break,
                    };
                    if self.nodes[pi].x == 0 {
                        break;
                    }
                    let px = self.nodes[pi].x;
                    let min_y = ord_min(&self.nodes[seq[i]].y, &self.nodes[seq[px]].y);
                    parent = Some(self.find_node(pi, &min_y, true));
                    let pi2 = parent.unwrap();
                    if ord_cmp(&self.nodes[pi2].y, &self.nodes[seq[px]].y) == 0 {
                        break;
                    }
                    parent = self.nodes[pi2].left;
                }
                let px = self.nodes[parent.unwrap()].x;
                let mt_px_down = self.nodes[mt[px]].down.unwrap();
                self.connect_h(mt_px_down, nd);
            }
        }
        mt
    }

    /// Draw the deep mountain structure.
    /// n = -1: ω-Y (full depth), n = 0: 0-Y, n = 1: 1-Y, etc.
    fn draw_mountain(&mut self, seq: &[usize], n: i32, consistent: bool) -> Vec<usize> {
        let mut mt = self.copy_mountain(seq);
        let len = seq.len();
        for i in 0..len {
            let mut nd1 = mt[i];
            loop {
                let nd1_left = self.nodes[nd1].left;
                if nd1_left.is_none() {
                    break;
                }
                let mut flag = false;
                let mut p: Option<usize> = Some(nd1);
                loop {
                    let pi = match p {
                        Some(p) => p,
                        None => break,
                    };
                    if self.nodes[pi].value < self.nodes[nd1].value {
                        break;
                    }
                    p = self.nodes[pi].left;
                    loop {
                        let pi2 = match p {
                            Some(p) => p,
                            None => break,
                        };
                        let up = match self.nodes[pi2].up {
                            Some(u) => u,
                            None => break,
                        };
                        if ord_cmp(&self.nodes[up].y, &self.nodes[nd1].y) <= 0 {
                            p = Some(up);
                        } else {
                            break;
                        }
                    }
                }
                if p.is_none() {
                    if consistent {
                        flag = true;
                        p = nd1_left;
                    } else {
                        break;
                    }
                }
                let pu = p.unwrap();
                let diff = ord_minus(&self.nodes[nd1].y, &self.nodes[pu].y);
                let mut dy = diff.len() as i32;
                if dy >= 1 {
                    flag = true;
                }
                if n >= 0 {
                    dy = dy.min(n);
                    if n != 0 && dy >= n {
                        break;
                    }
                }
                let newy = ord_plus(&self.nodes[nd1].y, &ord(dy as usize));
                if consistent && flag {
                    let p2 = self.find_node(nd1_left.unwrap(), &newy, false);
                    let new_node = self.new_node(self.nodes[nd1].value, i, newy);
                    self.connect_h(p2, new_node);
                    self.connect_v(new_node, nd1);
                    break;
                }
                let new_value = self.nodes[nd1].value - self.nodes[pu].value;
                let new_node = self.new_node(new_value, i, newy);
                self.connect_h(pu, new_node);
                self.connect_v(new_node, nd1);
                nd1 = new_node;
            }
        }
        mt
    }

    fn expand_wy_mountain(
        &mut self,
        seq: &mut Vec<usize>,
        fs: i32,
        n: i32,
        consistent: bool,
        depth: i32,
    ) -> Vec<usize> {
        if depth > 100 {
            return seq.clone();
        }
        let mt1 = self.draw_mountain(seq, n, consistent);

        if self.nodes[*seq.last().unwrap()].value <= 1 || fs <= 0 {
            seq.pop();
            return self.draw_mountain(seq, n, false);
        }

        self.nodes[*seq.last().unwrap()].value -= 1;
        let mut mt2 = self.draw_mountain(seq, n, consistent);

        let len = seq.len();
        let mut idx = vec![0usize; len];
        let mut nd: usize = 0;
        for i in 0..len {
            let mut top = mt1[i];
            while let Some(u) = self.nodes[top].up {
                top = u;
            }
            idx[i] = top;
            nd = top;
        }

        let mut iterate = false;
        let mut diagonal: Vec<usize> = Vec::new();
        let mut diagonal2: Vec<usize> = Vec::new();
        let mut top1: usize = 0;
        let mut root: usize = 0;

        if n > 0 {
            diagonal = self.copy_mountain(&idx);
            if self.nodes[*idx.last().unwrap()].value > 1 && !diagonal.is_empty() {
                iterate = true;
                diagonal2 = self.expand_wy_mountain(&mut diagonal, fs, n, consistent, depth + 1);
            }
        }

        if iterate {
            let bl0 = ((diagonal2.len() as f64 - len as f64 + 1.0) / fs as f64).round() as i32;
            if bl0 > 0 && len as i32 - 1 - bl0 >= 0 {
                root = idx[(len as i32 - 1 - bl0) as usize];
                top1 = self.new_node(1, len - 1, ord(n as usize));
            } else {
                iterate = false;
            }
        }

        if !iterate {
            let mut xd = mt2[self.nodes[*idx.last().unwrap()].x];
            while let Some(u) = self.nodes[xd].up {
                xd = u;
            }
            *idx.last_mut().unwrap() = xd;
            root = self.nodes[nd].left.unwrap();
            top1 = nd;
        }

        let bl = len as i32 - 1 - self.nodes[root].x as i32;

        // Collect reference chain rc
        let mut rc: Vec<usize> = Vec::new();
        {
            let root_col = self.nodes[root].x;
            let mut node = self.nodes[mt2[root_col]].down;
            while let Some(ni) = node {
                if ord_cmp(&self.nodes[root].y, &self.nodes[ni].y) < 0 {
                    break;
                }
                self.magma_op(ni);
                rc.push(ni);
                match self.nodes[ni].up {
                    Some(u) => node = Some(u),
                    None => break,
                }
            }
        }
        rc.push(top1);

        for i in 0..fs {
            let dis = (i + 1) * bl;

            // Build ref array by walking up from the last column's bottom
            let mut refs: Vec<Option<usize>> = vec![None; rc.len() - 1];
            {
                let mut node = self.nodes[*mt2.last().unwrap()].down;
                let mut ir = 1usize;
                let mut yr = self.nodes[rc[ir]].y.clone();
                while let Some(ni) = node {
                    let cond = match self.nodes[ni].up {
                        Some(u) => ord_cmp(&self.nodes[u].y, &yr) >= 0,
                        None => true,
                    };
                    if cond {
                        refs[ir - 1] = Some(ni);
                        ir += 1;
                        if ir >= rc.len() {
                            break;
                        }
                        yr = self.nodes[rc[ir]].y.clone();
                    }
                    node = self.nodes[ni].up;
                }
            }

            // Create tops/roots and new bottom columns
            let mut tops = vec![0usize; bl as usize];
            let mut roots = vec![0usize; bl as usize];
            for j in 0..bl as usize {
                tops[j] = self.new_node(-1, mt2.len(), vec![-1]);
                roots[j] = self.new_node(-1, mt2.len(), vec![-1]);
                mt2.push(self.new_node(-2, mt2.len(), vec![0]));
            }

            for j in 0..bl as usize {
                if i == fs - 1 && j == bl as usize - 1 {
                    break;
                }
                let col = self.nodes[root].x + j + 1;
                let mut nd3 = mt2[col];
                let mut ir2 = 0usize;
                let mut this_ref = refs[ir2];
                let mut new_left: usize = 0;

                loop {
                    if self.nodes[nd3].is_magma {
                        ir2 += 1;
                        if ir2 < refs.len() {
                            this_ref = refs[ir2];
                        }

                        // wildfire edge
                        let this_node = self.nodes[nd3].left.unwrap();
                        let tn_x = self.nodes[this_node].x;
                        let y3 = self.nodes[nd3].y.clone();
                        new_left = self.find_node(mt2[tn_x + dis as usize], &y3, false);
                        let new_right = self.new_node(-1, self.nodes[nd3].x + dis as usize, self.nodes[nd3].y.clone());
                        self.connect_h(new_left, new_right);
                        self.connect_v(new_right, tops[j]);
                        tops[j] = new_right;
                        if ord_cmp(&self.nodes[new_right].y, &vec![0]) == 0 {
                            mt2[self.nodes[nd3].x + dis as usize] = new_right;
                        }

                        // magma edge
                        let magma_node = self.nodes[self.nodes[nd3].up.unwrap()].left.unwrap();
                        let mn_x = self.nodes[magma_node].x;
                        let mut new_left2 = self.find_node(mt2[mn_x + dis as usize], &self.nodes[nd3].y, true);
                        loop {
                            let up2 = match self.nodes[new_left2].up {
                                Some(u) => u,
                                None => break,
                            };
                            if ord_cmp(&self.nodes[new_left2].y, &self.nodes[this_ref.unwrap()].y) >= 0 {
                                break;
                            }
                            let dy_len = ord_minus(&self.nodes[up2].y, &self.nodes[new_left2].y).len();
                            for k in 0..dy_len {
                                let new_y = ord_plus(&self.nodes[new_left2].y, &ord(k));
                                let new_right2 = self.new_node(-1, self.nodes[nd3].x + dis as usize, new_y);
                                self.connect_h(new_left2, new_right2);
                                self.connect_v(new_right2, tops[j]);
                                tops[j] = new_right2;
                                if ord_cmp(&self.nodes[new_right2].y, &vec![0]) == 0 {
                                    mt2[self.nodes[nd3].x + dis as usize] = new_right2;
                                }
                            }
                            new_left2 = up2;
                        }
                    } else {
                        // eruption edge
                        let this_node = self.nodes[nd3].left.unwrap();
                        let tn_x = self.nodes[this_node].x;
                        let dy = ord_minus(&self.nodes[nd3].y, &self.nodes[rc[ir2]].y);
                        let new_y = ord_plus(&self.nodes[this_ref.unwrap()].y, &dy);
                        let new_right = self.new_node(-1, self.nodes[nd3].x + dis as usize, new_y);

                        if tn_x < self.nodes[root].x {
                            new_left = this_node;
                        } else {
                            new_left = self.find_node(mt2[tn_x + dis as usize], &self.nodes[new_right].y, false);
                        }
                        self.connect_h(new_left, new_right);
                        self.connect_v(new_right, tops[j]);
                        tops[j] = new_right;
                        if ord_cmp(&self.nodes[new_right].y, &vec![0]) == 0 {
                            mt2[self.nodes[nd3].x + dis as usize] = new_right;
                        }
                    }
                    match self.nodes[nd3].up {
                        Some(u) => nd3 = u,
                        None => break,
                    }
                }

                let mut xd = tops[j];
                if iterate {
                    self.nodes[xd].value = self.nodes[diagonal2[self.nodes[xd].x]].value;
                } else {
                    let v = self.nodes[idx[self.nodes[root].x + j + 1]].value;
                    self.nodes[xd].value = v;
                }
                while ord_cmp(&self.nodes[xd].y, &vec![0]) > 0 {
                    let down = self.nodes[xd].down.unwrap();
                    let left = self.nodes[xd].left.unwrap();
                    let val = if consistent && self.nodes[xd].y[0] == 0 {
                        self.nodes[xd].value
                    } else {
                        self.nodes[xd].value + self.nodes[left].value
                    };
                    self.nodes[down].value = val;
                    xd = down;
                }
            }
        }

        mt2.pop();
        mt2
    }
}

pub fn expand_wy(seq: &[i32], fs: i32, n: i32, consistent: bool) -> Vec<i32> {
    let mut graph = Graph::new();
    let mut mt = graph.generate_mountain(seq);
    let result = graph.expand_wy_mountain(&mut mt, fs, n, consistent, 0);
    result.iter().map(|&i| graph.nodes[i].value).collect()
}

pub fn expand_1y(seq: &[i32], fs: i32) -> Vec<i32> {
    expand_wy(seq, fs, 1, false)
}

pub fn expand_wy_seq(seq: &[i32], fs: i32) -> Vec<i32> {
    expand_wy(seq, fs, -1, false)
}

pub fn expand_ny(seq: &[i32], fs: i32, n: i32) -> Vec<i32> {
    if n >= 0 {
        expand_wy(seq, fs, n, false)
    } else {
        expand_wy(seq, fs, -1, false)
    }
}

/// Build a layered ω-Y mountain for display, with row labels.
pub fn build_wy_mountain_with_rows(seq: &[i32], n: i32, consistent: bool) -> (Mountain, Vec<Ord>) {
    let mut graph = Graph::new();
    let mt = graph.generate_mountain(seq);
    let drawn = graph.draw_mountain(&mt, n, consistent);

    let len = drawn.len();
    let mut idx: Vec<Option<usize>> = drawn.iter().map(|&i| Some(i)).collect();

    let mut result: Mountain = Vec::new();
    let mut row_labels: Vec<Ord> = Vec::new();
    let mut row: Ord = vec![0];
    let mut running = true;

    while running {
        running = false;
        let mut row0: Ord = Vec::new();
        let mut has_row0 = false;
        let mut layer = vec![(0i32, 0i32); len];

        for i in 0..len {
            match idx[i] {
                None => {
                    layer[i] = (-1, -1);
                }
                Some(ni) => {
                    if ord_cmp(&row, &graph.nodes[ni].y) < 0 {
                        layer[i] = (-1, -1);
                    } else {
                        running = true;
                        let parent_col = graph.nodes[ni].left.map(|l| graph.nodes[l].x).unwrap_or(usize::MAX);
                        let parent_dist = if parent_col != usize::MAX {
                            i as i32 - parent_col as i32
                        } else {
                            0
                        };
                        layer[i] = (graph.nodes[ni].value, parent_dist);
                        idx[i] = graph.nodes[ni].up;
                    }
                }
            }
            if let Some(ni) = idx[i] {
                if !has_row0 || ord_cmp(&graph.nodes[ni].y, &row0) < 0 {
                    row0 = graph.nodes[ni].y.clone();
                    has_row0 = true;
                }
            }
        }

        if running {
            result.push(layer);
            row_labels.push(row.clone());
            if has_row0 {
                row = row0;
            } else {
                row = ord_plus(&row, &vec![1]);
            }
        }
    }

    (result, row_labels)
}

pub fn build_wy_mountain(seq: &[i32], n: i32, consistent: bool) -> Mountain {
    build_wy_mountain_with_rows(seq, n, consistent).0
}
