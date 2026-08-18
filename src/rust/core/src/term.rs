//! Ordinal arithmetic, fundamental sequences, and Veblen rendering.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static TERM_ID: AtomicUsize = AtomicUsize::new(1);

/// ψ_a(b) + c. `None` represents zero.
pub type Term = Option<Rc<TermNode>>;

pub struct TermNode {
    /// Stable identity for render caching. Unlike the Rc address, this is
    /// never reused by a different term, so cache entries cannot be
    /// corrupted by allocator address reuse.
    pub id: usize,
    pub a: Term,
    pub b: Term,
    pub c: Term,
}

pub fn zero() -> Term {
    None
}

pub fn is_zero(t: &Term) -> bool {
    t.is_none()
}

pub fn t(a: Term, b: Term, c: Term) -> Term {
    Some(Rc::new(TermNode {
        id: TERM_ID.fetch_add(1, Ordering::Relaxed),
        a,
        b,
        c,
    }))
}

/// Constants. Terms are immutable and eq/lt are structural, so constructing
/// fresh on each call is semantically identical to C++ static locals.
pub fn one() -> Term {
    t(zero(), zero(), zero())
}

pub fn omega() -> Term {
    t(zero(), one(), zero())
}

pub fn omega1() -> Term {
    t(one(), zero(), zero())
}

pub fn epsilon0() -> Term {
    t(zero(), omega1(), zero())
}

pub fn bho() -> Term {
    t(zero(), t(succ(&one()), zero(), zero()), zero())
}

// ============================================================
// Structural helpers
// ============================================================

pub fn is_ordinal_finite(a: &Term) -> bool {
    is_zero(a)
        || match a {
            Some(n) => is_zero(&n.a) && is_zero(&n.b),
            None => false,
        }
}

pub fn length1(a: &Term) -> i32 {
    match a {
        None => 0,
        Some(n) => 1 + length1(&n.c),
    }
}

pub fn eq(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (None, None) => true,
        (None, Some(_)) | (Some(_), None) => false,
        (Some(x), Some(y)) => eq(&x.a, &y.a) && eq(&x.b, &y.b) && eq(&x.c, &y.c),
    }
}

pub fn lt(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(x), Some(y)) => {
            if !eq(&x.a, &y.a) {
                lt(&x.a, &y.a)
            } else if !eq(&x.b, &y.b) {
                lt(&x.b, &y.b)
            } else {
                lt(&x.c, &y.c)
            }
        }
    }
}

pub fn gt(a: &Term, b: &Term) -> bool {
    !lt(a, b) && !eq(a, b)
}

pub fn le(a: &Term, b: &Term) -> bool {
    lt(a, b) || eq(a, b)
}

pub fn first_term(a: &Term) -> Term {
    match a {
        None => zero(),
        Some(n) => t(n.a.clone(), n.b.clone(), zero()),
    }
}

pub fn last_term(a: &Term) -> Term {
    match a {
        None => zero(),
        Some(n) => {
            if is_zero(&n.c) {
                a.clone()
            } else {
                last_term(&n.c)
            }
        }
    }
}

pub fn every_terms(a: &Term) -> Vec<Term> {
    let mut terms = Vec::new();
    let mut cur = a.clone();
    while !is_zero(&cur) {
        let node = cur.as_ref().unwrap();
        terms.push(first_term(&cur));
        cur = node.c.clone();
    }
    terms
}

// ============================================================
// Arithmetic
// ============================================================

fn from_int(n: i32) -> Term {
    let mut r = zero();
    for _ in 0..n {
        r = add(&r, &one());
    }
    r
}

pub fn is_succ(a: &Term) -> bool {
    match a {
        None => false,
        Some(_) => {
            let last = last_term(a);
            let node = last.as_ref().unwrap();
            is_zero(&node.a) && is_zero(&node.b)
        }
    }
}

pub fn pred(a: &Term) -> Term {
    if !is_succ(a) {
        return a.clone();
    }
    let mut r = zero();
    let mut cur = a.clone();
    while !is_zero(&cur) {
        let node = cur.as_ref().unwrap();
        if is_zero(&node.c) {
            break; // last term — skip
        }
        r = add(&r, &t(node.a.clone(), node.b.clone(), zero()));
        cur = node.c.clone();
    }
    r
}

pub fn add(a: &Term, b: &Term) -> Term {
    if is_zero(a) {
        return b.clone();
    }
    if is_zero(b) {
        return a.clone();
    }
    if lt(&first_term(a), &first_term(b)) {
        return b.clone();
    }
    let node = a.as_ref().unwrap();
    t(node.a.clone(), node.b.clone(), add(&node.c, b))
}

pub fn succ(a: &Term) -> Term {
    add(a, &one())
}

pub fn sub(a: &Term, b: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    if is_zero(b) {
        return a.clone();
    }
    if gt(&first_term(a), &first_term(b)) {
        return a.clone();
    }
    let na = a.as_ref().unwrap();
    let nb = b.as_ref().unwrap();
    sub(&na.c, &nb.c)
}

pub fn separate(a: &Term, b: &Term) -> (Term, Term) {
    if is_zero(a) {
        return (zero(), zero());
    }
    if lt(&first_term(a), b) {
        return (zero(), a.clone());
    }
    let node = a.as_ref().unwrap();
    let (s0, s1) = separate(&node.c, b);
    (t(node.a.clone(), node.b.clone(), s0), s1)
}

pub fn truncate(a: &Term, b: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let node = a.as_ref().unwrap();
    let tc = truncate(&node.c, b);
    if is_zero(&tc) && lt(&first_term(a), &t(b.clone(), zero(), zero())) {
        return zero();
    }
    t(node.a.clone(), node.b.clone(), tc)
}

pub fn exp(a: &Term) -> Term {
    if lt(a, &epsilon0()) {
        return t(zero(), a.clone(), zero());
    }
    let node = a.as_ref().unwrap();
    let (p, _rest) = separate(&node.b, &t(succ(&node.a), zero(), zero()));
    t(node.a.clone(), add(&p, &sub(a, &t(node.a.clone(), p.clone(), zero()))), zero())
}

pub fn log(a: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let node = a.as_ref().unwrap();
    let (p, q) = separate(&node.b, &t(succ(&node.a), zero(), zero()));
    if is_zero(&node.a) && is_zero(&p) {
        if !lt(&node.b, &epsilon0()) {
            if eq(&log(&q), &q) && q.as_ref().unwrap().c.is_none() && lt(&node.b, &omega1()) {
                return first_term(a);
            }
        }
        return q;
    }
    let m = t(node.a.clone(), p.clone(), q.clone()); // m = ψ_a(p) + q
    let big = t(node.a.clone(), t(succ(&node.a), zero(), zero()), zero());
    if !lt(&node.b, &big) {
        if eq(&log(&node.b), &node.b) && is_zero(&node.c) && lt(&node.b, &t(succ(&node.a), zero(), zero())) {
            return first_term(a);
        }
    }
    m
}

