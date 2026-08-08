//! Sudden Sequence System (SSS) expansion.
//!
//! Port of Hyp_Cos's `expandSSS` from JavaScript. Operates on a 1-D integer
//! sequence. The input is read-only; expansion produces a fresh sequence.
//!
//! Also converts an SSS sequence into a BOCF `Term`, following the main
//! algorithm from. Ordinal arithmetic (addition, successor,
//! multiplication, log, etc.) is reused from `crate::term`, which handles
//! boundary cases (e.g. `1 + ω = ω`) that the Python reference leaves
//! unresolved; only the SSS-specific conversion and normalization structure
//! is ported here.

use crate::term::{add, eq, exp, is_zero, last_term, log, one, t, zero, Term};
use crate::ocf::{Nocf, nocf_to_sss_string};

/// Lexicographic order on two integer sequences.
/// Returns 1 if `a > b`, -1 if `a < b`, 0 if equal. A shorter sequence that
/// is a prefix of the other is considered smaller.
fn lex_order(a: &[i64], b: &[i64]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] > b[i] {
            return 1;
        }
        if a[i] < b[i] {
            return -1;
        }
    }
    if a.len() > b.len() {
        1
    } else if a.len() < b.len() {
        -1
    } else {
        0
    }
}

/// Expand a Sudden Sequence System sequence by `fs` bad-part copies.
pub fn expand_sss(seq: &[i32], fs: i32) -> Vec<i32> {
    let mut s = seq.to_vec();
    let l = s.len().wrapping_sub(1);
    if s.is_empty() || s[l] <= 0 {
        s.pop();
        return s;
    }

    // Find the rightmost column strictly below the last value.
    let mut r = l as i64;
    while r >= 0 && s[r as usize] >= s[l] {
        r -= 1;
    }
    // For valid SSS sequences the first value is 0 < s[l], so r >= 0.
    if r < 0 {
        r = 0;
    }

    let bad_root = r as usize;
    let mut bad_value = s[bad_root];
    let subseq1: Vec<i64> = s[bad_root..]
        .iter()
        .map(|&x| x as i64 - s[bad_root] as i64)
        .collect();

    while r > 0 {
        r -= 1;
        let ri = r as usize;
        if s[ri] <= bad_value {
            bad_value = bad_value.min(s[ri]);
            let subseq2: Vec<i64> = s[ri..]
                .iter()
                .map(|&x| x as i64 - s[ri] as i64)
                .collect();
            if lex_order(&subseq1, &subseq2) > 0 {
                break;
            }
        }
    }

    let l_len = s.len() as i64;
    let res_len = (l_len + (l_len - 1 - r) * fs as i64).max(0) as usize;
    s[l] -= 1;
    let dif = s[l] - s[r as usize];

    let mut res = Vec::with_capacity(res_len);
    let mut j = 0usize;
    let mut count = 0i64;
    for _ in 0..res_len {
        res.push(s[j] + (dif as i64 * count) as i32);
        j += 1;
        if j >= s.len() {
            j = r as usize + 1;
            count += 1;
        }
    }
    res
}

/// Component-wise BOCF comparison (Python `compare`). Returns ±3 when the
/// subscripts differ, ±2 when the arguments differ, ±1 when the tails differ,
/// and 0 when equal.
fn compare(a: &Term, b: &Term) -> i32 {
    if eq(a, b) {
        return 0;
    }
    if is_zero(a) {
        return -3;
    }
    if is_zero(b) {
        return 3;
    }
    let na = a.as_ref().unwrap();
    let nb = b.as_ref().unwrap();
    let k = compare(&na.a, &nb.a);
    if k > 0 {
        return 3;
    }
    if k < 0 {
        return -3;
    }
    let k = compare(&na.b, &nb.b);
    if k > 0 {
        return 2;
    }
    if k < 0 {
        return -2;
    }
    let k = compare(&na.c, &nb.c);
    if k > 0 {
        1
    } else {
        -1
    }
}

/// Sum terms `l[a..b]` (Python `Sum`) using ordinal addition.
fn sum_range(l: &[Term], a: usize, b: usize) -> Term {
    let mut r = zero();
    for i in a..b {
        r = add(&r, &l[i]);
    }
    r
}

