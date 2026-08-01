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

fn is_finite_nat(t: &Term) -> bool {
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
        // ψβ(0)
        if is_zero(&beta) {
            return zero(); // ψ₀(0)[n] = 0
        }
        let cf_beta = cofinality(&beta);
        if eq(&cf_beta, &one()) {
            return from_int(n); // succ β → n
        }
        return t(fundamental_sequence(&beta, n), zero(), zero()); // limit β → ψβ[n](0)
    }

    let cf_gamma = cofinality(&gamma);
    if eq(&cf_gamma, &one()) {
        // succ argument: ψβ(γ[0]) repeated n times
        let gamma0 = fundamental_sequence(&gamma, 0);
        return mul_finite(&t(beta, gamma0, zero()), &from_int(n));
    }

    // β ≥ Cof(γ): ψβ(γ[n])
    if !lt(&beta, &cf_gamma) {
        return t(beta, fundamental_sequence(&gamma, n), zero());
    }

    // β < Cof(γ): iterate Re for successor cardinals Ω_{δ+1}
    if !is_zero(&cf_gamma) {
        let cfn = cf_gamma.as_ref().unwrap();
        if !is_zero(&cfn.a) && is_succ(&cfn.a) {
            let delta = sub(&cfn.a, &one());
            let mut re = zero();
            for _ in 0..n {
                re = fundamental_sequence_indexed(&gamma, &t(delta.clone(), re.clone(), zero()));
            }
            return t(beta, re, zero());
        }
    }
    t(beta, fundamental_sequence(&gamma, n), zero())
}

/// Fundamental sequence α[β] for ordinal index β.
pub fn fundamental_sequence_indexed(a: &Term, index: &Term) -> Term {
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
        let cf_beta = cofinality(&beta);
        if eq(&cf_beta, &one()) {
            if lt(index, a) {
                return index.clone();
            }
            return zero();
        }
        return t(fundamental_sequence_indexed(&beta, index), zero(), zero());
    }

    let cf_gamma = cofinality(&gamma);
    if eq(&cf_gamma, &one()) {
        return fundamental_sequence(a, length1(index));
    }
    if !lt(&beta, &cf_gamma) {
        return t(beta, fundamental_sequence_indexed(&gamma, index), zero());
    }
    fundamental_sequence(a, length1(index))
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

/// Ω·β.
fn omega_times(beta: &Term) -> Term {
    if is_zero(beta) {
        return zero();
    }
    if is_ordinal_finite(beta) {
        let n = length1(beta);
        let mut result = zero();
        for _ in 0..n {
            result = add(&result, &t(one(), zero(), zero()));
        }
        return result;
    }
    let mut result = zero();
    let mut curr = beta.clone();
    while !is_zero(&curr) {
        let head = first_term(&curr);
        let hn = head.as_ref().unwrap();
        if is_zero(&hn.a) {
            result = add(&result, &t(one(), log(&head), zero()));
        } else {
            result = add(&result, &t(one(), add(&omega1(), &hn.b), zero()));
        }
        curr = curr.as_ref().unwrap().c.clone();
    }
    result
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
                if eq(&last.exponent, &exp_t) && eq(&last.second, &second) {
                    last.count += 1;
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

    let omega_beta = omega_times(&beta);
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
        return full[9..full.len() - 1].to_string();
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
        if m.contains('&') {
            return format!("({})", m);
        }
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
            if is_position && !sugar {
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
        let mut coeff;
        if eq(&tm.second, &one()) || is_zero(&tm.second) {
            coeff = tm.count.to_string();
        } else {
            coeff = render_veblen_coeff(&tm.second, v_mode, sugar);
            if coeff.contains('+') {
                coeff = format!("({})", coeff);
            }
        }
        let pos = render_position(&tm.exponent, v_mode, sugar);
        result += &format!("{}{{@}}{}", coeff, pos);
        first = false;
    }
    if !is_zero(&tail) {
        if !first {
            result += ",";
        }
        let mut tcoeff = render_veblen_coeff(&tail, v_mode, sugar);
        if tcoeff.contains('+') {
            tcoeff = format!("({})", tcoeff);
        }
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
        let mut coeff;
        if eq(&tm.second, &one()) || is_zero(&tm.second) {
            coeff = tm.count.to_string();
        } else {
            coeff = render_veblen_coeff_matrix(&tm.second, sugar);
            if coeff.contains('+') {
                coeff = format!("({})", coeff);
            }
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
        let mut tcoeff = render_veblen_coeff_matrix(&tail, sugar);
        if tcoeff.contains('+') {
            tcoeff = format!("({})", tcoeff);
        }
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

// Debug wrappers
pub fn debug_decompose_power(a: &Term) -> (Term, Term) {
    decompose_power(a)
}

pub fn debug_compute_t(alpha: &Term) -> Term {
    compute_t(alpha)
}