// ============================================================
// Subscript depth
// ============================================================

pub fn subscript_depth(t: &Term) -> i32 {
    match t {
        None => 0,
        Some(n) => {
            let da = subscript_depth(&n.a) + 1;
            let db = subscript_depth(&n.b);
            let dc = subscript_depth(&n.c);
            da.max(db).max(dc)
        }
    }
}

// ============================================================
// Multiplication
// ============================================================

/// Multiply a by a finite ordinal b via iterated addition.
pub fn mul_finite(a: &Term, b: &Term) -> Term {
    if is_zero(b) {
        return zero();
    }
    let nb = b.as_ref().unwrap();
    add(a, &mul_finite(a, &nb.c))
}

/// General ordinal multiplication.
pub fn mul(a: &Term, b: &Term) -> Term {
    if is_zero(a) || is_zero(b) {
        return zero();
    }
    if lt(&first_term(&log(a)), &first_term(&log(b))) {
        return b.clone();
    }
    let (c, d) = separate(b, &omega());
    let log_a = log(a);
    let terms = every_terms(&c);

    let mut result = mul_finite(a, &d);
    for it in terms.iter().rev() {
        let term = exp(&add(&log_a, &log(it)));
        let node = term.as_ref().unwrap();
        result = t(node.a.clone(), node.b.clone(), result);
    }
    result
}

// ============================================================
// Standard form
// ============================================================

pub fn merge_psi_addends(a: &Term, b: &Term, c: &Term) -> Term {
    if is_zero(c) {
        return t(a.clone(), b.clone(), zero());
    }
    // c = ψ_d(t+h) + f, where all terms of t are >= ψ_{d+1}(0)
    let cn = c.as_ref().unwrap();
    let a0 = t(a.clone(), zero(), zero());
    if lt(b, &cn.b) && gt(c, &a0) {
        let tn = truncate(&cn.b, &succ(&cn.a));
        let fc = first_term(c);
        let _fcn = fc.as_ref().unwrap();
        let sub_part = sub(&fc, &t(cn.a.clone(), tn.clone(), zero()));
        return merge_psi_addends(a, &add(&tn, &sub_part), &cn.c);
    }
    merge_psi_addends(a, &add(b, &first_term(c)), &cn.c)
}

pub fn standard_form(a: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let node = a.as_ref().unwrap();
    add(
        &merge_psi_addends(&standard_form(&node.a), &zero(), &standard_form(&node.b)),
        &standard_form(&node.c),
    )
}

/// Like `merge_psi_addends`, but a ψ node is only absorbed into the argument
/// of a node of the SAME subscript. Cross-subscript merges (which jump a
/// level, e.g. ψ_0(ψ_1(ψ_2(0))) → ψ_0(ψ_2(0))) are never performed; those
/// summands are instead handled by plain ordinal absorption.
fn merge_psi_addends_no_jump(a: &Term, b: &Term, c: &Term) -> Term {
    if is_zero(c) {
        return t(a.clone(), b.clone(), zero());
    }
    let cn = c.as_ref().unwrap();
    let a0 = t(a.clone(), zero(), zero());
    if eq(&cn.a, a) && lt(b, &cn.b) && gt(c, &a0) {
        let tn = truncate(&cn.b, &succ(&cn.a));
        let fc = first_term(c);
        let sub_part = sub(&fc, &t(cn.a.clone(), tn.clone(), zero()));
        return merge_psi_addends_no_jump(a, &add(&tn, &sub_part), &cn.c);
    }
    merge_psi_addends_no_jump(a, &add(b, &first_term(c)), &cn.c)
}

/// Standard form for PSS-hydra-expressible terms that performs recursive
/// standardization and ordinal-sum absorption, but no ψ-subscript "jumping"
/// merges: only ψ_n(ψ_n(x)) → ψ_n(x) is applied, never ψ_n(ψ_m(x)) → ψ_n(x)
/// for m ≠ n.
pub fn standard_form_no_jump(a: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let node = a.as_ref().unwrap();
    add(
        &merge_psi_addends_no_jump(
            &standard_form_no_jump(&node.a),
            &zero(),
            &standard_form_no_jump(&node.b),
        ),
        &standard_form_no_jump(&node.c),
    )
}

/// Check whether `c` is in the closure set C(a, b) under the BOCF definition.
/// Returns true iff c ∈ C(a, b).
fn c_in_closure(c: &Term, a: &Term, b: &Term) -> bool {
    let psi_a_0 = t(a.clone(), zero(), zero());
    if lt(c, &psi_a_0) {
        return true;
    }
    // c >= ψ_a(0)
    match c {
        None => false,
        Some(n) => {
            if !is_zero(&n.c) {
                // c = c0 + c1
                let c0 = first_term(c);
                let c1 = n.c.clone();
                c_in_closure(&c0, a, b) && c_in_closure(&c1, a, b)
            } else {
                // c = ψ_c0(c1)
                c_in_closure(&n.a, a, b) && c_in_closure(&n.b, a, b) && lt(&n.b, b)
            }
        }
    }
}

/// Check whether a BOCF term is in standard (normal) form.
/// Returns true iff a ∈ T.
pub fn is_bocf_standard(a: &Term) -> bool {
    match a {
        None => true,
        Some(n) => {
            if !is_zero(&n.c) {
                // a = a0 + a1
                let a0 = first_term(a);
                let a1 = n.c.clone();
                if !is_bocf_standard(&a0) || !is_bocf_standard(&a1) {
                    return false;
                }
                match &a1 {
                    None => false,
                    Some(n1) => {
                        if is_zero(&n1.c) {
                            // a1 is a single term: check a0 >= a1
                            !lt(&a0, &a1)
                        } else {
                            // a1 = a2 + a3: check a0 >= a2
                            let a2 = first_term(&a1);
                            !lt(&a0, &a2)
                        }
                    }
                }
            } else {
                // a = ψ_a0(a1)
                is_bocf_standard(&n.a) && is_bocf_standard(&n.b) && c_in_closure(&n.b, &n.a, &n.b)
            }
        }
    }
}

/// Result of a BOCF standardness check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BocfStandardness {
    /// Term is standard.
    Standard = 0,
    /// Term is not standard but standard_form can normalize it.
    NonStandardButNormalizable = 1,
    /// Term is not standard and even standard_form leaves it non-standard.
    NonStandard = 2,
}