/// Truncate `x`'s tail at the threshold given by comparison level `k`
/// (Python `truncate`).
fn truncate(x: &Term, y: &Term, k: i32) -> Term {
    if is_zero(y) {
        return x.clone();
    }
    if compare(x, y) < k {
        return zero();
    }
    let nx = x.as_ref().unwrap();
    t(nx.a.clone(), nx.b.clone(), truncate(&nx.c, y, k))
}

/// Python `times` (ψ-subscript product, distinct from `term::mul_finite`).
fn times(sub: &Term, x: &Term) -> Term {
    if is_zero(x) {
        return zero();
    }
    let nx = x.as_ref().unwrap();
    t(sub.clone(), log(x), times(sub, &nx.c))
}

/// Python `succ` (successor respecting the BOCF normal form).
fn succ(x: &Term) -> Term {
    if is_zero(x) {
        return zero();
    }
    let nx = x.as_ref().unwrap();
    let x0 = nx.a.clone();
    if compare(&nx.b, &t(x0.clone(), zero(), zero())) < 0 {
        return add(&exp(&nx.b), &succ(&nx.c));
    }
    if compare(&nx.b, &t(x0.clone(), t(x0.clone(), zero(), one()), zero())) >= 0 {
        return add(&t(x0.clone(), nx.b.clone(), zero()), &succ(&nx.c));
    }
    let mut y = nx.b.clone();
    if compare(&y, &t(x0.clone(), one(), zero())) < 0 {
        y = y.as_ref().unwrap().c.clone();
    }
    add(
        &t(
            x0.clone(),
            add(&t(x0.clone(), t(x0.clone(), zero(), zero()), zero()), &y),
            zero(),
        ),
        &succ(&nx.c),
    )
}

/// Python `limit`.
fn limit(sub: &Term, la: &Term, x: &Term) -> Term {
    if is_zero(x) {
        return zero();
    }
    let nx = x.as_ref().unwrap();
    add(
        &exp(&add(&times(sub, la), &nx.b)),
        &limit(sub, la, &nx.c),
    )
}

/// Python `mul`.
fn mul(la: &Term, x: &Term) -> Term {
    let nl = la.as_ref().unwrap();
    let la0 = nl.a.clone();
    let la1 = nl.b.clone();
    let nx = x.as_ref().unwrap();
    let x1 = nx.b.clone();
    if is_zero(&x1) {
        let sub = add(&la0, &one());
        let in_psi = truncate(
            &la1,
            &t(
                sub.clone(),
                t(sub.clone(), t(sub.clone(), zero(), zero()), zero()),
                zero(),
            ),
            0,
        );
        let tail = if !is_zero(&in_psi) && eq(&la1, &in_psi) && is_zero(&nl.c) {
            nx.c.clone()
        } else {
            x.clone()
        };
        return t(la0.clone(), add(&in_psi, &limit(&sub, la, &tail)), zero());
    }
    let x1n = x1.as_ref().unwrap();
    let k = compare(&la0, &x1n.a);
    if k >= 0 {
        let sub = add(&la0, &one());
        let in_psi = truncate(
            &la1,
            &t(
                sub.clone(),
                t(sub.clone(), t(sub.clone(), zero(), zero()), zero()),
                zero(),
            ),
            0,
        );
        if k == 0 {
            let in_psi1 = truncate(&x1n.b, &exp(&times(&sub, &add(la, &one()))), 0);
            if compare(&in_psi, &in_psi1) < 0 {
                let tail = if is_zero(&x1n.c) && eq(&in_psi1, &x1n.b) {
                    nx.c.clone()
                } else {
                    x.clone()
                };
                return t(la0.clone(), add(&in_psi1, &limit(&sub, la, &tail)), zero());
            }
        }
        return t(la0.clone(), add(&in_psi, &limit(&sub, la, x)), zero());
    }
    let sub = add(&x1n.a, &one());
    let in_psi = truncate(&x1n.b, &exp(&times(&sub, la)), 0);
    let tail = if eq(&in_psi, &x1n.b) {
        nx.c.clone()
    } else {
        x.clone()
    };
    t(x1n.a.clone(), add(&in_psi, &limit(&sub, la, &tail)), zero())
}

