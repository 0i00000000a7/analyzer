//! PSS Hydra and HPrSS (Hyper Primitive Sequence System).
//!
//! PSS Hydra expressions: sums of ψ^H_n(A). Zero is the empty sum.
//! The hydra embeds into the BOCF Term as ψ^H_n(A) ↔ ψ_{n-1}(A) (Ω_n = ψ_n(0)),
//! so `standard_form(hydra_to_term(h))` is the BOCF conversion and
//! `term_to_hydra(standard_form(t))` is the reverse.

use crate::term::*;
use crate::Matrix;

// ════════════════════════════════════════════════════════════════
// PSS Hydra
// ════════════════════════════════════════════════════════════════

/// A sum of ψ^H_n(arg) terms; the empty vector is 0.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Hydra(pub Vec<(i64, Hydra)>);

pub fn hydra_zero() -> Hydra {
    Hydra(Vec::new())
}

pub fn is_hydra_zero(h: &Hydra) -> bool {
    h.0.is_empty()
}

/// Level: L(0) = 1, L(ψ^H_n(A)) = n, L(sum) = max of summand levels.
pub fn hydra_level(h: &Hydra) -> i64 {
    let mut max = 1i64;
    for (n, _) in &h.0 {
        if *n > max {
            max = *n;
        }
    }
    max
}

/// ψ^H_n(A) is legal iff level(A) ≤ n+1.
pub fn is_legal_hydra(h: &Hydra) -> bool {
    for (n, arg) in &h.0 {
        if *n < 1 || hydra_level(arg) > *n + 1 || !is_legal_hydra(arg) {
            return false;
        }
    }
    true
}

// ── Parsing / formatting ──

/// Parse "p1(p2(0)+p2)" or "ψ^H_1(ψ^H_2(0)+ψ^H_2)" style input.
/// "pn" is shorthand for "pn(0)".
pub fn parse_hydra(input: &str) -> Result<Hydra, String> {
    let s: Vec<char> = input.trim().chars().collect();
    let mut pos = 0usize;
    let mut sum = Vec::new();
    parse_hydra_expr(&s, &mut pos, &mut sum)?;
    skip_ws(&s, &mut pos);
    if pos != s.len() {
        return Err(format!("Unexpected trailing input at '{}'", s[pos..].iter().collect::<String>()));
    }
    Ok(Hydra(sum))
}

fn skip_ws(s: &[char], pos: &mut usize) {
    while *pos < s.len() && s[*pos].is_whitespace() {
        *pos += 1;
    }
}

/// Parse a sum of ψ terms into `out`.
fn parse_hydra_expr(s: &[char], pos: &mut usize, out: &mut Vec<(i64, Hydra)>) -> Result<(), String> {
    loop {
        skip_ws(s, pos);
        if *pos >= s.len() {
            return Err("Unexpected end of input".to_string());
        }
        let c = s[*pos];
        if c == '+' {
            *pos += 1;
            continue;
        }
        if c == '0' {
            // A bare 0 contributes nothing; skip any digits that follow.
            while *pos < s.len() && s[*pos].is_ascii_digit() {
                *pos += 1;
            }
            skip_ws(s, pos);
            if *pos >= s.len() || s[*pos] != '+' {
                break;
            }
            continue;
        }
        let n = if c == 'p' || c == 'P' {
            *pos += 1;
            parse_num(s, pos)?
        } else if c == 'ψ' {
            *pos += 1;
            if *pos < s.len() && s[*pos] == '^' {
                *pos += 1;
            }
            if *pos < s.len() && (s[*pos] == 'H' || s[*pos] == 'h') {
                *pos += 1;
            }
            if *pos < s.len() && s[*pos] == '_' {
                *pos += 1;
            }
            parse_num(s, pos)?
        } else {
            return Err(format!("Unexpected character '{}'", c));
        };
        if n < 1 {
            return Err("ψ^H subscript must be a positive integer".to_string());
        }
        skip_ws(s, pos);
        let arg = if *pos < s.len() && s[*pos] == '(' {
            *pos += 1;
            let mut inner = Vec::new();
            parse_hydra_expr(s, pos, &mut inner)?;
            skip_ws(s, pos);
            if *pos >= s.len() || s[*pos] != ')' {
                return Err("Missing ')'".to_string());
            }
            *pos += 1;
            Hydra(inner)
        } else {
            hydra_zero()
        };
        out.push((n, arg));
        skip_ws(s, pos);
        if *pos >= s.len() || s[*pos] != '+' {
            break;
        }
        continue;
    }
    Ok(())
}

fn parse_num(s: &[char], pos: &mut usize) -> Result<i64, String> {
    let start = *pos;
    while *pos < s.len() && s[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos == start {
        return Err("Expected digits".to_string());
    }
    let n: i64 = s[start..*pos].iter().collect::<String>().parse().map_err(|_| "Number too large".to_string())?;
    Ok(n)
}

/// Format as "p1(p2(0)+p1(0))"; zero formats as "0".
pub fn format_hydra(h: &Hydra) -> String {
    if h.0.is_empty() {
        return "0".to_string();
    }
    let mut parts = Vec::new();
    for (n, arg) in &h.0 {
        parts.push(format!("p{}({})", n, format_hydra(arg)));
    }
    parts.join("+")
}

/// Format as "ψ^H_1(ψ^H_2(0)+ψ^H_1(0))"; zero formats as "0".
pub fn format_hydra_psi(h: &Hydra) -> String {
    if h.0.is_empty() {
        return "0".to_string();
    }
    let mut parts = Vec::new();
    for (n, arg) in &h.0 {
        parts.push(format!("ψ^H_{}({})", n, format_hydra_psi(arg)));
    }
    parts.join("+")
}

// ── Expansion (rules 1-4) ──

/// The rightmost ψ^H_n(0) node, returned as a path of summand indices
/// (always descending through the last summand). None iff h is zero.
fn rightmost_zero_psi_path(h: &Hydra) -> Option<Vec<usize>> {
    if h.0.is_empty() {
        return None;
    }
    let last = h.0.len() - 1;
    let (_, arg) = &h.0[last];
    if arg.0.is_empty() {
        return Some(vec![last]);
    }
    let mut path = rightmost_zero_psi_path(arg)?;
    path.insert(0, last);
    Some(path)
}

/// Replace the summand at `path` (indices from the root) with `replacement`.
fn replace_at(h: &Hydra, path: &[usize], replacement: &Hydra) -> Hydra {
    if path.is_empty() {
        return replacement.clone();
    }
    let mut terms = h.0.clone();
    let i = path[0];
    if path.len() == 1 {
        terms.splice(i..=i, replacement.0.iter().cloned());
    } else {
        let (n, arg) = &terms[i];
        terms[i] = (n.clone(), replace_at(arg, &path[1..], replacement));
    }
    Hydra(terms)
}

fn summand_at<'a>(h: &'a Hydra, path: &[usize]) -> &'a (i64, Hydra) {
    let mut cur = h;
    for (i, idx) in path.iter().enumerate() {
        if i + 1 == path.len() {
            return &cur.0[*idx];
        }
        cur = &cur.0[*idx].1;
    }
    unreachable!()
}