/// Check BOCF standardness with intermediate warning level.
/// Returns `BocfStandardness` indicating how bad the non-standardness is.
///
/// A term that only fails in its sum structure (ordering/absorption) counts
/// as normalizable: standard_form fixes it without touching any ψ argument.
/// If standard_form would have to modify a ψ argument or subscript (e.g.
/// ψ₀(ψ₀(Ω)) → ψ₀(Ω)), the term is treated as hard-to-normalize and the
/// check reports `NonStandard`.
pub fn check_bocf_standardness(a: &Term) -> BocfStandardness {
    if is_bocf_standard(a) {
        return BocfStandardness::Standard;
    }
    let sf = standard_form(a);
    if is_bocf_standard(&sf) && sum_only_fix(a, &sf) {
        BocfStandardness::NonStandardButNormalizable
    } else {
        BocfStandardness::NonStandard
    }
}

/// True iff `sf` is obtained from `raw` by only absorbing/reordering
/// top-level summands: every top-level ψ of `sf` must already appear as a
/// top-level ψ of `raw`.
fn sum_only_fix(raw: &Term, sf: &Term) -> bool {
    let raw_sums = every_terms(raw);
    let sf_sums = every_terms(sf);
    sf_sums
        .iter()
        .all(|s| raw_sums.iter().any(|r| eq(s, r)))
}

pub fn is_finite_nat(t: &Term) -> bool {
    match t {
        None => true,
        Some(n) => {
            if !is_zero(&n.a) || !is_zero(&n.b) {
                return false;
            }
            is_finite_nat(&n.c)
        }
    }
}

// ============================================================
// Fundamental sequences for Extended Buchholz's ψ
// ============================================================

/// Cofinality (returns 0, 1, ω (=ψ₀(1)), or a regular cardinal Ωα).
pub fn cofinality(a: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let last = last_term(a);
    if !eq(&last, a) {
        return cofinality(&last);
    }
    let node = last.as_ref().unwrap();
    let beta = node.a.clone();
    let gamma = node.b.clone();
    if is_zero(&gamma) {
        if is_zero(&beta) {
            return one(); // ψ₀(0) → 1
        }
        let cf_beta = cofinality(&beta);
        if eq(&cf_beta, &one()) {
            return last; // succ subscript → regular
        }
        return cf_beta; // limit subscript → cof(subscript)
    }
    let cf_gamma = cofinality(&gamma);
    if eq(&cf_gamma, &one()) {
        return omega(); // succ argument → ω
    }
    if !lt(&beta, &cf_gamma) {
        return cf_gamma; // β ≥ Cof(γ) → Cof(γ) ≤ β
    }
    omega() // β < Cof(γ) → ω
}

fn sum_without_last(a: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let mut r = zero();
    let mut cur = a.clone();
    loop {
        if is_zero(&cur) {
            break;
        }
        let node = cur.as_ref().unwrap();
        if is_zero(&node.c) {
            break;
        }
        r = add(&r, &t(node.a.clone(), node.b.clone(), zero()));
        cur = node.c.clone();
    }
    r
}

/// Cofinality following BOCF_EBO's convention: `None` for the
/// successor/zero class ("undefined" in the reference), `Some(cf)`
/// otherwise. Note this differs from literal cofinality: ψ_v(0) with
/// successor v yields v, and ψ with a successor argument yields 0.
fn cof_opt(a: &Term) -> Option<Term> {
    if is_zero(a) {
        return None;
    }
    let last = last_term(a);
    let node = last.as_ref().unwrap();
    let v = &node.a;
    let arg = &node.b;
    if is_zero(arg) {
        if is_zero(v) {
            return None;
        }
        return match cof_opt(v) {
            None => Some(v.clone()),
            Some(cf) => Some(cf),
        };
    }
    match cof_opt(arg) {
        None => Some(zero()),
        Some(cf) => {
            if le(&cf, v) {
                Some(cf)
            } else {
                Some(zero())
            }
        }
    }
}

/// Fundamental sequence α[n] for integer index n.
pub fn fundamental_sequence(a: &Term, n: i32) -> Term {
    if is_zero(a) {
        return zero();
    }
    let last = last_term(a);
    if !eq(&last, a) {
        return add(&sum_without_last(a), &fundamental_sequence(&last, n));
    }
    let node = last.as_ref().unwrap();
    let beta = node.a.clone();
    let gamma = node.b.clone();

    if is_zero(&gamma) {
        if is_zero(&beta) {
            return zero(); // ψ₀(0)[n] = 0
        }
        if cof_opt(&beta).is_none() {
            return from_int(n); // successor-class subscript → n
        }
        return t(fundamental_sequence(&beta, n), zero(), zero());
    }

    match cof_opt(&gamma) {
        None => {
            // successor argument: ψβ(γ[0]) repeated n times
            let gamma0 = fundamental_sequence(&gamma, 0);
            mul_finite(&t(beta, gamma0, zero()), &from_int(n))
        }
        Some(cf) => {
            if le(&cf, &beta) {
                return t(beta, fundamental_sequence(&gamma, n), zero());
            }
            // β < Cof(γ): iterate with ψ_{cf[0]}(result) indices
            let cf_pred = fundamental_sequence(&cf, 0);
            let mut result = zero();
            for _ in 0..n {
                result = fundamental_sequence_indexed(
                    &gamma,
                    &t(cf_pred.clone(), result.clone(), zero()),
                );
            }
            t(beta, result, zero())
        }
    }
}

/// Fundamental sequence α[β] for ordinal index β.
pub fn fundamental_sequence_indexed(a: &Term, index: &Term) -> Term {
    if is_zero(a) {
        return zero();
    }
    let last = last_term(a);
    if !eq(&last, a) {
        return add(&sum_without_last(a), &fundamental_sequence_indexed(&last, index));
    }
    if is_finite_nat(index) {
        return fundamental_sequence(a, length1(index));
    }
    let node = last.as_ref().unwrap();
    let beta = node.a.clone();
    let gamma = node.b.clone();

    if is_zero(&gamma) {
        if is_zero(&beta) {
            return zero();
        }
        if cof_opt(&beta).is_none() {
            return index.clone();
        }
        return t(fundamental_sequence_indexed(&beta, index), zero(), zero());
    }

    match cof_opt(&gamma) {
        None => fundamental_sequence(a, length1(index)),
        Some(cf) => {
            if le(&cf, &beta) {
                return t(beta, fundamental_sequence_indexed(&gamma, index), zero());
            }
            fundamental_sequence(a, length1(index))
        }
    }
}

// ============================================================
// String conversion (LaTeX)
// ============================================================

fn omega_str(a: &Term) -> String {
    if is_zero(a) {
        return "\\omega".to_string();
    }
    if eq(a, &one()) {
        return "\\Omega".to_string();
    }
    format!("\\Omega_{{{}}}", render_term(a))
}