/// Python `psi_effect`.
fn psi_effect(sub: &Term, x: &Term) -> Term {
    let nx = x.as_ref().unwrap();
    // x.a is a ψ-term; guard against the None case which never occurs for
    // valid SSS inputs.
    if nx.a.is_some() && compare(&nx.a.as_ref().unwrap().a, sub) > 0 {
        let inner = psi_effect(&nx.a.as_ref().unwrap().a, x);
        return psi_effect(sub, &inner);
    }
    let la = last_term(&nx.a);
    let rem = truncate(&nx.a, &la, 1);
    if compare(&rem, sub) > 0 {
        let inner = psi_effect(&rem, x);
        return psi_effect(sub, &inner);
    }
    if eq(&la, &one()) {
        let x0 = nx.a.clone();
        let mut y = x.clone();
        loop {
            if is_zero(&y) {
                break;
            }
            let yn = y.as_ref().unwrap();
            if compare(&yn.b, &t(x0.clone(), zero(), zero())) >= 0 {
                y = yn.c.clone();
            } else {
                break;
            }
        }
        let mut in_psi = truncate(&nx.b, &t(add(&x0, &one()), zero(), zero()), 0);
        let succ_arg = if !is_zero(&in_psi) && eq(&in_psi, &nx.b) {
            nx.c.clone()
        } else {
            x.clone()
        };
        in_psi = add(&in_psi, &succ(&succ_arg));
        let r = t(sub.clone(), in_psi.clone(), zero());
        if is_zero(&y) || compare(&y.as_ref().unwrap().b, &r) < 0 {
            return r;
        }
        let yn = y.as_ref().unwrap();
        let in_psi2 = truncate(&yn.b.as_ref().unwrap().b, &t(x0.clone(), zero(), zero()), 0);
        let succ_arg2 = if is_zero(&yn.b.as_ref().unwrap().c) && eq(&in_psi2, &yn.b.as_ref().unwrap().b) {
            yn.c.clone()
        } else {
            y.clone()
        };
        let in_psi2 = add(&in_psi2, &succ(&succ_arg2));
        return t(sub.clone(), in_psi2, zero());
    }
    let la2 = log(&la);
    let k = compare(&nx.b, &t(nx.a.clone(), zero(), zero()));
    if k > 0 {
        let mut in_psi = truncate(&nx.b, &t(add(&nx.a, &one()), zero(), zero()), 0);
        let mut x2 = x.clone();
        if eq(&in_psi, &nx.b) {
            x2 = nx.c.clone();
        }
        in_psi = add(&in_psi, &x2);
        return t(sub.clone(), in_psi, zero());
    }
    if k == 0 {
        if compare(&nx.c, &t(nx.a.clone(), one(), zero())) >= 0 {
            return t(sub.clone(), nx.c.clone(), zero());
        }
        return t(sub.clone(), t(nx.a.clone(), zero(), nx.c.clone()), zero());
    }
    if compare(&nx.b, &t(sub.clone(), zero(), zero())) == 3 {
        let inner = psi_effect(&nx.b.as_ref().unwrap().a, x);
        return psi_effect(sub, &inner);
    }
    let m = if !is_zero(&la2) && {
        let ln = la2.as_ref().unwrap();
        is_zero(&ln.b) && is_zero(&ln.c) && is_zero(&nx.b) && !is_zero(&ln.a)
    } {
        if is_zero(&nx.c) {
            return t(sub.clone(), la2.clone(), zero());
        }
        let la2a = la2.as_ref().unwrap().a.clone();
        t(
            la2a.clone(),
            limit(&add(&la2a, &one()), &la2, &nx.c),
            zero(),
        )
    } else {
        mul(&la2, x)
    };
    let m0 = m.as_ref().map(|n| n.a.clone()).unwrap_or_else(zero);
    if eq(&m0, sub) {
        m
    } else {
        t(sub.clone(), m, zero())
    }
}