/// Expand a hydra expression: S[n]. Follows the four rules of the reference.
pub fn expand_hydra(h: &Hydra, n: i32) -> Result<Hydra, String> {
    if h.0.is_empty() {
        return Ok(hydra_zero());
    }
    let copies = n.max(0) as usize;
    let path = rightmost_zero_psi_path(h).unwrap();
    let target_n = summand_at(h, &path).0;

    // Rule 2: rightmost ψ^H_1(0) at top level → predecessor.
    if path.len() == 1 && target_n == 1 {
        let mut terms = h.0.clone();
        terms.pop();
        return Ok(Hydra(terms));
    }

    let parent_path = &path[..path.len() - 1];

    if target_n == 1 {
        // Rule 3: P = #_1(ψ^H_k(#_2 + ψ^H_1(0))) → P[n] = #_1(ψ^H_k(#_2) × n).
        let parent = summand_at(h, parent_path).clone();
        let k = parent.0;
        let mut tail = parent.1 .0.clone();
        tail.pop(); // the target is the last summand of the parent's argument
        let mut copies_vec: Vec<(i64, Hydra)> = Vec::new();
        for _ in 0..copies {
            copies_vec.push((k, Hydra(tail.clone())));
        }
        return Ok(replace_at(h, parent_path, &Hydra(copies_vec)));
    }

    // Rule 4: nearest ancestor ψ^H_{n-1} (k = target_n - 1).
    // P[n] = #_1(ψ_k^H(#_2(ψ_k^H(#_2(…ψ_k^H(0)…))))) with n layers of ψ_k:
    // X_0 = 0; X_{i+1} = ψ^H_k(#_2(X_i)); result = X_n. The innermost layer
    // is ψ^H_k(0) — never the original ψ^H_{k+1}(0). #_2 is the wrapper's
    // argument with the target ψ^H_{k+1}(0) replaced by the hole, so any ψ
    // chain above the target (e.g. p3( in p2(p3(p3(0))+p3(p3(0)))) is kept.
    let k = target_n - 1;
    let mut ancestor_paths: Vec<Vec<usize>> = Vec::new();
    let mut cur = parent_path.to_vec();
    while !cur.is_empty() {
        ancestor_paths.push(cur.clone());
        cur.pop();
    }
    let wrapper_path = ancestor_paths
        .iter()
        .find(|p| summand_at(h, p).0 == k)
        .ok_or_else(|| format!("No enclosing ψ^H_{} for expansion", k))?
        .clone();
    let wrapper_arg = summand_at(h, &wrapper_path).1.clone();
    let target_subpath = &path[wrapper_path.len()..];

    let mut x = hydra_zero();
    for _ in 0..copies {
        let inner = replace_at(&wrapper_arg, target_subpath, &x);
        x = Hydra(vec![(k, inner)]);
    }
    Ok(replace_at(h, &wrapper_path, &x))
}

// ════════════════════════════════════════════════════════════════
// Term (BOCF) embedding
// ════════════════════════════════════════════════════════════════

fn nat_term(k: i64) -> Term {
    let mut r = zero();
    for _ in 0..k {
        r = add(&r, &one());
    }
    r
}

/// ψ^H_n(A) ↔ ψ_{n-1}(A). Subscript 0 means plain ψ.
pub fn hydra_to_term(h: &Hydra) -> Term {
    let mut r = zero();
    for (n, arg) in &h.0 {
        let sub = nat_term(n - 1);
        let node = t(sub, hydra_to_term(arg), zero());
        r = add(&r, &node);
    }
    r
}

fn nat_value(t: &Term) -> i64 {
    if !is_finite_nat(t) {
        return -1;
    }
    length1(t) as i64
}

/// Convert a term to hydra form. Every ψ subscript must be a finite natural
/// number k, mapped to ψ^H_{k+1}. Returns Err for terms above the hydra range.
pub fn term_to_hydra(t: &Term) -> Result<Hydra, String> {
    if is_zero(t) {
        return Ok(hydra_zero());
    }
    let node = t.as_ref().unwrap();
    let k = nat_value(&node.a);
    if k < 0 {
        return Err("Term is not PSS-Hydra-expressible (subscript not a natural number)".to_string());
    }
    let mut terms = vec![(k + 1, term_to_hydra(&node.b)?)];
    let rest = term_to_hydra(&node.c)?;
    terms.extend(rest.0);
    Ok(Hydra(terms))
}

/// Convenience: hydra → standard-form BOCF term.
pub fn hydra_to_bocf(h: &Hydra) -> Term {
    standard_form(&hydra_to_term(h))
}

/// Convenience: BOCF term → standard hydra form (standard_form + 补层).
/// Err if above hydra range.
pub fn bocf_to_hydra(t: &Term) -> Result<Hydra, String> {
    Ok(fill_layers(&term_to_hydra(&standard_form(t))?))
}

// ════════════════════════════════════════════════════════════════
// PSS Hydra ↔ BMS
// ════════════════════════════════════════════════════════════════

/// Standard BMS of a hydra: hydra → standard-form BOCF → full BOCF→BMS
/// search algorithm. The direct structural embedding is not always standard.
pub fn hydra_to_bms(h: &Hydra) -> Result<Matrix, String> {
    crate::parser::term_to_bms(&hydra_to_bocf(h), &mut |_| {})
}

/// 2-row BMS → hydra. First-row values are the nesting depth; each (0,k)
/// column is ψ^H_{k+1}(subtree). Errors on invalid 2-row matrices.
pub fn bms_to_hydra(m: &Matrix) -> Result<Hydra, String> {
    for col in m {
        if col.len() < 2 {
            return Err("BMS input must be 2-row".to_string());
        }
        if col[0] < 0 || col[1] < 0 {
            return Err("BMS input has negative entries".to_string());
        }
    }
    bms_to_hydra_aux(m)
}