/// Decompose ψ_a(b) into Ω_a^{first} * second.
pub fn decompose_power(a: &Term) -> (Term, Term) {
    if is_zero(a) {
        return (zero(), zero());
    }
    let node = a.as_ref().unwrap();
    if is_zero(&node.a) {
        return (log(a), zero());
    }
    let (p, s) = separate(&node.b, &t(succ(&node.a), zero(), zero()));
    let (q, r) = separate(&s, &t(node.a.clone(), zero(), zero()));
    let second = exp(&r);
    let mut first = add(&one(), &p);
    let mut ptr = q;
    while !is_zero(&ptr) {
        let pn = ptr.as_ref().unwrap();
        let log_val = log(&ptr);
        let sub_val = sub(&log_val, &t(node.a.clone(), zero(), zero()));
        let exp_val = exp(&sub_val);
        first = add(&first, &exp_val);
        ptr = pn.c.clone();
    }
    (first, second)
}
// ── renderTerm cache (per-call, keyed by pointer identity; terms are immutable) ──

pub fn render_term(q: &Term) -> String {
    let mut cache = HashMap::new();
    render_term_cached(q, &mut cache)
}

fn render_term_cached(q: &Term, cache: &mut HashMap<usize, String>) -> String {
    if is_zero(q) {
        return "0".to_string();
    }
    if is_ordinal_finite(q) {
        return length1(q).to_string();
    }
    let key = q.as_ref().map(|n| n.id).unwrap_or(0);
    if let Some(s) = cache.get(&key) {
        return s.clone();
    }

    let (a_part, b_part) = separate(q, &first_term(q));
    let a0 = a_part.as_ref().unwrap().a.clone();
    let a1 = a_part.as_ref().unwrap().b.clone();

    let mut m = format!(
        "\\psi_{{{}}}\\left({}\\right)",
        render_term_cached(&a0, cache),
        render_term_cached(&a1, cache)
    );

    if is_zero(&a1) {
        m = omega_str(&a0);
    }
    if is_zero(&a0) {
        m = format!("\\psi\\left({}\\right)", render_term_cached(&a1, cache));
    }
    if is_zero(&a0) && eq(&a1, &one()) {
        m = "\\omega".to_string();
    } else if lt(&a1, &t(succ(&a0), zero(), zero())) {
        // && !eq(a1, T(succ(a0), ZERO(), ZERO())) — lt is strict, so this holds
        let (first, second) = decompose_power(&a_part);
        if !eq(&first, &a_part) {
            m = omega_str(&a0);
            if gt(&first, &one()) {
                m += &format!("^{{{}}}", render_term_cached(&first, cache));
            }
            if gt(&second, &one()) {
                m += &render_term_cached(&second, cache);
            }
            let len = length1(&a_part);
            if len > 1 {
                m += &len.to_string();
            }
            if !is_zero(&b_part) {
                m += &format!("+{}", render_term_cached(&b_part, cache));
            }
            cache.insert(key, m.clone());
            return m;
        }
    }

    let len = length1(&a_part);
    if len > 1 {
        m += &len.to_string();
    }
    if !is_zero(&b_part) {
        m += &format!("+{}", render_term_cached(&b_part, cache));
    }
    cache.insert(key, m.clone());
    m
}

// ============================================================
// Extended Veblen conversion (arXiv-2310.12832v2)
// ============================================================

fn is_below_bho(q: &Term) -> bool {
    lt(q, &bho())
}

// ----------------------------------------------------------
// k(α, β) from the paper
// ----------------------------------------------------------

fn k_function(alpha: &Term, beta: &Term) -> i32 {
    if lt(alpha, &omega1()) {
        if lt(alpha, beta) {
            return -1;
        }
        if eq(alpha, beta) {
            return 0;
        }
        return 1;
    }

    // α ≥ Ω: decompose α = ξ + Ω^γ·δ
    let mut terms: Vec<(Term, Term)> = Vec::new(); // (exponent, coeff)
    let mut tail = zero();
    {
        let mut curr = alpha.clone();
        while !is_zero(&curr) {
            let head = first_term(&curr);
            let hn = head.as_ref().unwrap();
            if !is_zero(&hn.a) {
                let (exp_t, second) = decompose_power(&head);
                terms.push((exp_t, second));
            } else {
                tail = add(&tail, &head);
            }
            curr = curr.as_ref().unwrap().c.clone();
        }
    }

    let (gamma, delta, _xi): (Term, Term, Term) = if !is_zero(&tail) {
        let mut xi = zero();
        let mut curr = alpha.clone();
        while !is_zero(&curr) {
            let head = first_term(&curr);
            let hn = head.as_ref().unwrap();
            if !is_zero(&hn.a) {
                xi = add(&xi, &head);
            }
            curr = curr.as_ref().unwrap().c.clone();
        }
        (zero(), tail.clone(), xi)
    } else if !terms.is_empty() {
        let mut xi = zero();
        for i in 0..terms.len() - 1 {
            let term = t(one(), terms[i].0.clone(), zero());
            xi = add(&xi, &term);
        }
        (terms.last().unwrap().0.clone(), terms.last().unwrap().1.clone(), xi)
    } else {
        if eq(alpha, beta) {
            return 0;
        }
        return if lt(alpha, beta) { -1 } else { 1 };
    };

    // Check: for all ρ ∈ s(α), k(ρ, β) = -1?
    let mut all_minus_one = true;
    for (exp_t, coeff) in &terms {
        if k_function(exp_t, beta) != -1 {
            all_minus_one = false;
            break;
        }
        if k_function(coeff, beta) != -1 {
            all_minus_one = false;
            break;
        }
    }
    if !is_zero(&tail) && all_minus_one {
        if k_function(&tail, beta) != -1 {
            all_minus_one = false;
        }
    }
    if all_minus_one {
        return -1;
    }

    // Check k = 0 condition: for all ρ ∈ s(ξ), k(ρ, β) = -1
    let mut xi_all_minus_one = true;
    let xi_end = if is_zero(&tail) {
        if terms.is_empty() {
            0
        } else {
            terms.len() - 1
        }
    } else {
        terms.len()
    };
    for i in 0..xi_end {
        let (exp_t, coeff) = &terms[i];
        if k_function(exp_t, beta) != -1 {
            xi_all_minus_one = false;
            break;
        }
        if k_function(coeff, beta) != -1 {
            xi_all_minus_one = false;
            break;
        }
    }
    if xi_all_minus_one {
        if eq(&gamma, beta) && eq(&delta, &one()) {
            return 0;
        }
        if k_function(&gamma, beta) == -1 && eq(&delta, beta) {
            return 0;
        }
    }

    1
}