/// Normalize a list of BOCF terms into a single term (Python `List_to_BOCF`).
fn list_to_bocf(l: &mut Vec<Term>) -> Term {
    if l.is_empty() {
        return zero();
    }
    loop {
        let mut t = 0usize;
        while t + 1 < l.len() && compare(&l[t], &l[t + 1]) > -3 {
            t += 1;
        }
        if t + 1 == l.len() {
            return sum_range(l, 0, l.len());
        }
        let mut t1 = t;
        while t1 + 1 < l.len() {
            let k = compare(&l[t1], &l[t1 + 1]);
            if k == 3 {
                break;
            }
            if k == -3 {
                t = t1;
            }
            t1 += 1;
        }
        if t1 > t + 1 {
            let s = sum_range(l, t + 1, t1 + 1);
            l[t + 1] = s;
            for _ in (t + 1..t1).rev() {
                l.remove(t + 2);
            }
        }
        if t + 2 == l.len() || compare(&l[t], &l[t + 2]) > -3 {
            let sub = l[t].as_ref().unwrap().a.clone();
            l.remove(t);
            l[t] = psi_effect(&sub, &l[t]);
        } else {
            let s0 = l[t + 2].as_ref().unwrap().a.clone();
            l[t + 1] = psi_effect(&s0, &l[t + 1]);
        }
    }
}

/// Convert an SSS sequence into a list of BOCF ψ-terms (Python `SSS_to_List`).
fn sss_to_terms(s: &[i32], start: usize) -> Vec<Term> {
    if start + 1 == s.len() || s[start + 1] < s[start] {
        return vec![one()];
    }
    if s[start + 1] == s[start] {
        let mut v = vec![one()];
        v.extend(sss_to_terms(s, start + 1));
        return v;
    }
    let tail = list_to_bocf(&mut sss_to_terms(s, start + 1));
    let mut l = vec![t(tail, zero(), zero())];
    let k = s[start];
    let mut t = start + 1;
    while t < s.len() && s[t] > k {
        t += 1;
    }
    if t < s.len() && s[t] == k {
        l.extend(sss_to_terms(s, t));
    }
    l
}

/// Convert an SSS sequence into its BOCF ordinal term.
pub fn sss_to_bocf(seq: &[i32]) -> Term {
    let mut s = seq.to_vec();
    if s.len() > 1 && s[1] > 0 {
        s.insert(0, 0);
    }
    list_to_bocf(&mut sss_to_terms(&s, 0))
}

// ════════════════════════════════════════════════════════════════
// NOCF (Nocf type lives in crate::ocf)
// ════════════════════════════════════════════════════════════════

fn is_num_token(t: &str) -> bool {
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
}