fn bms_to_hydra_aux(m: &Matrix) -> Result<Hydra, String> {
    if m.is_empty() {
        return Ok(hydra_zero());
    }
    let mut terms: Vec<(i64, Hydra)> = Vec::new();
    let mut i = 0usize;
    while i < m.len() {
        let col = &m[i];
        if col[0] != 0 {
            return Err("BMS input is not a valid 2-row matrix".to_string());
        }
        let n = (col[1] + 1) as i64;
        let mut j = i + 1;
        while j < m.len() && m[j][0] != 0 {
            j += 1;
        }
        // Columns i+1..j belong to this ψ's argument, one level deeper.
        let arg: Matrix = m[i + 1..j]
            .iter()
            .map(|c| vec![c[0] - 1, c[1]])
            .collect();
        let arg_h = bms_to_hydra_aux(&arg)?;
        terms.push((n, arg_h));
        i = j;
    }
    Ok(Hydra(terms))
}

// ════════════════════════════════════════════════════════════════
// Normalization (补层)
// ════════════════════════════════════════════════════════════════

/// Level of a hydra sum's top-level summands (max subscript); 0 for zero.
fn top_level(h: &Hydra) -> i64 {
    h.0.iter().map(|(n, _)| *n).max().unwrap_or(0)
}

/// Fill in missing subscript layers so the expression is legal
/// (level(A) ≤ n+1 for every ψ^H_n(A)). Value-preserving: in the BOCF
/// convention ψ_{n-1}(X) = X for X ≥ Ω_n, so inserting the intermediate
/// ψ^H layers does not change the ordinal.
///
/// If every summand of A is above level n+1, the whole sum is wrapped
/// (p1(p3+p3) → p1(p2(p3+p3))). Otherwise the offending summands are wrapped
/// as one block sharing a single (n+1) layer, with intermediate layers filled
/// inside the block (p1(p4+p3+p2) → p1(p2(p3(p4)+p3)+p2)).
pub fn fill_layers(h: &Hydra) -> Hydra {
    Hydra(
        h.0.iter()
            .map(|(n, arg)| (*n, fill_arg(*n, &fill_layers(arg))))
            .collect(),
    )
}

fn fill_arg(n: i64, arg: &Hydra) -> Hydra {
    let lvl = top_level(arg);
    if lvl <= n + 1 {
        return arg.clone();
    }
    let mut offending = Vec::new();
    let mut legal = Vec::new();
    for (m, a) in &arg.0 {
        if *m > n + 1 {
            offending.push((*m, a.clone()));
        } else {
            legal.push((*m, a.clone()));
        }
    }
    // Wrap the offending summands as one block sharing the (n+1) layer;
    // recursion fills intermediate layers inside the block. If every summand
    // is offending, `legal` is empty and this reduces to wrapping the whole sum.
    let mut result = vec![(n + 1, fill_arg(n + 1, &Hydra(offending)))];
    result.extend(legal);
    Hydra(result)
}

/// Standard hydra form: hydra → BOCF standard form → back → fill layers.
pub fn normalize_hydra(h: &Hydra) -> Hydra {
    match term_to_hydra(&hydra_to_bocf(h)) {
        Ok(t) => fill_layers(&t),
        Err(_) => fill_layers(h),
    }
}

// ════════════════════════════════════════════════════════════════
// HPrSS
// ════════════════════════════════════════════════════════════════

/// Expand an HPrSS sequence: S[n].
pub fn expand_hprss(seq: &[i32], n: i32) -> Vec<i32> {
    if seq.is_empty() {
        return Vec::new();
    }
    let len = seq.len();
    let last = seq[len - 1];
    if last == 1 {
        // successor: drop the last term
        return seq[..len - 1].to_vec();
    }
    // Parents and differences.
    let mut parent: Vec<Option<usize>> = vec![None; len];
    let mut diff: Vec<i64> = vec![0; len];
    for i in 0..len {
        let mut p: Option<usize> = None;
        for j in (0..i).rev() {
            if seq[j] < seq[i] {
                p = Some(j);
                break;
            }
        }
        parent[i] = p;
        diff[i] = match p {
            Some(pj) => (seq[i] - seq[pj]) as i64,
            None => seq[i] as i64,
        };
    }
    // Bad root: ancestor chain of the last term.
    let dn = diff[len - 1];
    let r = if dn == 1 {
        parent[len - 1].expect("last term of a limit HPrSS has a parent")
    } else {
        let mut cur = parent[len - 1];
        let mut found = None;
        while let Some(c) = cur {
            if diff[c] < dn {
                found = Some(c);
                break;
            }
            cur = parent[c];
        }
        found.expect("HPrSS limit has a bad root")
    };
    let delta = (seq[len - 1] - seq[r]) as i64 - 1;
    let good = seq[..r].to_vec();
    let bad = seq[r..len - 1].to_vec();
    let mut out = good;
    let copies = n.max(0) as i64;
    for t in 0..=copies {
        for &v in &bad {
            out.push(v + (t * delta) as i32);
        }
    }
    out
}

/// HPrSS → PSS Hydra: split at the first term ≤ the current pivot.
pub fn hprss_to_hydra(seq: &[i32]) -> Hydra {
    let mut terms: Vec<(i64, Hydra)> = Vec::new();
    let mut i = 0usize;
    while i < seq.len() {
        let a = seq[i] as i64;
        let mut j = i + 1;
        while j < seq.len() && seq[j] > seq[i] {
            j += 1;
        }
        let inner: Vec<i32> = seq[i + 1..j].iter().map(|v| v - seq[i]).collect();
        terms.push((a, hprss_to_hydra(&inner)));
        i = j;
    }
    Hydra(terms)
}

/// PSS Hydra → HPrSS: LP(ψ^H_x(H)) = (x, LP(H) + x).
pub fn hydra_to_hprss(h: &Hydra) -> Vec<i32> {
    let mut out = Vec::new();
    for (n, arg) in &h.0 {
        out.push(*n as i32);
        for v in hydra_to_hprss(arg) {
            out.push(v + *n as i32);
        }
    }
    out
}