/// λ = ψ₀(ξ) − 1.
fn psi0_minus_one(xi: &Term) -> Term {
    if is_zero(xi) {
        return zero();
    }
    t(zero(), xi.clone(), zero())
}

struct OmegaCnfTerm {
    exponent: Term,
    second: Term,
    count: i32,
}

fn decompose_omega_cnf(alpha: &Term, terms: &mut Vec<OmegaCnfTerm>, tail: &mut Term) {
    let mut curr = alpha.clone();
    while !is_zero(&curr) {
        let head = first_term(&curr);
        let hn = head.as_ref().unwrap();
        if !is_zero(&hn.a) {
            let (exp_t, second) = decompose_power(&head);
            if let Some(last) = terms.last_mut() {
                if eq(&last.exponent, &exp_t) {
                    // Same exponent: merge by accumulating total coefficient
                    let mut total = zero();
                    for _ in 0..last.count {
                        total = add(&total, &last.second);
                    }
                    total = add(&total, &second);
                    last.second = total;
                    last.count = 1;
                } else {
                    terms.push(OmegaCnfTerm {
                        exponent: exp_t,
                        second,
                        count: 1,
                    });
                }
            } else {
                terms.push(OmegaCnfTerm {
                    exponent: exp_t,
                    second,
                    count: 1,
                });
            }
        } else {
            *tail = add(tail, &head);
        }
        curr = curr.as_ref().unwrap().c.clone();
    }
}

/// t(α) — the t-function from the paper.
pub fn compute_t(alpha: &Term) -> Term {
    if is_zero(alpha) {
        return zero();
    }

    let mut terms: Vec<OmegaCnfTerm> = Vec::new();
    let mut tail = zero();
    decompose_omega_cnf(alpha, &mut terms, &mut tail);

    let (beta, gamma, xi): (Term, Term, Term) = if !is_zero(&tail) {
        let mut xi = zero();
        {
            let mut cur = alpha.clone();
            while !is_zero(&cur) {
                let head = first_term(&cur);
                let hn = head.as_ref().unwrap();
                if !is_zero(&hn.a) {
                    xi = add(&xi, &head);
                }
                cur = cur.as_ref().unwrap().c.clone();
            }
        }
        (zero(), tail.clone(), xi)
    } else if !terms.is_empty() {
        let beta = terms.last().unwrap().exponent.clone();
        let mut gamma = zero();
        {
            let mut start_idx = terms.len() as i32 - 1;
            while start_idx >= 0 && eq(&terms[start_idx as usize].exponent, &beta) {
                start_idx -= 1;
            }
            start_idx += 1;
            for i in start_idx as usize..terms.len() {
                for _ in 0..terms[i].count {
                    gamma = add(&gamma, &terms[i].second);
                }
            }
        }
        let mut xi = zero();
        {
            let mut cur = alpha.clone();
            while !is_zero(&cur) {
                let head = first_term(&cur);
                let hn = head.as_ref().unwrap();
                if !is_zero(&hn.a) {
                    let (exp_t, _second) = decompose_power(&head);
                    if gt(&exp_t, &beta) {
                        xi = add(&xi, &head);
                    }
                }
                cur = cur.as_ref().unwrap().c.clone();
            }
        }
        (beta, gamma, xi)
    } else {
        return alpha.clone(); // α < Ω: t(α) = α
    };

    let lambda = psi0_minus_one(&xi);
    let u = k_function(&beta, &lambda);
    let rho = if u == -1 {
        lambda.clone()
    } else if u == 0 {
        one()
    } else {
        zero()
    };

    let delta;
    {
        let sum = add(&rho, &gamma);
        if is_zero(&sum) {
            delta = zero();
        } else if is_ordinal_finite(&sum) {
            let n = length1(&sum);
            if n <= 1 {
                delta = zero();
            } else {
                delta = from_int(n - 1);
            }
        } else {
            delta = sum;
        }
    }

    let omega_beta = mul(&omega1(), &beta);
    add(&omega_beta, &delta)
}

// ----------------------------------------------------------
// Render V(α) as a Veblen φ expression
// ----------------------------------------------------------

fn has_omega_power_deep(t: &Term) -> bool {
    match t {
        None => false,
        Some(n) => !is_zero(&n.a) || has_omega_power_deep(&n.b) || has_omega_power_deep(&n.c),
    }
}

fn render_array_body(alpha: &Term, sugar: bool, is_position: bool) -> String {
    let full = render_array(alpha, true, sugar, is_position);
    if is_zero(alpha) {
        return "0".to_string();
    }
    if full.len() > 8 && full.starts_with("\\varphi(") && full.ends_with(')') {
        return full[8..full.len() - 1].to_string();
    }
    if full.len() > 9 && full.starts_with("\\omega^{") && full.ends_with('}') {
        return full[8..full.len() - 1].to_string();
    }
    full
}

fn render_position(beta: &Term, v_mode: bool, sugar: bool) -> String {
    if is_zero(beta) {
        return "0".to_string();
    }
    if is_finite_nat(beta) {
        return length1(beta).to_string();
    }
    if has_omega_power_deep(beta) {
        let body = if v_mode {
            render_array_body(beta, sugar, true)
        } else {
            render_array(beta, false, sugar, true)
        };
        if body.contains(',') || body.contains('@') {
            if body.len() > 8 && body.starts_with("\\varphi(") && body.ends_with(')') {
                return body;
            }
            return format!("({})", body);
        }
        return body;
    }
    render_term(beta)
}

fn render_position_matrix(beta: &Term, sugar: bool) -> String {
    if is_zero(beta) {
        return "0".to_string();
    }
    if is_finite_nat(beta) {
        return length1(beta).to_string();
    }
    if has_omega_power_deep(beta) {
        let m = render_array_matrix(beta, sugar, true);
        // Ω-power positions (ψ_a with a ≥ 1) are raw array tuples: drop the
        // \varphi prefix. ψ_0-values (e.g. φ(1,0,0,0)) keep it.
        let is_omega_power = beta.as_ref().map(|n| !is_zero(&n.a)).unwrap_or(false);
        let m = if is_omega_power {
            m.strip_prefix("\\varphi").unwrap_or(&m).to_string()
        } else {
            m
        };
        return m;
    }
    render_term(beta)
}

fn render_veblen_coeff(c: &Term, v_mode: bool, sugar: bool) -> String {
    if is_zero(c) {
        return "0".to_string();
    }
    if is_finite_nat(c) {
        return length1(c).to_string();
    }
    if has_omega_power_deep(c) {
        let v = render_veblen_rec(c, v_mode, sugar);
        if !v.is_empty() {
            return v;
        }
    }
    render_term(c)
}