/// Convert an SSS sequence into its NOCF term (port of `SSS转NOCF.html`,
/// with the Chinese identifiers renamed and the unused `I`-subscript display
/// code omitted).
pub fn sss_to_nocf(seq: &[i32]) -> Result<Nocf, String> {
    // ── 1. Build the parenthesised/comma token sequence ──
    let mut black: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    for &value in seq {
        if value < 0 {
            return Err("sequence values must be non-negative".to_string());
        }
        let v = value as usize;
        let comma_count = if cursor > 0 {
            black[..cursor - 1].iter().filter(|t| **t == ",").count()
        } else {
            0
        };
        if v == comma_count {
            black.insert(cursor, "(".to_string());
            black.insert(cursor + 1, ")".to_string());
            cursor += 1;
        } else if v > comma_count {
            let diff = v - comma_count;
            black.insert(cursor, "(".to_string());
            black.insert(cursor + 1, ")".to_string());
            for _ in 0..diff {
                black.insert(cursor, ",".to_string());
            }
            cursor += diff + 1;
        } else {
            let comma_positions: Vec<usize> = black
                .iter()
                .enumerate()
                .filter(|(_, t)| *t == ",")
                .map(|(i, _)| i + 1)
                .collect();
            if v >= comma_positions.len() {
                return Err("invalid sequence".to_string());
            }
            let insert_pos = comma_positions[v];
            black.insert(insert_pos - 1, "(".to_string());
            black.insert(insert_pos, ")".to_string());
            cursor = insert_pos;
        }
    }

    // ── 2. Insert '1' before each comma not preceded by '1' or ')' ──
    let mut need_insert = true;
    while need_insert {
        need_insert = false;
        let mut insert_positions: Vec<usize> = Vec::new();
        for i in 0..black.len() {
            if black[i] == "," && (i == 0 || (black[i - 1] != "1" && black[i - 1] != ")")) {
                insert_positions.push(i);
            }
        }
        for &p in insert_positions.iter().rev() {
            black.insert(p, "1".to_string());
            need_insert = true;
        }
    }

    // ── 3. Collapse `()` → `2` and `(n)` → `n+1` ──
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < black.len() {
            if black[i] == "(" && black.get(i + 1).map_or(false, |t| t == ")") {
                black.splice(i..i + 2, ["2".to_string()]);
                changed = true;
            } else if black[i] == "("
                && black.get(i + 1).map_or(false, |t| is_num_token(t))
                && black.get(i + 2).map_or(false, |t| t == ")")
            {
                let n: u32 = black[i + 1].parse().unwrap_or(0);
                black.splice(i..i + 3, [(n + 1).to_string()]);
                changed = true;
            } else {
                i += 1;
            }
        }
    }

    // ── 4. Subtract 1 from every number, then reverse the token list ──
    let mut tokens: Vec<String> = Vec::with_capacity(black.len());
    for t in &black {
        if is_num_token(t) {
            let n: i64 = t.parse().unwrap_or(0);
            tokens.push((n - 1).to_string());
        } else {
            tokens.push(t.clone());
        }
    }
    tokens.reverse();

    // ── 5. Swap parentheses, then insert 'W' before every '(' ──
    let mut w_tokens: Vec<String> = Vec::with_capacity(tokens.len() + 8);
    for t in &tokens {
        if t == "(" {
            w_tokens.push(")".to_string());
        } else if t == ")" {
            w_tokens.push("W".to_string());
            w_tokens.push("(".to_string());
        } else {
            w_tokens.push(t.clone());
        }
    }
    let w_string: String = w_tokens.concat();

    // ── 6. Parse the W-number into a NOCF tree ──
    let inner = parse_w_expr(&w_string)?;
    // Only wrap in ψ₀ when the sequence is lexicographically ≥ [0,1];
    // smaller sequences (e.g. [0], [0,0], [0,0,0]) keep the raw NOCF.
    let ge_01 = {
        let pivot = [0, 1];
        let mut i = 0;
        loop {
            let a = seq.get(i).copied();
            let b = pivot.get(i).copied();
            match (a, b) {
                (Some(x), Some(y)) if x == y => i += 1,
                (Some(_), None) => break true,          // seq longer
                (None, Some(_)) => break false,         // seq shorter
                (None, None) => break true,             // equal → wrap
                (Some(x), Some(y)) => break x > y,
            }
        }
    };
    if ge_01 {
        Ok(Nocf::psi(Nocf::Zero, inner))
    } else {
        Ok(inner)
    }
}

/// Parse a W-number (`W(p1,...,pn)`, leaves are natural numbers) into NOCF.
/// Following `wToNocf`: `W(a)` → `p(a)` and `W(s1,...,sk,a)` → `p_{s1}(a)`
/// (only the first subscript component is kept; the unused `I`-display for
/// multi-component subscripts is omitted since it never occurs for SSS).
fn parse_w_expr(input: &str) -> Result<Nocf, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    let e = parse_w_node(&chars, &mut pos)?;
    if pos != chars.len() {
        return Err("trailing input in W-number".to_string());
    }
    Ok(e)
}