/// Standard HPrSS of a hydra: hydra → BOCF standard form → back to the
/// unfilled hydra → LP. Applying 补层 before LP would give the 0-Y sequence
/// instead of the HPrSS one (p1(p3(0)) → HPrSS (1,4) vs 0-Y (1,3,6)).
pub fn hydra_to_hprss_standard(h: &Hydra) -> Vec<i32> {
    match term_to_hydra(&hydra_to_bocf(h)) {
        Ok(unfilled) => hydra_to_hprss(&unfilled),
        Err(_) => hydra_to_hprss(h),
    }
}

/// Convenience: HPrSS → standard BMS.
pub fn hprss_to_bms(seq: &[i32]) -> Result<Matrix, String> {
    hydra_to_bms(&hprss_to_hydra(seq))
}

/// HPrSS mountain diagram (definition 14.5). Two layers:
/// - bottom: the sequence with parents (rightmost smaller term to the left),
///   stored as (value, parent offset);
/// - top: the difference sequence, d_i = a_i − a_{p(i)} (or a_i without a
///   parent). The parent of d_i is the rightmost j < i with d_j < d_i and a_j
///   an ancestor of a_i.
pub fn build_hprss_mountain(seq: &[i32]) -> crate::Mountain {
    let len = seq.len();
    if len == 0 {
        return Vec::new();
    }
    let mut parents = vec![0i32; len];
    let mut diffs = vec![0i32; len];
    for i in 0..len {
        let mut p = i as i64 - 1;
        while p >= 0 && seq[p as usize] >= seq[i] {
            p -= 1;
        }
        if p >= 0 {
            parents[i] = i as i32 - p as i32;
            diffs[i] = seq[i] - seq[p as usize];
        } else {
            diffs[i] = seq[i];
        }
    }
    let mut d_parents = vec![0i32; len];
    for i in 0..len {
        for j in (0..i).rev() {
            if diffs[j] < diffs[i] && is_hprss_ancestor(&parents, j, i) {
                d_parents[i] = i as i32 - j as i32;
                break;
            }
        }
    }
    vec![
        seq.iter().zip(&parents).map(|(&v, &p)| (v, p)).collect(),
        diffs.iter().zip(&d_parents).map(|(&v, &p)| (v, p)).collect(),
    ]
}

fn is_hprss_ancestor(parents: &[i32], j: usize, i: usize) -> bool {
    let mut cur = i;
    loop {
        if cur == j {
            return true;
        }
        let off = parents[cur];
        if off <= 0 {
            return false;
        }
        cur -= off as usize;
    }
}

// ════════════════════════════════════════════════════════════════
// LPrSS (Long Primitive Sequence System)
// ════════════════════════════════════════════════════════════════

/// Expand an LPrSS sequence: S[m] = G + B + B_1 + ... + B_m (def. 14.1/14.2).
/// Bad root = rightmost term left of the last that is smaller than it;
/// d = last − root; B_t = B + t(d−1). (#,1) is a successor.
pub fn expand_lprss(seq: &[i32], n: i32) -> Vec<i32> {
    if seq.is_empty() {
        return Vec::new();
    }
    let len = seq.len();
    let last = seq[len - 1];
    if last == 1 {
        // successor: drop the last term
        return seq[..len - 1].to_vec();
    }
    let mut root = None;
    for i in (0..len - 1).rev() {
        if seq[i] < last {
            root = Some(i);
            break;
        }
    }
    let root = match root {
        Some(r) => r,
        None => return seq[..len - 1].to_vec(), // no smaller term: treat as successor
    };
    let delta = last - seq[root] - 1;
    let good = &seq[..root];
    let bad = &seq[root..len - 1];
    let copies = n.max(0) as usize;
    let mut out = good.to_vec();
    for i in 0..=copies {
        for &v in bad {
            out.push(v + (i as i32) * delta);
        }
    }
    out
}

/// LPrSS mountain diagram (def. 14.3). Two layers:
/// - bottom: the sequence with parents (rightmost smaller term to the left);
/// - top: the difference sequence, d_i = a_i − a_{p(i)} (or a_i without a
///   parent). The parent of d_i is the difference term of a_i's parent, so it
///   shares a_i's parent offset.
pub fn build_lprss_mountain(seq: &[i32]) -> crate::Mountain {
    let len = seq.len();
    if len == 0 {
        return Vec::new();
    }
    let mut parents = vec![0i32; len];
    let mut diffs = vec![0i32; len];
    for i in 0..len {
        let mut p = i as i64 - 1;
        while p >= 0 && seq[p as usize] >= seq[i] {
            p -= 1;
        }
        if p >= 0 {
            parents[i] = i as i32 - p as i32;
            diffs[i] = seq[i] - seq[p as usize];
        } else {
            diffs[i] = seq[i];
        }
    }
    vec![
        seq.iter().zip(&parents).map(|(&v, &p)| (v, p)).collect(),
        diffs.iter().zip(&parents).map(|(&v, &p)| (v, p)).collect(),
    ]
}

/// LPrSS → PSS Hydra.
///
/// A `1` inside the tail is an ordinal-addition split: the value of
/// (1, s₁, 1, s₂, ...) is FULL(1, s₁) + FULL(1, s₂) + ... Each 1-free chunk s
/// contributes ψ^H_1(S(s)). The tail s splits into segments at every i with
/// s[i] ≤ the current segment's head h; a segment [h, τ]:
/// - h = 2: ψ^H_1(S(τ − 1)) with the head dropped;
/// - h ≥ 3: ψ^H_2((h−3)·ψ^H_2(0) + X), where τ' = τ − (h−1) and
///   X = S(τ') when τ' is empty or starts with 2, ψ^H_1(S(τ')) otherwise.
/// Here τ − c shifts every subtail term by −c.
pub fn lprss_to_hydra(seq: &[i32]) -> Hydra {
    if seq.is_empty() {
        return hydra_zero();
    }
    let mut terms: Vec<(i64, Hydra)> = Vec::new();
    let mut chunk: Vec<i32> = Vec::new();
    for &t in &seq[1..] {
        if t == 1 {
            terms.push((1, lprss_tail_hydra(&chunk)));
            chunk.clear();
        } else {
            chunk.push(t);
        }
    }
    terms.push((1, lprss_tail_hydra(&chunk)));
    Hydra(terms)
}

fn lprss_tail_hydra(tail: &[i32]) -> Hydra {
    let mut terms: Vec<(i64, Hydra)> = Vec::new();
    let mut i = 0usize;
    while i < tail.len() {
        let h = tail[i] as i64;
        let mut j = i + 1;
        while j < tail.len() && (tail[j] as i64) > h {
            j += 1;
        }
        terms.extend(lprss_seg_value(h, &tail[i + 1..j]).0);
        i = j;
    }
    Hydra(terms)
}