fn render_veblen_coeff_matrix(c: &Term, sugar: bool) -> String {
    if is_zero(c) {
        return "0".to_string();
    }
    if is_finite_nat(c) {
        return length1(c).to_string();
    }
    if has_omega_power_deep(c) {
        let v = render_veblen_rec_matrix(c, sugar);
        if !v.is_empty() {
            return v;
        }
    }
    render_term(c)
}

fn render_array(alpha: &Term, v_mode: bool, sugar: bool, is_position: bool) -> String {
    if is_zero(alpha) {
        return "1".to_string();
    }

    let mut terms: Vec<OmegaCnfTerm> = Vec::new();
    let mut tail = zero();
    decompose_omega_cnf(alpha, &mut terms, &mut tail);

    let mut has_complex_position = false;
    for tm in &terms {
        if !is_finite_nat(&tm.exponent) {
            has_complex_position = true;
            break;
        }
    }

    if !has_complex_position && terms.is_empty() {
        if is_zero(&tail) {
            return "1".to_string();
        }
        let tn = tail.as_ref().unwrap();
        if is_zero(&tn.c) && is_zero(&tn.a) && has_omega_power_deep(&tail) {
            let coeff_str = render_veblen_coeff(&tail, v_mode, sugar);
            if coeff_str.len() >= 8 && coeff_str.starts_with("\\omega^{") {
                if is_position && !sugar {
                    return format!("\\varphi(\\omega^{{{}}})", coeff_str);
                }
                return format!("\\omega^{{{}}}", coeff_str);
            }
            if is_position {
                return format!("\\varphi({})", coeff_str);
            }
            return coeff_str;
        }
        {
            let coeff_str = render_veblen_coeff(&tail, v_mode, sugar);
            if coeff_str == "1" {
                return "\\omega".to_string();
            }
            return format!("\\omega^{{{}}}", coeff_str);
        }
    }

    if !has_complex_position {
        let mut max_exp = 0;
        for tm in &terms {
            let e = length1(&tm.exponent);
            if e > max_exp {
                max_exp = e;
            }
        }
        let mut coeff_terms: Vec<Term> = vec![zero(); (max_exp + 1) as usize];
        for tm in &terms {
            let pos = length1(&tm.exponent) as usize;
            let mut contrib = zero();
            if eq(&tm.second, &one()) || is_zero(&tm.second) {
                for _ in 0..tm.count {
                    contrib = add(&contrib, &one());
                }
            } else {
                for _ in 0..tm.count {
                    contrib = add(&contrib, &tm.second);
                }
            }
            coeff_terms[pos] = add(&coeff_terms[pos], &contrib);
        }

        if sugar && !is_position {
            let tail_sugar = if is_zero(&tail) {
                "0".to_string()
            } else {
                render_veblen_coeff(&tail, v_mode, sugar)
            };
            if max_exp == 1 && eq(&coeff_terms[1], &one()) {
                return format!("\\varepsilon_{{{}}}", tail_sugar);
            }
            if max_exp == 1 && eq(&coeff_terms[1], &add(&one(), &one())) {
                return format!("\\zeta_{{{}}}", tail_sugar);
            }
            if max_exp == 1 && eq(&coeff_terms[1], &add(&add(&one(), &one()), &one())) {
                return format!("\\eta_{{{}}}", tail_sugar);
            }
            if max_exp == 2 && eq(&coeff_terms[2], &one()) && is_zero(&coeff_terms[1]) {
                return format!("\\Gamma_{{{}}}", tail_sugar);
            }
        }

        let mut result = "\\varphi(".to_string();
        let mut first = true;
        for i in (1..=max_exp).rev() {
            if !first {
                result += ",";
            }
            result += &render_veblen_coeff(&coeff_terms[i as usize], v_mode, sugar);
            first = false;
        }
        if !first || !is_zero(&tail) {
            if !first {
                result += ",";
            }
            if is_zero(&tail) {
                result += "0";
            } else {
                result += &render_veblen_coeff(&tail, v_mode, sugar);
            }
        }
        result += ")";
        return result;
    }

    // Has non-finite positions — use @ notation
    let mut result = "\\varphi(".to_string();
    let mut first = true;
    for tm in &terms {
        if !first {
            result += ",";
        }
        let coeff;
        if eq(&tm.second, &one()) || is_zero(&tm.second) {
            coeff = if tm.count == 1 { "1".to_string() } else { tm.count.to_string() };
        } else {
            let mut coeff_term = zero();
            for _ in 0..tm.count {
                coeff_term = add(&coeff_term, &tm.second);
            }
            coeff = render_veblen_coeff(&coeff_term, v_mode, sugar);
        }
        let pos = render_position(&tm.exponent, v_mode, sugar);
        result += &format!("{}{{@}}{}", coeff, pos);
        first = false;
    }
    if !is_zero(&tail) {
        if !first {
            result += ",";
        }
        let tcoeff = render_veblen_coeff(&tail, v_mode, sugar);
        result += &format!("{}{{@}}0", tcoeff);
    }
    result += ")";
    result
}