fn parse_w_node(chars: &[char], pos: &mut usize) -> Result<Nocf, String> {
    if *pos >= chars.len() {
        return Err("unexpected end of W-number".to_string());
    }
    if chars[*pos] == 'W' {
        *pos += 1;
        if chars.get(*pos) != Some(&'(') {
            return Err("expected '(' after W".to_string());
        }
        *pos += 1;
        let mut params: Vec<Nocf> = Vec::new();
        loop {
            let e = parse_w_node(chars, pos)?;
            params.push(e);
            if chars.get(*pos) == Some(&',') {
                *pos += 1;
                continue;
            }
            break;
        }
        if chars.get(*pos) != Some(&')') {
            return Err("expected ')'".to_string());
        }
        *pos += 1;
        return Ok(w_params_to_nocf(params));
    }
    // natural number
    let start = *pos;
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos == start {
        return Err("unexpected character in W-number".to_string());
    }
    let n: u32 = chars[start..*pos]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| "bad number in W-number".to_string())?;
    Ok(Nocf::from_nat(n as i32))
}

fn w_params_to_nocf(params: Vec<Nocf>) -> Nocf {
    match params.len() {
        0 => Nocf::Zero,
        1 => Nocf::psi(Nocf::Zero, params[0].clone()),
        n => {
            let arg = params[n - 1].clone();
            let sub = params[0].clone();
            Nocf::psi(sub, arg)
        }
    }
}

// ════════════════════════════════════════════════════════════════
// TPrSS
// ════════════════════════════════════════════════════════════════

/// Convert an SSS sequence into its TPrSS column (port of
/// `display_tprss2_plain` / `convertToTPRSS2` from `sss.js`). The conversion
/// operates directly on the NOCF tree produced by [`sss_to_nocf`], so the
/// string round-trips (`nocfToFormatted` → `generateNOCF5` → reparse) in the
/// JS reference are avoided.
pub fn sss_to_tprss(seq: &[i32]) -> Result<String, String> {
    let nocf = sss_to_nocf(seq)?;
    let column = nocf_to_tprss(&nocf);
    // Strip the outermost column wrapper: `(a,b,...)` → `a,b,...`.
    Ok(column[1..column.len() - 1].to_string())
}

/// Convert a NOCF tree to its TPrSS column string.
fn nocf_to_tprss(nocf: &Nocf) -> String {
    convert_node(nocf)
}

/// `convertNode` from `sss.js`, applied to the NOCF tree after `generateNOCF5`.
fn convert_node(nocf: &Nocf) -> String {
    match nocf {
        Nocf::Zero => "0".to_string(),
        Nocf::Psi(v, a) => {
            let mut elements: Vec<String> = Vec::new();
            // Subscript: emit its value as-is (natural numbers are displayed
            // as numbers, ψ terms are emitted as nested tuples).
            match v.as_ref() {
                Nocf::Zero => elements.push("0".to_string()),
                other => {
                    let n = other.to_nat();
                    if n >= 0 { elements.push(n.to_string()); }
                    else { elements.push(convert_node(other)); }
                }
            }
            // Argument: skip zero, otherwise expand/convert and flatten.
            match a.as_ref() {
                Nocf::Zero => {}
                other => {
                    for el in flatten_tprss(&arg_converted(other)) {
                        elements.push(el);
                    }
                }
            }
            format!("({})", elements.join(","))
        }
    }
}

/// Convert a NOCF argument, expanding natural numbers via the `generateNOCF5`
/// scheme (nested ψ's).
fn arg_converted(nocf: &Nocf) -> String {
    match nocf {
        Nocf::Zero => "0".to_string(),
        other => {
            let n = other.to_nat();
            if n >= 0 { convert_node(&g5(n as u32)) }
            else { convert_node(other) }
        }
    }
}

/// `generateNOCF5`'s expansion of a natural number n>0 into n nested `p(0)`s.
fn g5(k: u32) -> Nocf {
    if k == 0 {
        Nocf::Zero
    } else {
        Nocf::psi(Nocf::Zero, g5(k - 1))
    }
}

/// `flattenElements` from `sss.js`: split a tuple on its top-level commas.
fn flatten_tprss(s: &str) -> Vec<String> {
    if s == "0" {
        return vec!["0".to_string()];
    }
    if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        return split_top_commas(inner);
    }
    vec![s.to_string()]
}