fn lprss_seg_value(h: i64, sub: &[i32]) -> Hydra {
    if h == 2 {
        let shifted: Vec<i32> = sub.iter().map(|&t| t - 1).collect();
        return Hydra(vec![(1, lprss_tail_hydra(&shifted))]);
    }
    let off = (h - 1) as i32;
    let tau: Vec<i32> = sub.iter().map(|&t| t - off).collect();
    let inner = if tau.is_empty() || tau[0] == 2 {
        lprss_tail_hydra(&tau)
    } else {
        Hydra(vec![(1, lprss_tail_hydra(&tau))])
    };
    let mut arg = inner;
    for _ in 0..(h - 3) {
        arg.0.insert(0, (2, hydra_zero()));
    }
    Hydra(vec![(2, arg)])
}

/// PSS Hydra → LPrSS (the standard LPrSS form of the ordinal).
/// Errors for ordinals outside the LPrSS range (above the limit φ(ω,0),
/// where a ψ^H_n with n ≥ 3 would be needed).
pub fn hydra_to_lprss(h: &Hydra) -> Result<Vec<i32>, String> {
    if h.0.is_empty() {
        return Ok(Vec::new());
    }
    let mut seq = vec![1i32];
    let mut first = true;
    for (n, arg) in &h.0 {
        if *n != 1 {
            return Err("Ordinal is not LPrSS-expressible (beyond the limit φ(ω,0))".to_string());
        }
        if !first {
            seq.push(1);
        }
        first = false;
        seq.extend(lprss_decode_tail(arg)?);
    }
    Ok(seq)
}

fn lprss_decode_tail(s: &Hydra) -> Result<Vec<i32>, String> {
    let mut tail: Vec<i32> = Vec::new();
    for (n, arg) in &s.0 {
        tail.extend(lprss_decode_seg(*n, arg)?);
    }
    Ok(tail)
}

fn lprss_decode_seg(n: i64, arg: &Hydra) -> Result<Vec<i32>, String> {
    if n == 1 {
        let mut seg = vec![2];
        for v in lprss_decode_tail(arg)? {
            seg.push(v + 1);
        }
        return Ok(seg);
    }
    if n != 2 {
        return Err("Ordinal is not LPrSS-expressible (beyond the limit φ(ω,0))".to_string());
    }
    let mut c = 0usize;
    while c < arg.0.len() && arg.0[c] == (2, hydra_zero()) {
        c += 1;
    }
    let h = 3 + c as i64;
    let x = Hydra(arg.0[c..].to_vec());
    // X is 0 (standalone), a sum of ψ^H_1 terms (τ₁ = 2), or a single ψ^H_1
    // whose argument starts with ψ^H_2 (τ₁ ≥ 3). Anything else means the
    // ordinal reaches or exceeds the LPrSS limit φ(ω,0) = ψ(Ω^ω).
    let tau = if x.0.is_empty() {
        Vec::new()
    } else if x.0[0].0 == 2 || x.0.iter().any(|(n, _)| *n != 1) {
        return Err("Ordinal is beyond the LPrSS limit φ(ω,0)".to_string());
    } else if x.0.len() == 1 && !x.0[0].1 .0.is_empty() && x.0[0].1 .0[0].0 == 2 {
        // Prefer the shorter standard decoding: p1(S(τ)) with τ₁ ≥ 3.
        lprss_decode_tail(&x.0[0].1)?
    } else {
        lprss_decode_tail(&x)?
    };
    let mut seg = vec![h as i32];
    for v in tau {
        seg.push(v + h as i32 - 1);
    }
    Ok(seg)
}