fn render_array_matrix(alpha: &Term, sugar: bool, is_position: bool) -> String {
    if is_zero(alpha) {
        return "1".to_string();
    }

    let mut terms: Vec<OmegaCnfTerm> = Vec::new();
    let mut tail = zero();
    decompose_omega_cnf(alpha, &mut terms, &mut tail);

    let mut has_complex_position = false;
    for tm in &terms {
        if !is_finite_nat(&tm.exponent) {
            has_complex_position = true;
            break;
        }
    }

    if !has_complex_position && terms.is_empty() {
        if is_zero(&tail) {
            return "1".to_string();
        }
        let tn = tail.as_ref().unwrap();
        if is_zero(&tn.c) && is_zero(&tn.a) && has_omega_power_deep(&tail) {
            return render_veblen_coeff_matrix(&tail, sugar);
        }
        {
            let coeff_str = render_veblen_coeff_matrix(&tail, sugar);
            if coeff_str == "1" {
                return "\\omega".to_string();
            }
            return format!("\\omega^{{{}}}", coeff_str);
        }
    }

    if !has_complex_position {
        let mut max_exp = 0;
        for tm in &terms {
            let e = length1(&tm.exponent);
            if e > max_exp {
                max_exp = e;
            }
        }
        let mut coeff_terms: Vec<Term> = vec![zero(); (max_exp + 1) as usize];
        for tm in &terms {
            let pos = length1(&tm.exponent) as usize;
            let mut contrib = zero();
            if eq(&tm.second, &one()) || is_zero(&tm.second) {
                for _ in 0..tm.count {
                    contrib = add(&contrib, &one());
                }
            } else {
                for _ in 0..tm.count {
                    contrib = add(&contrib, &tm.second);
                }
            }
            coeff_terms[pos] = add(&coeff_terms[pos], &contrib);
        }

        if sugar && !is_position {
            let tail_sugar = if is_zero(&tail) {
                "0".to_string()
            } else {
                render_veblen_coeff_matrix(&tail, sugar)
            };
            if max_exp == 1 && eq(&coeff_terms[1], &one()) {
                return format!("\\varepsilon_{{{}}}", tail_sugar);
            }
            if max_exp == 1 && eq(&coeff_terms[1], &add(&one(), &one())) {
                return format!("\\zeta_{{{}}}", tail_sugar);
            }
            if max_exp == 1 && eq(&coeff_terms[1], &add(&add(&one(), &one()), &one())) {
                return format!("\\eta_{{{}}}", tail_sugar);
            }
            if max_exp == 2 && eq(&coeff_terms[2], &one()) && is_zero(&coeff_terms[1]) {
                return format!("\\Gamma_{{{}}}", tail_sugar);
            }
        }

        let mut top = String::new();
        let mut bottom = String::new();
        let mut first = true;
        for i in (1..=max_exp).rev() {
            if !first {
                top += "&";
                bottom += "&";
            }
            top += &render_veblen_coeff_matrix(&coeff_terms[i as usize], sugar);
            bottom += &i.to_string();
            first = false;
        }
        if !is_zero(&tail) {
            if !first {
                top += "&";
                bottom += "&";
            }
            top += &render_veblen_coeff_matrix(&tail, sugar);
            bottom += "0";
        }
        return format!("\\varphi\\begin{{pmatrix}}{}\\\\{}\\end{{pmatrix}}", top, bottom);
    }

    let mut top = String::new();
    let mut bottom = String::new();
    let mut first = true;

    for tm in &terms {
        if !first {
            top += "&";
            bottom += "&";
        }
        let coeff;
        if eq(&tm.second, &one()) || is_zero(&tm.second) {
            coeff = if tm.count == 1 { "1".to_string() } else { tm.count.to_string() };
        } else {
            let mut coeff_term = zero();
            for _ in 0..tm.count {
                coeff_term = add(&coeff_term, &tm.second);
            }
            coeff = render_veblen_coeff_matrix(&coeff_term, sugar);
        }
        top += &coeff;
        bottom += &render_position_matrix(&tm.exponent, sugar);
        first = false;
    }
    if !is_zero(&tail) {
        if !first {
            top += "&";
            bottom += "&";
        }
        let tcoeff = render_veblen_coeff_matrix(&tail, sugar);
        top += &tcoeff;
        bottom += "0";
    }
    format!("\\varphi\\begin{{pmatrix}}{}\\\\{}\\end{{pmatrix}}", top, bottom)
}

fn psi0_to_veblen(alpha: &Term, v_mode: bool, sugar: bool) -> String {
    if is_zero(alpha) {
        return "1".to_string();
    }
    let t_alpha = compute_t(alpha);
    render_array(&t_alpha, v_mode, sugar, false)
}

fn psi0_to_veblen_matrix(alpha: &Term, sugar: bool) -> String {
    if is_zero(alpha) {
        return "1".to_string();
    }
    let t_alpha = compute_t(alpha);
    render_array_matrix(&t_alpha, sugar, false)
}

fn render_veblen_rec(q: &Term, v_mode: bool, sugar: bool) -> String {
    if is_zero(q) {
        return "0".to_string();
    }
    if is_ordinal_finite(q) {
        let len = length1(q);
        return if len <= 1 { "1".to_string() } else { len.to_string() };
    }
    if !is_below_bho(q) {
        return String::new();
    }
    let (head, tail) = separate(q, &first_term(q));
    let hn = head.as_ref().unwrap();
    if !is_zero(&hn.a) {
        return String::new(); // not ψ₀
    }
    let mut result = psi0_to_veblen(&hn.b, v_mode, sugar);
    if result.is_empty() {
        return String::new();
    }

    let mut cur = hn.c.clone();
    while !is_zero(&cur) {
        let cn = cur.as_ref().unwrap();
        if !is_zero(&cn.a) || !eq(&cn.b, &hn.b) {
            break;
        }
        let dup = psi0_to_veblen(&cn.b, v_mode, sugar);
        if dup.is_empty() {
            return String::new();
        }
        result += &format!("+{}", dup);
        cur = cn.c.clone();
    }

    if !is_zero(&cur) {
        let tail_v = render_veblen_rec(&cur, v_mode, sugar);
        if tail_v.is_empty() {
            return String::new();
        }
        result += &format!("+{}", tail_v);
    }
    if !is_zero(&tail) {
        let tail_v = render_veblen_rec(&tail, v_mode, sugar);
        if tail_v.is_empty() {
            return String::new();
        }
        result += &format!("+{}", tail_v);
    }
    result
}

fn render_veblen_rec_matrix(q: &Term, sugar: bool) -> String {
    if is_zero(q) {
        return "0".to_string();
    }
    if is_ordinal_finite(q) {
        let len = length1(q);
        return if len <= 1 { "1".to_string() } else { len.to_string() };
    }
    if !is_below_bho(q) {
        return String::new();
    }
    let (head, tail) = separate(q, &first_term(q));
    let hn = head.as_ref().unwrap();
    if !is_zero(&hn.a) {
        return String::new();
    }
    let mut result = psi0_to_veblen_matrix(&hn.b, sugar);
    if result.is_empty() {
        return String::new();
    }
    let mut cur = hn.c.clone();
    while !is_zero(&cur) {
        let cn = cur.as_ref().unwrap();
        if !is_zero(&cn.a) || !eq(&cn.b, &hn.b) {
            break;
        }
        let dup = psi0_to_veblen_matrix(&cn.b, sugar);
        if dup.is_empty() {
            return String::new();
        }
        result += &format!("+{}", dup);
        cur = cn.c.clone();
    }
    if !is_zero(&cur) {
        let tail_v = render_veblen_rec_matrix(&cur, sugar);
        if tail_v.is_empty() {
            return String::new();
        }
        result += &format!("+{}", tail_v);
    }
    if !is_zero(&tail) {
        let tail_v = render_veblen_rec_matrix(&tail, sugar);
        if tail_v.is_empty() {
            return String::new();
        }
        result += &format!("+{}", tail_v);
    }
    result
}

pub fn term_to_veblen(q: &Term) -> String {
    render_veblen_rec(q, true, true)
}

pub fn term_to_veblen_plain(q: &Term) -> String {
    render_veblen_rec(q, true, false)
}

pub fn term_to_veblen_matrix(q: &Term) -> String {
    render_veblen_rec_matrix(q, true)
}

pub fn term_to_veblen_matrix_plain(q: &Term) -> String {
    render_veblen_rec_matrix(q, false)
}