fn split_top_commas(s: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                parts.push(cur.clone());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(xs: &[i32]) -> Vec<i32> {
        xs.to_vec()
    }

    #[test]
    fn successor_drops_last() {
        assert_eq!(expand_sss(&seq(&[0, 1, 0]), 1), vec![0, 1]);
        assert_eq!(expand_sss(&seq(&[0]), 1), Vec::<i32>::new());
    }

    #[test]
    fn simple_expansion() {
        // (0,1,2)[1] → (0,1,1,2,2), verified against the JS reference.
        assert_eq!(expand_sss(&seq(&[0, 1, 2]), 1), vec![0, 1, 1, 2, 2]);
        assert_eq!(expand_sss(&seq(&[0, 1, 2]), 2), vec![0, 1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn preserves_input() {
        let input = vec![0, 1, 2, 3];
        expand_sss(&input, 1);
        assert_eq!(input, vec![0, 1, 2, 3]);
    }

    #[test]
    fn bocf_reference_outputs() {
        let cases: &[(&[i32], &str)] = &[
            (&[0, 1], "\\omega"),
            (&[0, 1, 2], "\\psi\\left(\\Omega^{\\Omega}\\right)"),
            (&[0, 1, 2, 3], "\\psi\\left(\\Omega_{\\Omega}\\right)"),
            (&[0, 1, 2, 1], "\\psi\\left(\\Omega_{2}^{\\Omega}\\omega\\right)"),
            (&[0, 1, 1], "\\omega^{\\omega}"),
            (&[0, 1, 2, 3, 4], "\\psi\\left(\\Omega_{\\Omega_{\\Omega}}\\right)"),
            (&[0, 1, 2, 2], "\\psi\\left(\\Omega_{2}^{\\Omega_{2}}\\right)"),
        ];
        for (s, expected) in cases {
            let t = sss_to_bocf(s);
            let got = crate::term::term_to_string(true, &t);
            assert_eq!(&got, expected, "SSS {:?}", s);
        }
    }

    #[test]
    fn nocf_reference_outputs() {
        let cases: &[(&[i32], &str)] = &[
            (&[0, 1], "\\psi\\left(\\Omega\\right)"),
            (&[0, 0, 0], "3"),
            (&[0, 1, 2], "\\psi\\left(\\Omega_{\\Omega}\\right)"),
            (&[0, 1, 2, 3], "\\psi\\left(\\Omega_{\\Omega_{\\Omega}}\\right)"),
            (&[0, 1, 2, 1], "\\psi\\left(\\Omega_{\\Omega+1}\\right)"),
            (&[0, 1, 1], "\\psi\\left(\\Omega_{2}\\right)"),
            (&[0, 1, 2, 3, 4], "\\psi\\left(\\Omega_{\\Omega_{\\Omega_{\\Omega}}}\\right)"),
            (&[0, 1, 2, 2], "\\psi\\left(\\Omega_{\\Omega_{2}}\\right)"),
        ];
        for (s, expected) in cases {
            let got = sss_to_nocf(s).expect("no conversion error");
            let got_str = nocf_to_sss_string(&got);
            assert_eq!(&got_str, expected, "SSS {:?}", s);
        }
    }

    #[test]
    fn tprss_reference_outputs() {
        // Expected values computed from the wrapped NOCF tree (i.e. with the
        // ψ₀ wrapper that `sss_to_nocf` applies), matching the `tprss_wrapped`
        // column of the `sss.js` reference pipeline.
        let cases: &[(&[i32], &str)] = &[
            (&[0], "0"),
            (&[0, 0], "0,0"),
            (&[0, 0, 0], "0,0,0"),
            (&[0, 1], "0,1"),
            (&[0, 1, 1], "0,2"),
            (&[0, 1, 2], "0,(1)"),
            (&[0, 1, 2, 1], "0,(1,0)"),
            (&[0, 1, 2, 2], "0,(2)"),
            (&[0, 1, 2, 3], "0,((1))"),
            (&[0, 1, 2, 3, 4], "0,(((1)))"),
            (&[0, 1, 2, 1, 2], "0,(1,1)"),
            (&[0, 1, 2, 1, 1], "0,(1,0,0)"),
        ];
        for (s, expected) in cases {
            let got = sss_to_tprss(s).expect("no conversion error");
            assert_eq!(&got, expected, "SSS {:?}", s);
        }
    }
}