// ════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> Hydra {
        parse_hydra(s).unwrap()
    }

    fn fmt(h: &Hydra) -> String {
        format_hydra(h)
    }

    #[test]
    fn parse_basic() {
        assert_eq!(fmt(&h("0")), "0");
        assert_eq!(fmt(&h("p1(0)")), "p1(0)");
        assert_eq!(fmt(&h("p1")), "p1(0)");
        assert_eq!(fmt(&h("p1(p2(0)+p2)")), "p1(p2(0)+p2(0))");
        assert_eq!(fmt(&h("p1 + p2")), "p1(0)+p2(0)");
        // ψ^H_n input parses identically to pn
        assert_eq!(fmt(&h("ψ^H_1(ψ^H_2(0))")), "p1(p2(0))");
        assert_eq!(fmt(&h("ψH1(0)")), "p1(0)");
        assert_eq!(h("ψ^H_1(p1(0))"), h("p1(p1(0))"));
        // ψ format round trips through the parser
        assert_eq!(h("ψ^H_1(ψ^H_3(ψ^H_3(0)+ψ^H_2(0))+ψ^H_2(ψ^H_2(ψ^H_2(0))))"), h("p1(p3(p3(0)+p2(0))+p2(p2(p2(0))))"));
        assert_eq!(format_hydra_psi(&h("p1(p2(0)+p1(0))")), "ψ^H_1(ψ^H_2(0)+ψ^H_1(0))");
        assert_eq!(format_hydra_psi(&h("0")), "0");
        assert_eq!(fmt(&h("p2(0)+p1(p2(0))")), "p2(0)+p1(p2(0))");
        assert_eq!(fmt(&h("p1(p2(0)+p2(0))")), "p1(p2(0)+p2(0))");
        assert!(parse_hydra("p0(0)").is_err());
        assert!(parse_hydra("x1").is_err());
        assert!(parse_hydra("p1(").is_err());
    }

    #[test]
    fn hydra_expand_rule2() {
        // ψ^H_1(ψ^H_2(ψ^H_2(0))) + ψ^H_1(0) → predecessor
        let e = expand_hydra(&h("p1(p2(p2(0)))+p1(0)"), 3).unwrap();
        assert_eq!(fmt(&e), "p1(p2(p2(0)))");
    }

    #[test]
    fn hydra_expand_rule3_doc_example2() {
        let e = expand_hydra(&h("p1(p2(p3(p2(0))+p1(0)))"), 3).unwrap();
        assert_eq!(fmt(&e), "p1(p2(p3(p2(0)))+p2(p3(p2(0)))+p2(p3(p2(0))))");
    }

    #[test]
    fn hydra_expand_rule4_doc_example3() {
        let e = expand_hydra(&h("p1(p2(0)+p2(0))"), 2).unwrap();
        assert_eq!(fmt(&e), "p1(p2(0)+p1(p2(0)))");
    }

    #[test]
    fn hydra_expand_rule4_doc_example4() {
        // Doc example 4 result, reproduced exactly with X_0 = 0 and
        // X_{i+1} = ψ^H_k(#_2(X_i)): 4 layers of ψ^H_2, innermost #_2(0).
        let e = expand_hydra(&h("p1(p2(p3(p3(0))+p3(p3(0))))"), 4).unwrap();
        assert_eq!(
            fmt(&e),
            "p1(p2(p3(p3(0))+p3(p2(p3(p3(0))+p3(p2(p3(p3(0))+p3(p2(p3(p3(0))+p3(0)))))))))"
        );
    }

    #[test]
    fn hydra_expand_rule4_empty_hash2() {
        // p1(p2(0)) = ε_0: the innermost layer is ψ^H_1(0), never ψ^H_2(0)
        assert_eq!(fmt(&expand_hydra(&h("p1(p2(0))"), 1).unwrap()), "p1(0)");
        assert_eq!(fmt(&expand_hydra(&h("p1(p2(0))"), 2).unwrap()), "p1(p1(0))");
        assert_eq!(fmt(&expand_hydra(&h("p1(p2(0))"), 3).unwrap()), "p1(p1(p1(0)))");
        // ψ^H_1(ψ^H_2(ψ^H_3(0))): target is the p3 itself, #_2 empty → n layers of ψ^H_2
        assert_eq!(fmt(&expand_hydra(&h("p1(p2(p3(0)))"), 2).unwrap()), "p1(p2(p2(0)))");
    }

    #[test]
    fn hydra_expand_zero() {
        assert_eq!(fmt(&expand_hydra(&h("0"), 3).unwrap()), "0");
        assert_eq!(fmt(&expand_hydra(&h("p1(0)"), 3).unwrap()), "0");
    }

    #[test]
    fn hprss_expand_doc_example() {
        // (1,4,6,6)[2] = (1,4,6, 5,8,10, 9,12,14)
        let e = expand_hprss(&[1, 4, 6, 6], 2);
        assert_eq!(e, vec![1, 4, 6, 5, 8, 10, 9, 12, 14]);
        // (1,2,2)[2] = (1,2,1,2,1,2)
        assert_eq!(expand_hprss(&[1, 2, 2], 2), vec![1, 2, 1, 2, 1, 2]);
        // (1,2,3)[2] = (1,2,2,2)
        assert_eq!(expand_hprss(&[1, 2, 3], 2), vec![1, 2, 2, 2]);
        // successor
        assert_eq!(expand_hprss(&[1, 2, 1], 5), vec![1, 2]);
        // empty
        assert_eq!(expand_hprss(&[], 5), Vec::<i32>::new());
    }

    #[test]
    fn hprss_hydra_round_trip_doc_example() {
        // (1,4,7,6,3,5,7) ↔ ψ^H_1(ψ^H_3(ψ^H_3(0)+ψ^H_2(0)) + ψ^H_2(ψ^H_2(ψ^H_2(0))))
        let seq = [1, 4, 7, 6, 3, 5, 7];
        assert_eq!(fmt(&hprss_to_hydra(&seq)), "p1(p3(p3(0)+p2(0))+p2(p2(p2(0))))");
        assert_eq!(hydra_to_hprss(&hprss_to_hydra(&seq)), seq.to_vec());
        // (3,2) ↔ p3(0)+p2(0)
        assert_eq!(fmt(&hprss_to_hydra(&[3, 2])), "p3(0)+p2(0)");
        // (2,4) ↔ p2(p2(0))
        assert_eq!(fmt(&hprss_to_hydra(&[2, 4])), "p2(p2(0))");
        // (1,2) ↔ p1(p1(0)); p1(p2(0)) ↔ (1,3)
        assert_eq!(fmt(&hprss_to_hydra(&[1, 2])), "p1(p1(0))");
        assert_eq!(hydra_to_hprss(&h("p1(p2(0))")), vec![1, 3]);
        assert_eq!(hydra_to_hprss(&h("p3(0)+p2(0)")), vec![3, 2]);
    }

    #[test]
    fn hydra_to_hprss_standard_test() {
        // BHO: unfilled p1(p3(0)) gives the standard HPrSS (1,4); the LP of
        // the 补层'd p1(p2(p3(0))) is the 0-Y sequence (1,3,6) instead
        assert_eq!(hydra_to_hprss_standard(&h("p1(p3(0))")), vec![1, 4]);
        assert_eq!(hydra_to_hprss(&h("p1(p2(p3(0)))")), vec![1, 3, 6]);
        // already-standard hydras round trip through the standard path
        assert_eq!(hydra_to_hprss_standard(&h("p1(p2(0))")), vec![1, 3]);
        let seq = [1, 4, 7, 6, 3, 5, 7];
        assert_eq!(hydra_to_hprss_standard(&hprss_to_hydra(&seq)), seq.to_vec());
        // mixed offenders: standard form of p1(p4+p3+p2) is unfilled, LP = (1,5,4,3)
        assert_eq!(hydra_to_hprss_standard(&h("p1(p4(0)+p3(0)+p2(0))")), vec![1, 5, 4, 3]);
    }

    #[test]
    fn hprss_mountain_doc_example() {
        // (1,4,6,6): parents (4→1, 6→4, last 6→4), differences (1,3,2,2) as in the doc
        let m = build_hprss_mountain(&[1, 4, 6, 6]);
        assert_eq!(m[0], vec![(1, 0), (4, 1), (6, 1), (6, 2)]);
        assert_eq!(m[1], vec![(1, 0), (3, 1), (2, 2), (2, 3)]);
        // (1,4,7,6,3,5,7): doc example
        let m = build_hprss_mountain(&[1, 4, 7, 6, 3, 5, 7]);
        assert_eq!(m[0], vec![(1, 0), (4, 1), (7, 1), (6, 2), (3, 4), (5, 1), (7, 1)]);
        assert_eq!(m[1], vec![(1, 0), (3, 1), (3, 2), (2, 3), (2, 4), (2, 5), (2, 6)]);
        assert_eq!(build_hprss_mountain(&[]), Vec::<Vec<(i32, i32)>>::new());
        // a = [1,2]: d = [1,1], d2 has no smaller d to its left
        let m = build_hprss_mountain(&[1, 2]);
        assert_eq!(m[1], vec![(1, 0), (1, 0)]);
    }

    #[test]
    fn lprss_expand() {
        // (1,4,6,6): root = 4 (rightmost < 6), d = 2, δ = 1 → G + B + B1 + B2
        assert_eq!(expand_lprss(&[1, 4, 6, 6], 2), vec![1, 4, 6, 5, 7, 6, 8]);
        // (1,2,2): root = 1, d = 1, δ = 0 → copies of (1,2)
        assert_eq!(expand_lprss(&[1, 2, 2], 2), vec![1, 2, 1, 2, 1, 2]);
        // successor
        assert_eq!(expand_lprss(&[1, 2, 1], 5), vec![1, 2]);
        // (1,3): root = 1, δ = 1 → [1] + [2] = (1,2)
        assert_eq!(expand_lprss(&[1, 3], 1), vec![1, 2]);
        // empty
        assert_eq!(expand_lprss(&[], 3), Vec::<i32>::new());
    }

    #[test]
    fn lprss_mountain() {
        // (1,4,6,6): layer 0 parents are standard (rightmost smaller: 6→4);
        // d-parents share a_i's parent offset (6→4, so d_3's parent is d_1)
        let m = build_lprss_mountain(&[1, 4, 6, 6]);
        assert_eq!(m[0], vec![(1, 0), (4, 1), (6, 1), (6, 2)]);
        assert_eq!(m[1], vec![(1, 0), (3, 1), (2, 1), (2, 2)]);
        assert_eq!(build_lprss_mountain(&[]), Vec::<Vec<(i32, i32)>>::new());
    }

    #[test]
    fn lprss_to_hydra_values() {
        // (1,2) = ω, (1,3) = ε₀, (1,4) = Γ₀
        assert_eq!(fmt(&lprss_to_hydra(&[1, 2])), "p1(p1(0))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3])), "p1(p2(0))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 4])), "p1(p2(p2(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 5])), "p1(p2(p2(0)+p2(0)))");
        // heads 2 and 3 with subtails
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 2, 3])), "p1(p2(0)+p1(p1(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 2, 4])), "p1(p2(0)+p1(p2(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 2, 4, 3, 4])), "p1(p2(0)+p1(p2(0)+p1(p1(0))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 4])), "p1(p2(p1(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 4, 5])), "p1(p2(p1(p1(0))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 4, 4])), "p1(p2(p1(0)+p1(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 5])), "p1(p2(p1(p2(0))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 5, 6])), "p1(p2(p1(p2(p1(0)))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 5, 7])), "p1(p2(p1(p2(p1(p2(0))))))");
        // head 4
        assert_eq!(fmt(&lprss_to_hydra(&[1, 4, 5])), "p1(p2(p2(0)+p1(0)))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 4, 6])), "p1(p2(p2(0)+p1(p2(0))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 4, 6, 8])), "p1(p2(p2(0)+p1(p2(p1(p2(0))))))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 4, 4])), "p1(p2(p2(0))+p2(p2(0)))");
        // sums of segments
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 3])), "p1(p2(0)+p2(0))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 4, 3])), "p1(p2(p1(0))+p2(0))");
        assert_eq!(fmt(&lprss_to_hydra(&[1, 3, 2, 4, 2])), "p1(p2(0)+p1(p2(0))+p1(0))");
        // segment values round trip through the BOCF embedding
        let h = lprss_to_hydra(&[1, 4, 6, 6]);
        assert!(is_legal_hydra(&normalize_hydra(&h)));
    }

    #[test]
    fn lprss_round_trip() {
        for seq in [
            vec![1],
            vec![1, 1],
            vec![1, 2],
            vec![1, 2, 2, 1, 2],
            vec![1, 2, 3, 1, 2, 3],
            vec![1, 3],
            vec![1, 3, 2, 3],
            vec![1, 3, 2, 4, 3, 4],
            vec![1, 3, 4],
            vec![1, 3, 4, 4],
            vec![1, 3, 5],
            vec![1, 3, 5, 6, 7],
            vec![1, 3, 5, 7],
            vec![1, 4],
            vec![1, 4, 4],
            vec![1, 4, 5, 6],
            vec![1, 4, 6, 8],
            vec![1, 5],
        ] {
            let h = lprss_to_hydra(&seq);
            let back = hydra_to_lprss(&h).unwrap();
            assert_eq!(back, seq, "round trip failed for {:?}", seq);
        }
        // standard forms decode to the shorter representation
        assert_eq!(hydra_to_lprss(&h("p1(p2(p1(p2(0))))")).unwrap(), vec![1, 3, 5]);
        // addition splits survive
        assert_eq!(hydra_to_lprss(&h("p1(p1(0)+p1(0))+p1(p1(0))")).unwrap(), vec![1, 2, 2, 1, 2]);
        assert_eq!(hydra_to_lprss(&h("p1(0)+p1(0)")).unwrap(), vec![1, 1]);
        // outside the LPrSS range
        assert!(hydra_to_lprss(&h("p1(p3(0))")).is_err());
        assert!(hydra_to_lprss(&h("p2(0)")).is_err());
        // the limit φ(ω,0) = ψ(Ω^ω) and everything above it is not expressible
        assert!(hydra_to_lprss(&h("p1(p2(p2(p1(0))))")).is_err());
        assert!(hydra_to_lprss(&h("p1(p2(p2(p2(0))))")).is_err());
        assert!(hydra_to_lprss(&h("p1(p2(p1(0)+p2(0)))")).is_err());
        // through the standard form: hydra → bocf → hydra → lprss
        for s in ["p1(p2(p2(0)))", "p1(p2(p1(p2(0))))", "p1(p2(p2(0)+p1(0)))+p1(p1(0))"] {
            let std = term_to_hydra(&hydra_to_bocf(&h(s))).unwrap();
            let l = hydra_to_lprss(&std).unwrap();
            assert_eq!(hydra_to_lprss(&lprss_to_hydra(&l)).unwrap(), l);
        }
    }

    #[test]
    fn term_embedding() {
        // ψ^H_1(ψ^H_2(0)) ↔ ψ_0(Ω) = ε_0
        let tm = hydra_to_term(&h("p1(p2(0))"));
        assert!(eq(&tm, &epsilon0()));
        // ψ_0(ψ_2(0)) = BHO ↔ p1(p2(p3(0))) (standard form after 补层)
        assert_eq!(fmt(&bocf_to_hydra(&bho()).unwrap()), "p1(p2(p3(0)))");
        // round trip: hydra → term → standard → hydra is legal and idempotent
        let original = h("p1(p3(p3(0)+p2(0))+p2(p2(p2(0))))");
        let rt = bocf_to_hydra(&hydra_to_term(&original)).unwrap();
        assert!(is_legal_hydra(&rt));
        assert_eq!(fmt(&normalize_hydra(&rt)), fmt(&rt));
        // above hydra range: subscript ω
        assert!(term_to_hydra(&crate::term::t(zero(), t(omega(), zero(), zero()), zero())).is_err());
    }

    #[test]
    fn fill_layers_examples() {
        // p1(p3) → p1(p2(p3))
        assert_eq!(fmt(&fill_layers(&h("p1(p3(0))"))), "p1(p2(p3(0)))");
        // p1(p3+p2) → p1(p2(p3)+p2)
        assert_eq!(fmt(&fill_layers(&h("p1(p3(0)+p2(0))"))), "p1(p2(p3(0))+p2(0))");
        // p1(p3+p3) → p1(p2(p3+p3))
        assert_eq!(fmt(&fill_layers(&h("p1(p3(0)+p3(0))"))), "p1(p2(p3(0)+p3(0)))");
        // deep gap: p1(p5) → p1(p2(p3(p4(p5))))
        assert_eq!(fmt(&fill_layers(&h("p1(p5(0))"))), "p1(p2(p3(p4(p5(0)))))");
        // mixed with deep gap: p1(p4+p2) → p1(p2(p3(p4))+p2)
        assert_eq!(fmt(&fill_layers(&h("p1(p4(0)+p2(0))"))), "p1(p2(p3(p4(0)))+p2(0))");
        // consecutive offenders share one fill layer: p1(p4+p3+p2) → p1(p2(p3(p4)+p3)+p2)
        assert_eq!(fmt(&fill_layers(&h("p1(p4(0)+p3(0)+p2(0))"))), "p1(p2(p3(p4(0))+p3(0))+p2(0))");
        // offenders with a gap share too, gap gets its own layer: p1(p5+p3+p2) → p1(p2(p3(p4(p5))+p3)+p2)
        assert_eq!(fmt(&fill_layers(&h("p1(p5(0)+p3(0)+p2(0))"))), "p1(p2(p3(p4(p5(0)))+p3(0))+p2(0))");
        // nested gap inside a summand: p1(p2(p4)) → p1(p2(p3(p4)))
        assert_eq!(fmt(&fill_layers(&h("p1(p2(p4(0)))"))), "p1(p2(p3(p4(0))))");
        // already legal → identity
        assert_eq!(fmt(&fill_layers(&h("p1(p2(0)+p1(0))"))), "p1(p2(0)+p1(0))");
        // output is always legal
        for s in ["p1(p3(0))", "p1(p5(0))", "p1(p3(0)+p2(0))", "p1(p4(0)+p2(0))", "p2(p4(0)+p2(0))", "p1(p2(p4(0)))"] {
            let f = fill_layers(&h(s));
            assert!(is_legal_hydra(&f), "not legal after fill: {} -> {}", s, fmt(&f));
            // idempotent
            assert_eq!(fmt(&fill_layers(&f)), fmt(&f));
        }
    }

    #[test]
    fn normalize_hprss_pipeline() {
        // hprss → hydra → standard form → 补层
        let raw = hprss_to_hydra(&[1, 4, 7, 6, 3, 5, 7]);
        assert_eq!(fmt(&raw), "p1(p3(p3(0)+p2(0))+p2(p2(p2(0))))");
        let norm = normalize_hydra(&raw);
        // levels of p1's argument are 3 and 2 → mixed case: wrap only the p3 summand
        assert_eq!(fmt(&norm), "p1(p2(p3(p3(0)+p2(0)))+p2(p2(p2(0))))");
        assert!(is_legal_hydra(&norm));
        // BHO from BOCF standard form ψ_0(Ω_2) needs 补层
        let norm_bho = normalize_hydra(&h("p1(p3(0))"));
        assert_eq!(fmt(&norm_bho), "p1(p2(p3(0)))");
        assert!(is_legal_hydra(&norm_bho));
    }

    #[test]
    fn bms_round_trip() {
        // ψ^H_1(ψ^H_2(0)) ↔ (0,0,0)(1,1,0)
        assert_eq!(hydra_to_bms(&h("p1(p2(0))")).unwrap(), vec![vec![0, 0, 0], vec![1, 1, 0]]);
        // (0,0,0)(1,1,0)(2,1,0)(1,1,0) ↔ p1(p2(p2(0))+p2(0))
        let m = vec![vec![0, 0, 0], vec![1, 1, 0], vec![2, 1, 0], vec![1, 1, 0]];
        assert_eq!(fmt(&bms_to_hydra(&m).unwrap()), "p1(p2(p2(0))+p2(0))");
        // round trip
        let h2 = bms_to_hydra(&m).unwrap();
        assert_eq!(hydra_to_bms(&h2).unwrap(), m);
        // single column (0,0,0) = 1
        assert_eq!(fmt(&bms_to_hydra(&vec![vec![0, 0, 0]]).unwrap()), "p1(0)");
        // two (0,0,0) columns = 2
        assert_eq!(
            fmt(&bms_to_hydra(&vec![vec![0, 0, 0], vec![0, 0, 0]]).unwrap()),
            "p1(0)+p1(0)"
        );
        // invalid: 1-row
        assert!(bms_to_hydra(&vec![vec![0]]).is_err());
    }

    #[test]
    fn hprss_to_bms_conv() {
        assert_eq!(hprss_to_bms(&[1]).unwrap(), vec![vec![0, 0, 0]]);
        // (1,2) ↔ p1(p1(0)) ↔ (0,0,0)(1,0,0)
        assert_eq!(hprss_to_bms(&[1, 2]).unwrap(), vec![vec![0, 0, 0], vec![1, 0, 0]]);
        // (1,2,3) ↔ p1(p1(p1(0))) ↔ (0,0,0)(1,0,0)(2,0,0)
        assert_eq!(hprss_to_bms(&[1, 2, 3]).unwrap(), vec![vec![0, 0, 0], vec![1, 0, 0], vec![2, 0, 0]]);
    }
}