pub fn term_to_string(_latex: bool, q: &Term) -> String {
    render_term(q)
}

/// Render a Term in raw ψ form (LaTeX). Simplifications:
/// natural numbers as digits, ψ_α(0) = Ω_α, ψ_0 written as ψ,
/// and runs of identical summands merged as ordinal products (ω+ω → ω2).
pub fn term_to_psi_simple(q: &Term) -> String {
    if is_zero(q) {
        return "0".to_string();
    }
    if is_ordinal_finite(q) {
        return length1(q).to_string();
    }
    let mut result = String::new();
    let mut cur = q.clone();
    while !is_zero(&cur) {
        let head = first_term(&cur);
        let (run, rest) = separate(&cur, &head);
        let count = length1(&run);
        let mut part = psi_simple_head(&head);
        if count > 1 {
            part.push_str(&count.to_string());
        }
        if !result.is_empty() {
            result.push('+');
        }
        result.push_str(&part);
        cur = rest;
    }
    result
}

fn psi_simple_head(p: &Term) -> String {
    let node = p.as_ref().unwrap();
    let a = &node.a;
    let b = &node.b;
    if is_zero(b) {
        if is_zero(a) {
            return "1".to_string(); // ψ_0(0) = 1
        }
        if eq(a, &one()) {
            return "\\Omega".to_string();
        }
        return format!("\\Omega_{{{}}}", term_to_psi_simple(a));
    }
    if is_zero(a) {
        return format!("\\psi\\left({}\\right)", term_to_psi_simple(b));
    }
    format!(
        "\\psi_{{{}}}\\left({}\\right)",
        term_to_psi_simple(a),
        term_to_psi_simple(b)
    )
}

/// Pure ψ form: only `ψ`, `(`, `)`, `_`, `+`, `0` appear. Naturals are sums
/// of ψ(0), Ω_α is ψ_α(0), and runs of identical heads are fully expanded
/// (no digit multipliers, no \\Omega, no ordinal products).
pub fn term_to_pure(q: &Term) -> String {
    if is_zero(q) {
        return "0".to_string();
    }
    let mut parts = Vec::new();
    let mut cur = q.clone();
    while !is_zero(&cur) {
        let node = cur.as_ref().unwrap();
        let a = &node.a;
        let b = &node.b;
        if is_zero(a) {
            parts.push(format!("\\psi\\left({}\\right)", term_to_pure(b)));
        } else {
            parts.push(format!(
                "\\psi_{{{}}}\\left({}\\right)",
                term_to_pure(a),
                term_to_pure(b)
            ));
        }
        cur = node.c.clone();
    }
    parts.join("+")
}

// Debug wrappers
pub fn debug_decompose_power(a: &Term) -> (Term, Term) {
    decompose_power(a)
}

pub fn debug_compute_t(alpha: &Term) -> Term {
    compute_t(alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{eval_ast, parse_bocf};

    fn eval(input: &str) -> Term {
        eval_ast(&parse_bocf(input).expect("parse")).expect("eval")
    }

    #[test]
    fn fs_psi_omega_cubed() {
        // ψ(Ω^3)[3] = ψ₀(ψ₁(Ω + ψ₀(ψ₁(Ω + ψ₀(ψ₁(Ω + 1))))))
        let a = eval("ψ(Ω^3)");
        let r = fundamental_sequence(&a, 3);
        let p1 = t(one(), add(&omega1(), &one()), zero()); // ψ₁(Ω+1)
        let q1 = t(zero(), p1.clone(), zero());
        let p2 = t(one(), add(&omega1(), &q1), zero());
        let q2 = t(zero(), p2.clone(), zero());
        let p3 = t(one(), add(&omega1(), &q2), zero());
        let expected = t(zero(), p3, zero());
        assert!(
            eq(&r, &expected),
            "got {}",
            term_to_string(false, &r)
        );
    }

    #[test]
    fn fs_epsilon_zero() {
        // ψ₀(Ω)[3] = ψ₀(ψ₀(ψ₀(1)))
        let a = t(zero(), omega1(), zero());
        let r = fundamental_sequence(&a, 3);
        let one_ = one();
        let q1 = t(zero(), one_, zero());
        let q2 = t(zero(), q1.clone(), zero());
        let expected = t(zero(), q2, zero());
        assert!(
            eq(&r, &expected),
            "got {}",
            term_to_string(false, &r)
        );
    }

    #[test]
    fn psi_simple_display() {
        assert_eq!(term_to_psi_simple(&zero()), "0");
        assert_eq!(term_to_psi_simple(&from_int(3)), "3");
        assert_eq!(term_to_psi_simple(&omega1()), "\\Omega");
        // runs of identical summands merge as ordinal products
        let omega2 = add(&omega1(), &omega1());
        assert_eq!(term_to_psi_simple(&omega2), "\\Omega2");
        let w2 = add(&omega(), &omega());
        assert_eq!(term_to_psi_simple(&w2), "\\psi\\left(1\\right)2");
        assert_eq!(term_to_psi_simple(&add(&w2, &one())), "\\psi\\left(1\\right)2+1");
        let e = eval("ψ(Ω^3)");
        let r = fundamental_sequence(&e, 3);
        assert_eq!(
            term_to_psi_simple(&r),
            "\\psi\\left(\\psi_{1}\\left(\\Omega+\\psi\\left(\\psi_{1}\\left(\\Omega+\\psi\\left(\\psi_{1}\\left(\\Omega+1\\right)\\right)\\right)\\right)\\right)\\right)"
        );
    }

    #[test]
    fn pure_display() {
        assert_eq!(term_to_pure(&zero()), "0");
        // 1 = ψ₀(0)
        assert_eq!(
            term_to_pure(&one()),
            "\\psi\\left(0\\right)"
        );
        // naturals expand to sums of ψ(0), never digits
        assert_eq!(
            term_to_pure(&from_int(3)),
            "\\psi\\left(0\\right)+\\psi\\left(0\\right)+\\psi\\left(0\\right)"
        );
        // ω = ψ₀(1)
        assert_eq!(
            term_to_pure(&omega()),
            "\\psi\\left(\\psi\\left(0\\right)\\right)"
        );
        // Ω = ψ₁(0): subscript 1 = ψ₀(0)
        assert_eq!(
            term_to_pure(&omega1()),
            "\\psi_{\\psi\\left(0\\right)}\\left(0\\right)"
        );
        // only ψ, (, ), _, +, 0 may appear: no digits besides 0, no \Omega
        let s = term_to_pure(&eval("ψ(Ω^3)"));
        assert!(!s.contains("\\Omega"));
        assert!(!s.chars().any(|c| c.is_ascii_digit() && c != '0'));
    }
}
