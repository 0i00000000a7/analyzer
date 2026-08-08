//! BOCF → MOCF (Madore) converter.
//!
//! Translates a Buchholz's OCF expression (BOCF) into Madore's OCF (MOCF)
//! form, following the correspondence observed in `bocf vs mocf.csv`.
//!
//! The translation works on the parsed AST so the explicit cardinal / power
//! structure is preserved.  Core rules (level-0 collapse):
//!   ψ₀(Ω^e·k + r)  e finite ≥2  → ψ(Ω^{e-1}·k + x) · ω^{T(w)}   (r = Ω×x + w)
//!   ψ₀(Ω×k + r)                → ψ(σ(k)) · ω^{T(r)}              (σ(n)=n−1)
//!   ψ₀(Ω^λ·k + r)  λ limit     → ψ(Ω^λ·k + x) · ω^{T(w)}   (fixed point)
//! where T is the value translation (tails: Ω→1, Ω^e→Ω^{e-1}, ψ-terms
//! recursively collapsed) and the display normalizes ω^{ψ(a)·c+rest} → ψ(a)^c·ω^{rest}.

use crate::parser::Ast;

// ════════════════════════════════════════════════════════════════
// Intermediate Cxpr (mirrors the CSV's MOCF display conventions)
// ════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
pub enum C {
    Zero,
    One,
    Nat(i32),
    Omega,
    OmegaSub(Box<C>),
    OmegaPow(Box<C>),
    Psi(Option<Box<C>>, Box<C>),
    Pow(Box<C>, Box<C>),
    Mul(Box<C>, Box<C>),
    Sum(Vec<C>),
}

fn c_nat(n: i32) -> C {
    if n <= 0 { C::Zero } else { C::Nat(n) }
}
fn c_omega() -> C { C::OmegaPow(Box::new(C::One)) }
fn is_c_zero(c: &C) -> bool { matches!(c, C::Zero) }

fn c_sum(parts: Vec<C>) -> C {
    let mut flat: Vec<C> = Vec::new();
    for p in parts {
        if is_c_zero(&p) { continue; }
        if let C::Sum(inner) = p { flat.extend(inner); } else { flat.push(p); }
    }
    if flat.is_empty() { C::Zero }
    else if flat.len() == 1 { flat.pop().unwrap() }
    else { C::Sum(flat) }
}

fn c_mul(a: C, b: C) -> C {
    if is_c_zero(&a) || is_c_zero(&b) { return C::Zero; }
    if matches!(&a, C::One) || matches!(&a, C::Nat(1)) { return b; }
    if matches!(&b, C::One) || matches!(&b, C::Nat(1)) { return a; }
    C::Mul(Box::new(a), Box::new(b))
}

// ════════════════════════════════════════════════════════════════
// AST helpers
// ════════════════════════════════════════════════════════════════

fn flatten_add(n: &Ast, out: &mut Vec<Ast>) {
    match n {
        Ast::Add(l, r) => { flatten_add(l, out); flatten_add(r, out); }
        other => out.push(other.clone()),
    }
}

fn sum_of(blocks: &[Ast]) -> Ast {
    if blocks.is_empty() { return Ast::Num(0); }
    let mut it = blocks.iter().rev();
    let mut acc = it.next().unwrap().clone();
    for b in it {
        acc = Ast::Add(Box::new(b.clone()), Box::new(acc));
    }
    acc
}

fn as_nat(a: &Ast) -> Option<i32> {
    match a { Ast::Num(n) if *n >= 0 => Some(*n), _ => None }
}

/// True if the block represents an ordinal < Ω (= ψ₁(0)).
/// ψ₀(...) values are always below Ω, as are ω-powers and naturals.
fn is_below_omega1(a: &Ast) -> bool {
    match a {
        Ast::Num(_) | Ast::W => true,
        Ast::Psi(None, _) => true,
        Ast::Pow(b, _) => is_below_omega1(b),
        Ast::Mul(l, r) => is_below_omega1(l) && is_below_omega1(r),
        Ast::Omega(_) => false,
        Ast::Psi(Some(_), _) => false,
        Ast::Add(l, r) => is_below_omega1(l) && is_below_omega1(r),
    }
}

/// The leading (largest) cardinal-power shape of a primitive block.
#[derive(Clone, Debug)]
enum Head {
    /// Ω^e  (e = 1 for bare Ω)
    OmegaPow(Ast),
    /// Ω_sub
    Cardinal(Ast),
    /// Ω_sub^e
    CardinalPow(Ast, Ast),
}

fn classify_head(t: &Ast) -> Option<Head> {
    match t {
        Ast::Omega(None) => Some(Head::OmegaPow(Ast::Num(1))),
        Ast::Omega(Some(s)) => Some(Head::Cardinal((**s).clone())),
        Ast::Pow(b, e) => match b.as_ref() {
            Ast::Omega(None) => Some(Head::OmegaPow((**e).clone())),
            Ast::Omega(Some(s)) => Some(Head::CardinalPow((**s).clone(), (**e).clone())),
            _ => None,
        },
        _ => None,
    }
}

/// Split a block into its cardinal-power head and its multiplier k,
/// recursing through nested products (Ω·a·b·... = head with multiplier a·b·...).
fn split_head_mult(block: &Ast) -> (Option<Head>, Ast) {
    match block {
        Ast::Mul(b, k) => match split_head_mult(b) {
            (Some(h), m) => {
                let m = if matches!(m, Ast::Num(1)) {
                    (**k).clone()
                } else {
                    Ast::Mul(Box::new(m), k.clone())
                };
                (Some(h), m)
            }
            (None, _) => (None, Ast::Num(1)),
        },
        other => (classify_head(other), Ast::Num(1)),
    }
}

/// Subtract one Ω-power level from a tail block (for the value translation).
fn translate_down(block: &Ast) -> Ast {
    match block {
        Ast::Omega(None) => Ast::Num(1),
        Ast::Mul(b, k) if matches!(b.as_ref(), Ast::Omega(None)) => (**k).clone(),
        Ast::Mul(b, _)
            if matches!(b.as_ref(), Ast::Omega(Some(sub)) if is_successor_ord(sub)) =>
        {
            match b.as_ref() {
                Ast::Omega(Some(_)) => match block {
                    Ast::Mul(_, k) => (**k).clone(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
        Ast::Mul(b, k) if split_head_mult(b).0.is_some() => {
            let tb = translate_down(b);
            if matches!(tb, Ast::Num(1)) { (**k).clone() } else { Ast::Mul(Box::new(tb), k.clone()) }
        }
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(None)) => {
            if let Some(n) = as_nat(e) {
                Ast::Pow(Box::new(Ast::Omega(None)), Box::new(Ast::Num(n - 1)))
            } else {
                block.clone()
            }
        }
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(Some(_))) => {
            if let Some(n) = as_nat(e) {
                if n >= 2 {
                    Ast::Pow(b.clone(), Box::new(Ast::Num(n - 1)))
                } else {
                    block.clone()
                }
            } else {
                block.clone()
            }
        }
        _ => block.clone(),
    }
}

// ════════════════════════════════════════════════════════════════
// Conversion
// ════════════════════════════════════════════════════════════════

pub fn bocf_to_mocf(input: &str) -> Result<String, String> {
    bocf_to_c(input).map(|c| render(&c))
}

/// Map a BOCF Term directly to its canonical MOCF string (forward
/// converter used as an oracle by the inverse direction).
pub fn term_to_mocf(t: &crate::term::Term) -> String {
    let s = crate::term::standard_form(t);
    render(&normalize(conv_ord(&term_to_ast(&s))))
}

/// Full pipeline up to the intermediate MOCF value.
fn bocf_to_c(input: &str) -> Result<C, String> {
    let ast = crate::parser::parse_bocf(input)?;
    match crate::parser::eval_ast(&ast) {
        Ok(t) => {
            let s = crate::term::standard_form(&t);
            Ok(normalize(conv_ord(&term_to_ast(&s))))
        }
        Err(_) => {
            let ast = rewrite_nonstandard_psi(&ast);
            Ok(normalize(conv_ord(&ast)))
        }
    }
}

/// Collapse ψ_v-blocks (v ≥ 1) that are not in standard form (e.g.
/// ψ_2(ψ_2(ψ_2(0))+Ω) → Ω_2^{Ω_2}·Ω) by a term round-trip; standard
/// ψ-blocks and everything else stay syntactically untouched.
fn rewrite_nonstandard_psi(a: &Ast) -> Ast {
    match a {
        Ast::Psi(Some(sub), arg) => {
            let arg = rewrite_nonstandard_psi(arg);
            let sub = rewrite_nonstandard_psi(sub);
            let node = Ast::Psi(Some(Box::new(sub.clone())), Box::new(arg));
            match crate::parser::eval_ast(&node) {
                Ok(t) => {
                    let s = crate::term::standard_form(&t);
                    let same_head = match &s {
                        Some(n) if !crate::term::is_zero(&n.a) => {
                            crate::term::eq(&n.a, &term_of_ast(&sub))
                                && !crate::term::is_zero(&n.b)
                        }
                        _ => false,
                    };
                    if same_head { node } else { term_to_ast(&s) }
                }
                Err(_) => node,
            }
        }
        Ast::Psi(None, arg) => Ast::Psi(None, Box::new(rewrite_nonstandard_psi(arg))),
        Ast::Add(l, r) => Ast::Add(
            Box::new(rewrite_nonstandard_psi(l)),
            Box::new(rewrite_nonstandard_psi(r)),
        ),
        Ast::Mul(l, r) => Ast::Mul(
            Box::new(rewrite_nonstandard_psi(l)),
            Box::new(rewrite_nonstandard_psi(r)),
        ),
        Ast::Pow(b, e) => Ast::Pow(
            Box::new(rewrite_nonstandard_psi(b)),
            Box::new(rewrite_nonstandard_psi(e)),
        ),
        _ => a.clone(),
    }
}

fn term_of_ast(a: &Ast) -> crate::term::Term {
    crate::parser::eval_ast(a).unwrap_or_else(|_| crate::term::zero())
}

/// Convert a normalized Term back to an Ast, mirroring render_term:
/// ψ_a(b) with b < Ω_{a+1} decomposes to Ω_a^first·second, ψ_a(0) = Ω_a,
/// runs of identical summands become natural products.
fn term_to_ast(q: &crate::term::Term) -> Ast {
    use crate::term as tm;
    if tm::is_zero(q) {
        return Ast::Num(0);
    }
    if tm::is_ordinal_finite(q) {
        return Ast::Num(tm::length1(q));
    }
    let mut parts: Vec<Ast> = Vec::new();
    let mut cur = q.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let count = tm::length1(&run);
        let part = term_block_ast(&head, count);
        parts.push(part);
        cur = rest;
    }
    sum_of(&parts)
}

fn omega_a_ast(a: &crate::term::Term) -> Ast {
    use crate::term as tm;
    if tm::is_zero(a) {
        Ast::W
    } else if tm::eq(a, &tm::one()) {
        Ast::Omega(None)
    } else {
        Ast::Omega(Some(Box::new(term_to_ast(a))))
    }
}

fn term_block_ast(p: &crate::term::Term, count: i32) -> Ast {
    use crate::term as tm;
    let node = p.as_ref().unwrap();
    let a = &node.a;
    let b = &node.b;
    let psi_form = || {
        if tm::is_zero(a) {
            Ast::Psi(None, Box::new(term_to_ast(b)))
        } else {
            Ast::Psi(Some(Box::new(term_to_ast(a))), Box::new(term_to_ast(b)))
        }
    };
    let ast = if tm::is_zero(b) {
        if tm::is_zero(a) { Ast::Num(1) } else { omega_a_ast(a) }
    } else if tm::is_zero(a) && tm::eq(b, &tm::one()) {
        Ast::W
    } else if tm::lt(b, &tm::t(tm::succ(a), tm::zero(), tm::zero())) {
        let (first, second) = tm::decompose_power(p);
        if tm::eq(&first, p) {
            psi_form()
        } else {
            let mut m = if tm::gt(&first, &tm::one()) {
                Ast::Pow(Box::new(omega_a_ast(a)), Box::new(term_to_ast(&first)))
            } else {
                omega_a_ast(a)
            };
            if tm::gt(&second, &tm::one()) {
                m = Ast::Mul(Box::new(m), Box::new(term_to_ast(&second)));
            }
            m
        }
    } else {
        psi_form()
    };
    if count > 1 {
        Ast::Mul(Box::new(ast), Box::new(Ast::Num(count)))
    } else {
        ast
    }
}

/// Convert a BOCF ordinal *value* (tails / arguments / exponents).
fn conv_ord(n: &Ast) -> C {
    match n {
        Ast::Num(k) => c_nat(*k),
        Ast::W => c_omega(),
        Ast::Omega(None) => C::Omega,
        Ast::Omega(Some(s)) => C::OmegaSub(Box::new(conv_ord(s))),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(n, &mut blocks);
            c_sum(blocks.iter().map(conv_ord).collect())
        }
        Ast::Mul(l, r) => c_mul(conv_ord(l), conv_ord(r)),
        Ast::Pow(b, e) => conv_pow(b, e),
        Ast::Psi(sub, arg) => conv_psi(sub.as_deref(), arg),
    }
}

fn conv_pow(base: &Ast, exp: &Ast) -> C {
    match base {
        Ast::W => C::OmegaPow(Box::new(conv_ord(exp))),
        Ast::Omega(None) => C::Pow(Box::new(C::Omega), Box::new(conv_ord(exp))),
        Ast::Omega(Some(_)) => C::Pow(Box::new(conv_ord(base)), Box::new(conv_ord(exp))),
        _ => C::Pow(Box::new(conv_ord(base)), Box::new(conv_ord(exp))),
    }
}

/// Convert ψ_v(a).  v = None is the level-0 collapse (the main region).
fn conv_psi(sub: Option<&Ast>, arg: &Ast) -> C {
    match sub {
        None => conv_psi0(arg),
        Some(v) => {
            // Level-shift: the subscript collapses the next cardinal.
            let vc = conv_ord(v);
            let argc = conv_at_level(v, arg);
            C::Psi(Some(Box::new(vc)), Box::new(argc))
        }
    }
}

/// Convert the argument of ψ_v (v ≥ 1): Ω_{v+1}-leading terms collapse,
/// higher cardinals become their own collapse value, lower ones stay.
fn conv_at_level(v: &Ast, arg: &Ast) -> C {
    let mut blocks = Vec::new();
    flatten_add(arg, &mut blocks);
    if blocks.is_empty() { return C::Zero; }
    let (head_opt, mult) = split_head_mult(&blocks[0]);
    let tail = sum_of(&blocks[1..]);
    let vn = as_nat(v);
    let next = vn.map(|n| n + 1); // Ω_{v+1}

    match head_opt {
        Some(Head::Cardinal(s)) => {
            let sv = as_nat(&s);
            if let (Some(sn), Some(nx)) = (sv, next) {
                if sn == nx {
                    // Ω_{v+1} · k collapses to σ(k)
                    let sk = sigma(&mult);
                    c_sum(vec![sk, conv_ord(&tail)])
                } else if sn > nx {
                    // Ω_s (s a successor) above the collapse cardinal Ω_{v+1}
                    // appears as its own collapse value ψ_{s-1}.
                    let ps = pred_ord(&s);
                    let inner = C::Psi(Some(Box::new(conv_ord(&ps))), Box::new(sigma(&mult)));
                    c_sum(vec![inner, conv_ord(&tail)])
                } else {
                    // below the collapse cardinal → value
                    let headc = conv_ord(&blocks[0]);
                    c_sum(vec![headc, conv_ord(&tail)])
                }
            } else {
                let headc = conv_ord(&blocks[0]);
                c_sum(vec![headc, conv_ord(&tail)])
            }
        }
        _ => {
            // Otherwise convert everything as values (lower cardinals stay,
            // higher cardinals / ψ-terms collapse recursively).
            let headc = conv_ord(&blocks[0]);
            let tailc = conv_ord(&tail);
            c_sum(vec![headc, tailc])
        }
    }
}

/// σ(k): finite naturals decrement, everything else converts as a value.
fn sigma(k: &Ast) -> C {
    if let Some(n) = as_nat(k) {
        c_nat(n - 1)
    } else {
        conv_ord(k)
    }
}

/// The level-0 collapse ψ₀(arg).
fn conv_psi0(arg: &Ast) -> C {
    let mut blocks = Vec::new();
    flatten_add(arg, &mut blocks);
    if blocks.is_empty() {
        // ψ₀(0) = 1
        return C::Nat(1);
    }
    let (head_opt, mult) = split_head_mult(&blocks[0]);
    let tail = sum_of(&blocks[1..]);

    match head_opt {
        None => {
            // Leading block is not a cardinal → α < Ω, so ψ₀(α) = ω^T(α).
            C::OmegaPow(Box::new(conv_ord(&sum_of(&blocks))))
        }
        Some(Head::OmegaPow(e)) => {
            if let Some(n) = as_nat(&e) {
                if n == 1 {
                    // Ω · k + r
                    collapse_omega1(&mult, &tail)
                } else {
                    // Ω^n · k + r, n finite ≥ 2
                    collapse_omegapow_finite(n, &mult, &tail)
                }
            } else {
                // Fixed point: Ω^λ (λ limit)
                collapse_fixed_omegapow(&e, &mult, &tail)
            }
        }
        Some(Head::Cardinal(s)) => {
            if is_successor_ord(&s) {
                collapse_cardinal_succ(&s, &mult, &tail)
            } else {
                // Ω_λ (λ limit, e.g. Ω_ω, Ω_Ω) → fixed point
                collapse_fixed_cardinal(&s, &mult, &tail)
            }
        }
        Some(Head::CardinalPow(s, e)) => {
            if let Some(n) = as_nat(&e) {
                if n == 1 {
                    if is_successor_ord(&s) {
                        collapse_cardinal_succ(&s, &mult, &tail)
                    } else {
                        collapse_fixed(C::OmegaSub(Box::new(conv_ord(&s))), &mult, &tail)
                    }
                } else {
                    // Ω_sub^n, n finite ≥ 2
                    if is_successor_ord(&s) {
                        collapse_cardinalpow_succ(&s, n, &mult, &tail)
                    } else {
                        collapse_fixed(make_cardinalpow(&conv_ord(&s), &conv_ord(&e)), &mult, &tail)
                    }
                }
            } else {
                collapse_fixed(make_cardinalpow(&conv_ord(&s), &conv_ord(&e)), &mult, &tail)
            }
        }
    }
}

fn make_omegapow(exp: &C) -> C {
    C::Pow(Box::new(C::Omega), Box::new(exp.clone()))
}
fn make_cardinalpow(sub: &C, exp: &C) -> C {
    C::Pow(Box::new(C::OmegaSub(Box::new(sub.clone()))), Box::new(exp.clone()))
}

/// ψ₀(Ω×X + r).  The tail r splits into Ω-leading parts (merged into the
/// multiplier), ψ(Ω+·)-blocks contributing to the exponent E, and a small rest
/// becoming an ω-power factor.
fn collapse_omega1(mult: &Ast, tail: &Ast) -> C {
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_parts: Vec<Ast> = vec![mult.clone()];
    let mut psi_blocks: Vec<Ast> = Vec::new();
    let mut small: Vec<Ast> = Vec::new();
    for b in &blocks {
        if !is_below_omega1(b) {
            x_parts.push(translate_down(b));
        } else if as_psi_omega_block(b).is_some() {
            psi_blocks.push(b.clone());
        } else {
            small.push(b.clone());
        }
    }
    let x = sum_of(&x_parts);
    let sigma_x = sigma(&x);
    let k_ge_2 = matches!(&sigma_x, C::Nat(n) if *n >= 1);
    let small_c = if small.is_empty() { C::Zero } else { conv_ord(&sum_of(&small)) };

    if psi_blocks.is_empty() {
        let psi = C::Psi(None, Box::new(sigma_x));
        return if is_c_zero(&small_c) { psi } else { c_mul(psi, C::OmegaPow(Box::new(small_c))) };
    }

    if k_ge_2 {
        // ψ(σ(X)) · M(ψ-blocks) · ω^{M(small)}: pure ψ-powers stay direct.
        let wc = normalize(conv_ord(&sum_of(&psi_blocks)));
        let factor = if is_pure_psi_pow(&wc) { wc } else { C::OmegaPow(Box::new(wc)) };
        let psi = c_mul(C::Psi(None, Box::new(sigma_x)), factor);
        return if is_c_zero(&small_c) { psi } else { c_mul(psi, C::OmegaPow(Box::new(small_c))) };
    }

    // k == 1: exponent machinery.  E accumulates the ψ(Ω+·)-block
    // contributions; T = ψ(0)·E when a deeper block (y ≠ 0) is present,
    // else T = E.
    let mut e: Option<C> = None;
    let mut has_deep = false;
    for b in &psi_blocks {
        let (y, m) = as_psi_omega_block(b).unwrap();
        let contrib = g_contrib(&y, &m);
        e = Some(match e {
            None => contrib,
            Some(prev) => combine_e(prev, &contrib),
        });
        if !is_zero_ast(&y) {
            has_deep = true;
        }
    }
    let e = e.unwrap();
    let t = if has_deep { c_mul(psi0_c(), e) } else { e };
    let exp = if is_c_zero(&small_c) { t } else { c_sum(vec![t, small_c]) };
    c_mul(C::Psi(None, Box::new(C::Zero)), C::OmegaPow(Box::new(exp)))
}

/// ψ(0) as a C value.
fn psi0_c() -> C { C::Psi(None, Box::new(C::Zero)) }
fn is_psi0_c(c: &C) -> bool { matches!(c, C::Psi(None, a) if matches!(a.as_ref(), C::Zero)) }

/// True if c is ψ(a)^n with finite n ≥ 2 (a pure ψ-power).
fn is_pure_psi_pow(c: &C) -> bool {
    matches!(c, C::Pow(b, e) if matches!(b.as_ref(), C::Psi(..)) && matches!(e.as_ref(), C::Nat(n) if *n >= 2))
}

/// Decompose a block of the form ψ(Ω+y)·m, returning (y, m).
fn as_psi_omega_block(b: &Ast) -> Option<(Ast, Ast)> {
    let (inner, m) = match b {
        Ast::Mul(p, k) => ((**p).clone(), (**k).clone()),
        _ => (b.clone(), Ast::Num(1)),
    };
    match &inner {
        Ast::Psi(None, arg) => match arg.as_ref() {
            Ast::Omega(None) => Some((Ast::Num(0), m)),
            Ast::Add(l, r) if matches!(l.as_ref(), Ast::Omega(None)) => Some(((**r).clone(), m)),
            _ => None,
        },
        _ => None,
    }
}

/// Exponent contribution of a ψ(Ω+y)·m block at level 0.
fn g_contrib(y: &Ast, m: &Ast) -> C {
    let mv = conv_ord(m);
    if is_zero_ast(y) {
        // ψ(Ω)·m → ψ(0)·m
        return c_mul(psi0_c(), mv);
    }
    if let Ast::Psi(None, inner) = y {
        if let Ast::Omega(None) = inner.as_ref() {
            // ψ(Ω+ψ(Ω))·m → ψ(0)·m
            return c_mul(psi0_c(), mv);
        }
        if let Ast::Add(l, r) = inner.as_ref() {
            if matches!(l.as_ref(), Ast::Omega(None)) {
                // y = ψ(Ω+z): F₃(z)·m = ψ(0)^{M(z)}·(z==1?ω:1)·m
                let z = &**r;
                let p = C::Pow(Box::new(psi0_c()), Box::new(conv_ord(z)));
                let f3 = if matches!(z, Ast::Num(1)) { c_mul(p, c_omega()) } else { p };
                return c_mul(f3, mv);
            }
        }
    }
    // Otherwise ω^{M(y)}·m
    c_mul(C::OmegaPow(Box::new(conv_ord(y))), mv)
}

/// Combine a trailing ψ(Ω)·j contribution (as ψ(0)·j) into the exponent E.
fn combine_e(prev: C, contrib: &C) -> C {
    let j = match contrib {
        C::Mul(a, b) if is_psi0_c(a) => (**b).clone(),
        c if is_psi0_c(c) => C::One,
        _ => return c_sum(vec![prev, contrib.clone()]),
    };
    let prev_small = match &prev {
        C::OmegaPow(_) => true,
        C::Mul(a, _) => matches!(a.as_ref(), C::OmegaPow(_)),
        _ => false,
    };
    if prev_small {
        // E < ψ(0): add j
        c_sum(vec![prev, j])
    } else if matches!(&j, C::One) {
        c_sum(vec![prev, C::One])
    } else {
        c_mul(prev, j)
    }
}

/// ψ₀(Ω^n·k + r) → ψ(Ω^{n-1}·k + x) · ω^{T(w)}, with r = Ω×x + w.
fn collapse_omegapow_finite(n: i32, mult: &Ast, tail: &Ast) -> C {
    let convk = conv_ord(mult);
    let lead = c_mul(make_omegapow(&c_nat(n - 1)), convk);
    let k_ge_2 = matches!(as_nat(mult), Some(v) if v >= 2);
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_parts: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    let mut in_w = false;
    for b in &blocks {
        if is_c_zero(&conv_ord(b)) { continue; }
        if !in_w && !is_below_omega1(b) {
            let tb = translate_down(b);
            let part = if k_ge_2 {
                raw_arg_for(&tb).unwrap_or_else(|| conv_ord(&tb))
            } else {
                conv_ord(&tb)
            };
            x_parts.push(part);
        } else {
            in_w = true;
            w_parts.push(b.clone());
        }
    }
    let w = sum_of(&w_parts);
    let arg = c_sum(std::iter::once(lead).chain(x_parts).collect());
    let psi = C::Psi(None, Box::new(arg));
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

/// For a pure ψ(Ω^e) block (e ≥ 2), the k≥2 addend is the un-wrapped
/// collapse argument ψ(Ω^{e-1} + ψ(Ω^{e-1})).
fn raw_arg_for(b: &Ast) -> Option<C> {
    if let Ast::Psi(None, arg) = b {
        if let Ast::Pow(bb, e) = arg.as_ref() {
            if matches!(bb.as_ref(), Ast::Omega(None)) {
                if let Some(ev) = as_nat(e) {
                    if ev >= 2 {
                        let inner = make_omegapow(&c_nat(ev - 1));
                        return Some(C::Psi(None, Box::new(c_sum(vec![
                            inner.clone(),
                            C::Psi(None, Box::new(inner)),
                        ]))));
                    }
                }
            }
        }
    }
    None
}

/// ψ₀(Ω_sub^n·k + r) with sub a successor, n finite ≥ 2 (rows 434-441):
/// lead Ω_sub^{n-1}·k; tails Ω_sub·X become ψ_{sub-1}(Ω_sub + M(X)) and
/// ψ_{sub-1}-arguments shift Ω_sub-powers down by one.
fn collapse_cardinalpow_succ(s: &Ast, n: i32, mult: &Ast, tail: &Ast) -> C {
    let pred = pred_ord(s);
    let convk = conv_sym(&card_arg_shift(s, mult));
    let lead = c_mul(make_cardinalpow(&conv_ord(s), &c_nat(n - 1)), convk);
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_c: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    for b in &blocks {
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            continue;
        }
        let psi_pred = match b {
            Ast::Mul(p, k) => match p.as_ref() {
                Ast::Psi(Some(sub), arg) if ast_eq(sub, &pred) => {
                    Some(((**arg).clone(), (**k).clone()))
                }
                _ => None,
            },
            Ast::Psi(Some(sub), arg) if ast_eq(sub, &pred) => {
                Some(((**arg).clone(), Ast::Num(1)))
            }
            _ => None,
        };
        if let Some((parg, pm)) = psi_pred {
            let xc = C::Psi(
                Some(Box::new(conv_ord(&pred))),
                Box::new(conv_sym(&card_arg_shift(s, &parg))),
            );
            if matches!(pm, Ast::Num(1)) {
                x_c.push(xc);
            } else {
                x_c.push(c_mul(xc, conv_sym(&card_arg_shift(s, &pm))));
            }
            continue;
        }
        let (h, m) = split_head_mult(b);
        match &h {
            Some(Head::Cardinal(s2)) if ast_eq(s2, s) => {
                // Ω_s·X → ψ_{s-1}(Ω_s + M(X))
                let xc = conv_sym(&card_arg_shift(s, &m));
                x_c.push(C::Psi(
                    Some(Box::new(conv_ord(&pred))),
                    Box::new(c_sum(vec![C::OmegaSub(Box::new(conv_ord(s))), xc])),
                ));
            }
            _ => {
                x_c.push(conv_ord(&translate_down(b)));
            }
        }
    }
    let arg = if x_c.is_empty() {
        lead
    } else {
        let mut parts = vec![lead];
        parts.extend(x_c);
        c_sum(parts)
    };
    let psi = C::Psi(None, Box::new(arg));
    let w = sum_of(&w_parts);
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

/// Shift Ω_s-powers down one inside a ψ_{s-1}-region term: Ω_s^e → Ω_s^{e-1}
/// (finite e ≥ 2), recursing through products, sums and ψ-arguments.
fn card_arg_shift(s: &Ast, a: &Ast) -> Ast {
    match a {
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(Some(x)) if ast_eq(x, s)) => {
            if let Some(n) = as_nat(e) {
                if n >= 2 {
                    Ast::Pow(b.clone(), Box::new(Ast::Num(n - 1)))
                } else {
                    a.clone()
                }
            } else {
                a.clone()
            }
        }
        Ast::Psi(sub, arg) => Ast::Psi(sub.clone(), Box::new(card_arg_shift(s, arg))),
        Ast::Mul(l, r) => Ast::Mul(
            Box::new(card_arg_shift(s, l)),
            Box::new(card_arg_shift(s, r)),
        ),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(a, &mut blocks);
            sum_of(&blocks.iter().map(|b| card_arg_shift(s, b)).collect::<Vec<_>>())
        }
        _ => a.clone(),
    }
}

/// ψ₀(Ω_sub·k + r) → ψ(ψ_{pred(sub)}(σ(k)) + x) · ω^{T(w)}, with the
/// level-v exponent machinery for ψ_v(Ω_{v+1}·j + y)·m tails.
fn collapse_cardinal_succ(s: &Ast, mult: &Ast, tail: &Ast) -> C {
    let sub_idx = pred_ord(s);
    let vc = conv_ord(&sub_idx);
    let inner = C::Psi(Some(Box::new(vc.clone())), Box::new(sigma(mult)));

    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_parts: Vec<Ast> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    let mut contribs: Vec<C> = Vec::new();
    for b in &blocks {
        if let Some((j, y, m)) = as_psi_card_block(b, &sub_idx, s) {
            let base_j = C::Psi(Some(Box::new(vc.clone())), Box::new(sigma(&j)));
            let e = e_val_level(&sub_idx, s, &y);
            contribs.push(c_mul(base_j, c_mul(e, conv_ord(&m))));
        } else if is_below_omega1(b) {
            w_parts.push(b.clone());
        } else {
            x_parts.push(translate_down(b));
        }
    }
    let x_c = if x_parts.is_empty() { C::Zero } else { conv_ord(&sum_of(&x_parts)) };
    let mut parts: Vec<C> = vec![inner];
    parts.extend(contribs);
    if !is_c_zero(&x_c) {
        parts.push(x_c);
    }
    let arg = c_sum(parts);
    let psi = C::Psi(None, Box::new(arg));
    let w = sum_of(&w_parts);
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

/// Decompose a block ψ_v(Ω_s·j + y)·m (subscript v = pred(s)).
fn as_psi_card_block(b: &Ast, v_ast: &Ast, s_ast: &Ast) -> Option<(Ast, Ast, Ast)> {
    let (inner, m) = match b {
        Ast::Mul(p, k) => ((**p).clone(), (**k).clone()),
        _ => (b.clone(), Ast::Num(1)),
    };
    match &inner {
        Ast::Psi(Some(sub), arg) if ast_eq(sub, v_ast) => {
            let mut ablocks = Vec::new();
            flatten_add(arg, &mut ablocks);
            if ablocks.is_empty() {
                return None;
            }
            let (h, j) = split_head_mult(&ablocks[0]);
            match h {
                Some(Head::Cardinal(s2)) if ast_eq(&s2, s_ast) => {
                    Some((j, sum_of(&ablocks[1..]), m))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// E-value of a level-v tail remainder y (rows 412-423):
/// 0 → 1; ψ_v(Ω_s·j + z)·h → ψ_v(j-1)^{E(z)+(h-1)}; otherwise the
/// structural value, wrapped in ω^ when below Ω.
fn e_val_level(v_ast: &Ast, s_ast: &Ast, y: &Ast) -> C {
    if is_zero_ast(y) {
        return C::One;
    }
    if let Some((j, z, h)) = as_psi_card_block(y, v_ast, s_ast) {
        let vc = conv_ord(v_ast);
        let base = C::Psi(Some(Box::new(vc)), Box::new(sigma(&j)));
        let ez = e_val_level(v_ast, s_ast, &z);
        let hm1 = match as_nat(&h) {
            Some(n) if n >= 1 => c_nat(n - 1),
            _ => conv_ord(&h),
        };
        let exp = c_add_ord(ez, hm1);
        return if matches!(&exp, C::One) || matches!(&exp, C::Nat(1)) {
            base
        } else {
            C::Pow(Box::new(base), Box::new(exp))
        };
    }
    let mc = conv_struct_level(v_ast, s_ast, y);
    if is_below_c(&mc) {
        C::OmegaPow(Box::new(mc))
    } else {
        mc
    }
}

/// Structural level-v conversion of a tail remainder: Ω_{v+1} becomes
/// ψ_v(0) inside ψ-arguments, everything else converts as a value.
fn conv_struct_level(v_ast: &Ast, s_ast: &Ast, y: &Ast) -> C {
    match y {
        Ast::Omega(Some(x)) if ast_eq(x, s_ast) => {
            C::Psi(Some(Box::new(conv_ord(v_ast))), Box::new(C::Zero))
        }
        Ast::Psi(sub, arg) => C::Psi(
            sub.as_ref().map(|t| Box::new(conv_ord(t))),
            Box::new(conv_struct_level(v_ast, s_ast, arg)),
        ),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(y, &mut blocks);
            c_sum(blocks.iter().map(|b| conv_struct_level(v_ast, s_ast, b)).collect())
        }
        Ast::Mul(l, r) => c_mul(
            conv_struct_level(v_ast, s_ast, l),
            conv_struct_level(v_ast, s_ast, r),
        ),
        Ast::Pow(b, e) => C::Pow(
            Box::new(conv_struct_level(v_ast, s_ast, b)),
            Box::new(conv_struct_level(v_ast, s_ast, e)),
        ),
        _ => conv_ord(y),
    }
}

/// True if the C value is below Ω.
fn is_below_c(c: &C) -> bool {
    match c {
        C::Zero | C::One | C::Nat(_) | C::OmegaPow(_) => true,
        C::Psi(None, _) => true,
        C::Mul(a, b) => is_below_c(a) && is_below_c(b),
        C::Sum(ts) => ts.iter().all(is_below_c),
        _ => false,
    }
}

/// ψ₀(lead · k + r) → ψ(lead·k + x) · ω^{T(w)} (lead is a fixed point).
/// ψ_v-blocks in the tail evaluate; Ω-power blocks keep ψ-values inside
/// their exponents symbolic (rows 450 vs 454).
fn collapse_fixed(lead: C, mult: &Ast, tail: &Ast) -> C {
    let convk = conv_ord(mult);
    let lead = c_mul(lead, convk);
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_c: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    for b in &blocks {
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            continue;
        }
        let is_psi_block = match b {
            Ast::Psi(..) => true,
            Ast::Mul(p, _) if matches!(p.as_ref(), Ast::Psi(..)) => true,
            _ => false,
        };
        x_c.push(if is_psi_block {
            conv_ord(b)
        } else {
            conv_sym(&translate_down(b))
        });
    }
    let arg = if x_c.is_empty() {
        lead
    } else {
        let mut parts = vec![lead];
        parts.extend(x_c);
        c_sum(parts)
    };
    let psi = C::Psi(None, Box::new(arg));
    let w = sum_of(&w_parts);
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

/// ψ₀(Ω_λ·k + r) with λ a limit (rows 507-526): like collapse_fixed, but
/// tails Ω_s·X with s a successor ≥ 2 become ψ_{s-1}(lead + M(X)).
fn collapse_fixed_cardinal(s: &Ast, mult: &Ast, tail: &Ast) -> C {
    let leadc = C::OmegaSub(Box::new(conv_ord(s)));
    let lead = c_mul(leadc.clone(), conv_ord(mult));
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_c: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    for b in &blocks {
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            continue;
        }
        let (h, m) = split_head_mult(b);
        match &h {
            Some(Head::Cardinal(s2)) if is_successor_ord(s2) && as_nat(s2).map_or(false, |n| n >= 2) => {
                x_c.push(C::Psi(
                    Some(Box::new(conv_ord(&pred_ord(s2)))),
                    Box::new(c_sum(vec![leadc.clone(), conv_ord(&m)])),
                ));
            }
            _ => {
                x_c.push(conv_ord(&translate_down(b)));
            }
        }
    }
    let arg = if x_c.is_empty() {
        lead
    } else {
        let mut parts = vec![lead];
        parts.extend(x_c);
        c_sum(parts)
    };
    let psi = C::Psi(None, Box::new(arg));
    let w = sum_of(&w_parts);
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

/// Symbolic conversion: like conv_ord, but ψ-values are evaluated only when
/// their argument is nonzero and below Ω; ψ(0) and ψ-values with cardinal
/// arguments stay symbolic (rows 187, 233, 278, 310, 358-362).
fn conv_sym(a: &Ast) -> C {
    match a {
        Ast::Psi(None, arg)
            if !matches!(arg.as_ref(), Ast::Num(0)) && is_below_omega1(arg) =>
        {
            conv_psi0(arg)
        }
        Ast::Num(k) => c_nat(*k),
        Ast::W => c_omega(),
        Ast::Omega(None) => C::Omega,
        Ast::Omega(Some(s)) => C::OmegaSub(Box::new(conv_sym(s))),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(a, &mut blocks);
            c_sum(blocks.iter().map(conv_sym).collect())
        }
        Ast::Mul(l, r) => c_mul(conv_sym(l), conv_sym(r)),
        Ast::Pow(b, e) => C::Pow(Box::new(conv_sym(b)), Box::new(conv_sym(e))),
        Ast::Psi(sub, arg) => C::Psi(
            sub.as_ref().map(|s| Box::new(conv_sym(s))),
            Box::new(conv_sym(arg)),
        ),
    }
}

fn ast_eq(a: &Ast, b: &Ast) -> bool {
    format!("{:?}", a) == format!("{:?}", b)
}

/// Deep translation inside a fixed-point argument: a bare trailing Ω becomes
/// 1; ψ-blocks recurse into their arguments; everything else is kept.
fn translate_deep(a: &Ast, _f: &Ast) -> Ast {
    match a {
        Ast::Add(l, r) => {
            if matches!(r.as_ref(), Ast::Omega(None)) {
                Ast::Add(l.clone(), Box::new(Ast::Num(1)))
            } else {
                Ast::Add(l.clone(), Box::new(translate_deep(r, _f)))
            }
        }
        Ast::Psi(sub, arg) => Ast::Psi(sub.clone(), Box::new(translate_deep(arg, _f))),
        Ast::Mul(l, r) => Ast::Mul(Box::new(translate_deep(l, _f)), r.clone()),
        Ast::Pow(b, e) => Ast::Pow(b.clone(), Box::new(translate_deep(e, _f))),
        _ => a.clone(),
    }
}

fn is_fixed_block(b: &Ast) -> bool {
    match split_head_mult(b) {
        (Some(Head::OmegaPow(e)), _) => as_nat(&e).is_none(),
        _ => false,
    }
}

/// In an exponent whose leading block is a fixed point (Ω^λ, λ limit), a
/// trailing bare Ω is absorbed to 1 (row 323); recurse into Ω-powers.
fn exp_shift(a: &Ast) -> Ast {
    match a {
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(a, &mut blocks);
            let last = blocks.last().unwrap();
            let shifted = matches!(last, Ast::Omega(None)) && is_fixed_block(&blocks[0]);
            let mut out: Vec<Ast> = blocks.clone();
            if shifted {
                let n = out.len();
                out[n - 1] = Ast::Num(1);
            }
            sum_of(&out)
        }
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(None)) => {
            Ast::Pow(b.clone(), Box::new(exp_shift(e)))
        }
        _ => a.clone(),
    }
}

/// Convert the exponent of a fixed-point lead Ω^e: a top-level ψ is fully
/// evaluated (rows 196-203); anything else is kept symbolically, with a
/// trailing bare Ω absorbed to 1 behind a fixed-point head (row 323).
fn conv_exp(e: &Ast) -> C {
    if let Ast::Psi(None, arg) = e {
        if !matches!(arg.as_ref(), Ast::Num(0)) {
            return conv_psi0(arg);
        }
    }
    conv_sym(&exp_shift(e))
}

/// Translate an Ω^e tail block of a fixed-point collapse.
fn translate_fixed_tail(ee: &Ast, f: &Ast) -> Ast {
    if let Some(n) = as_nat(ee) {
        return if n <= 1 {
            Ast::Num(1)
        } else {
            Ast::Pow(Box::new(Ast::Omega(None)), Box::new(Ast::Num(n - 1)))
        };
    }
    Ast::Pow(Box::new(Ast::Omega(None)), Box::new(translate_deep(ee, f)))
}

/// ψ₀(Ω^λ·k + r) with λ a limit: the argument is kept symbolically; tails
/// Ω^e shift down one level and Ω^{ψ(F)} collapses to ψ(F).
fn collapse_fixed_omegapow(e: &Ast, mult: &Ast, tail: &Ast) -> C {
    let f_ast = Ast::Pow(Box::new(Ast::Omega(None)), Box::new(e.clone()));
    let lead0 = C::Pow(Box::new(C::Omega), Box::new(conv_exp(e)));
    let lead = c_mul(lead0, conv_sym(mult));
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_parts: Vec<Ast> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    for b in &blocks {
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            continue;
        }
        let (h, m) = split_head_mult(b);
        let m_one = matches!(m, Ast::Num(1));
        match h {
            Some(Head::OmegaPow(ee)) => {
                let is_psi_f = matches!(ee, Ast::Psi(None, ref inner) if ast_eq(inner.as_ref(), &f_ast));
                if is_psi_f {
                    x_parts.push(b.clone());
                } else {
                    let tr = translate_fixed_tail(&ee, &f_ast);
                    if m_one {
                        x_parts.push(tr);
                    } else {
                        x_parts.push(Ast::Mul(Box::new(tr), Box::new(m)));
                    }
                }
            }
            _ => x_parts.push(b.clone()),
        }
    }
    let w = sum_of(&w_parts);
    let x_c = if x_parts.is_empty() { C::Zero } else { conv_sym(&sum_of(&x_parts)) };
    let arg = if is_c_zero(&x_c) { lead } else { c_sum(vec![lead, x_c]) };
    let psi = C::Psi(None, Box::new(arg));
    if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) }
}

fn is_zero_ast(a: &Ast) -> bool {
    matches!(a, Ast::Num(0))
}

/// True if the ordinal (in normal form) is a successor, i.e. it ends in a
/// positive finite number (α = β + 1).  Used to distinguish Ω subscripts
/// that collapse (successor: Ω_2, Ω_{Ω+1}, ...) from those that are fixed
/// points (limit: Ω_ω, Ω_{ω·2}, Ω_Ω, ...).
fn is_successor_ord(n: &Ast) -> bool {
    match n {
        Ast::Num(k) => *k >= 1,
        Ast::Add(_, r) => is_successor_ord(r),
        _ => false,
    }
}

/// Predecessor of a successor ordinal (call only when is_successor_ord is true).
fn pred_ord(n: &Ast) -> Ast {
    match n {
        Ast::Num(k) => Ast::Num(k - 1),
        Ast::Add(l, r) => {
            let rp = pred_ord(r);
            if matches!(&rp, Ast::Num(0)) {
                (**l).clone()
            } else {
                Ast::Add(l.clone(), Box::new(rp))
            }
        }
        _ => n.clone(),
    }
}

// ════════════════════════════════════════════════════════════════
// Normalization: ω^{ψ(a)·c + rest} → ψ(a)^c · ω^{rest}
// ════════════════════════════════════════════════════════════════

fn flatten_c_sum(c: &C) -> Vec<C> {
    match c {
        C::Sum(terms) => {
            let mut v = Vec::new();
            for t in terms { v.extend(flatten_c_sum(t)); }
            v
        }
        other => vec![other.clone()],
    }
}

fn is_finite_c(c: &C) -> bool { matches!(c, C::Nat(_) | C::One | C::Zero) }
fn c_nat_val(c: &C) -> i32 {
    match c { C::Nat(n) => *n, C::One => 1, _ => 0 }
}

/// Ordinal addition of exponents (normal-form absorption):
/// finite + limit = limit; both finite sums as a natural; otherwise append.
fn c_add_ord(a: C, b: C) -> C {
    if is_c_zero(&a) { return b; }
    if is_c_zero(&b) { return a; }
    let a_fin = is_finite_c(&a);
    let b_fin = is_finite_c(&b);
    if a_fin && b_fin { return c_nat(c_nat_val(&a) + c_nat_val(&b)); }
    if a_fin && !b_fin { return b; }
    c_sum(vec![a, b])
}

/// Flatten a product into its factors, merging consecutive identical
/// ψ-powers with ordinal-exponent addition (ψ(a)^B · ψ(a)^C → ψ(a)^{B+C}).
fn merge_product(a: C, b: C) -> C {
    let mut factors: Vec<C> = Vec::new();
    fn push_factor(f: C, out: &mut Vec<C>) {
        match f {
            C::Mul(x, y) => { push_factor(*x, out); push_factor(*y, out); }
            other => out.push(other),
        }
    }
    push_factor(a, &mut factors);
    push_factor(b, &mut factors);

    enum E {
        PsiP(Box<C>, C),
        Other(C),
    }
    let mut entries: Vec<E> = Vec::new();
    for f in factors {
        let e = match &f {
            C::Psi(..) => E::PsiP(Box::new(f.clone()), c_nat(1)),
            C::Pow(bx, e) if matches!(bx.as_ref(), C::Psi(..)) => E::PsiP(bx.clone(), (**e).clone()),
            _ => E::Other(f),
        };
        if let E::PsiP(bb, xx) = &e {
            if let Some(E::PsiP(lb, lx)) = entries.last_mut() {
                if render(lb) == render(bb) {
                    let newv = c_add_ord((*lx).clone(), xx.clone());
                    *lx = newv;
                    continue;
                }
            }
        }
        entries.push(e);
    }

    let mut acc = C::One;
    for e in entries {
        let f = match e {
            E::PsiP(base, ex) if matches!(&ex, C::One) || matches!(&ex, C::Nat(1)) => *base,
            E::PsiP(base, ex) => C::Pow(base, Box::new(ex)),
            E::Other(c) => c,
        };
        acc = c_mul(acc, f);
    }
    acc
}

/// Split a summand into (Ω-power base part, coefficient) for ψ_v-based
/// blocks (Ω^e · d written as ψ_v(0)^e·d).  Returns None for non-mergeable
/// terms (ω-powers, ψ_0 collapses, bare naturals).
fn split_block_coeff(t: &C) -> Option<(C, C)> {
    let is_psi_base = |x: &C| matches!(x, C::Psi(Some(_), _));
    // ψ_v^e (e ≥ 2) as base ψ_v with coefficient ψ_v^{e-1}: ψ^e = ψ·ψ^{e-1}.
    let pow_split = |p: &Box<C>, e: &Box<C>| -> Option<(C, C)> {
        if !is_psi_base(p) {
            return None;
        }
        let em1 = match e.as_ref() {
            C::Nat(n) if *n >= 2 => C::Nat(n - 1),
            C::Nat(_) => return None,
            other => other.clone(),
        };
        let coeff = if matches!(&em1, C::One) || matches!(&em1, C::Nat(1)) {
            (**p).clone()
        } else {
            C::Pow(p.clone(), Box::new(em1))
        };
        Some(((**p).clone(), coeff))
    };
    match t {
        C::Psi(Some(_), _) => Some((t.clone(), C::One)),
        C::Mul(a, b) => {
            match a.as_ref() {
                x if is_psi_base(x) => Some(((**a).clone(), (**b).clone())),
                C::Pow(p, e) => pow_split(p, e)
                    .map(|(base, coeff)| (base, c_mul(coeff, (**b).clone()))),
                _ => None,
            }
        }
        C::Pow(p, e) => pow_split(p, e),
        _ => None,
    }
}

fn rebuild_coeff(base: C, coeff: C) -> C {
    if matches!(coeff, C::One) { base } else { C::Mul(Box::new(base), Box::new(coeff)) }
}

/// Merge consecutive same-base ψ_v(0)-blocks in a sum by ordinal coefficient
/// addition (Ω^e·c1 + Ω^e·c2 = Ω^e·(c1+c2)), writing the result in base-Ω.
fn merge_arg_blocks(c: C) -> C {
    match c {
        C::Sum(terms) => {
            let mut out: Vec<C> = Vec::new();
            for t in terms {
                let mut merged = false;
                if let Some((base, coeff)) = split_block_coeff(&t) {
                    if let Some(last) = out.last_mut() {
                        if let Some((lbase, lcoeff)) = split_block_coeff(last) {
                            if render(&lbase) == render(&base) {
                                *last = rebuild_coeff(lbase, c_add_ord(lcoeff, coeff));
                                merged = true;
                            }
                        }
                    }
                }
                if !merged { out.push(t); }
            }
            c_sum(out.into_iter().map(normalize).collect())
        }
        other => other,
    }
}

fn normalize(c: C) -> C {
    match c {
        C::Sum(terms) => merge_arg_blocks(c_sum(terms.into_iter().map(normalize).collect())),
        C::Mul(a, b) => {
            let na = normalize(*a);
            let nb = normalize(*b);
            merge_product(na, nb)
        }
        C::OmegaSub(a) => {
            let na = normalize(*a);
            if let C::Nat(1) = &na { C::Omega } else { C::OmegaSub(Box::new(na)) }
        }
        C::OmegaPow(a) => normalize_omegapow(*a),
        C::Pow(b, e) => {
            let nb = normalize(*b);
            let ne = normalize(*e);
            if matches!(&ne, C::Nat(1)) || matches!(&ne, C::One) {
                nb
            } else {
                C::Pow(Box::new(nb), Box::new(ne))
            }
        }
        C::Psi(v, a) => C::Psi(
            v.map(|x| Box::new(normalize(*x))),
            Box::new(normalize(*a)),
        ),
        other => other,
    }
}

fn flatten_product(c: &C, out: &mut Vec<C>) {
    match c {
        C::Mul(a, b) => { flatten_product(a, out); flatten_product(b, out); }
        other => out.push(other.clone()),
    }
}

fn product_of(factors: &[C]) -> C {
    let mut acc = C::One;
    for f in factors { acc = c_mul(acc, f.clone()); }
    acc
}

/// Extract the leading ψ-factor of an (additive/product) block, returning
/// (base, cof) so that the block = ψ(base)·cof.  Applies
/// ω^{ψ(a)^c} = ψ(a)^{ψ(a)^{c-1}} (c finite) or ψ(a)^{ψ(a)^c} (c ≥ ω),
/// and ω^{ψ(a)·B} = ψ(a)^B.
fn extract_psi_factor(lead: &C) -> Option<(C, C)> {
    let mut factors: Vec<C> = Vec::new();
    flatten_product(lead, &mut factors);
    let idx = factors.iter().position(|f| {
        matches!(f, C::Psi(..)) || matches!(f, C::Pow(b, _) if matches!(b.as_ref(), C::Psi(..)))
    })?;
    let f = factors.remove(idx);
    let cof_rest = product_of(&factors);
    match f {
        C::Psi(..) => Some((f, cof_rest)),
        C::Pow(b, c) => {
            let base = (*b).clone();
            let cof_c = match c.as_ref() {
                C::Nat(n) if *n >= 2 => {
                    if *n == 2 { base.clone() } else { C::Pow(b.clone(), Box::new(c_nat(n - 1))) }
                }
                C::Nat(_) => C::One, // ψ(a)^1 = ψ(a)
                _ => C::Pow(b.clone(), c.clone()), // c ≥ ω
            };
            Some((base, c_mul(cof_c, cof_rest)))
        }
        _ => unreachable!(),
    }
}

fn normalize_omegapow(exp: C) -> C {
    let e = normalize(exp);
    let parts = flatten_c_sum(&e);
    if parts.is_empty() { return C::OmegaPow(Box::new(C::Zero)); }
    let lead = &parts[0];
    let rest: Vec<C> = parts[1..].to_vec();

    // ψ(α) is a fixed point of ω^, so ω^{ψ(a)·cof + rest} = ψ(a)^cof · ω^{rest}.
    let factored = extract_psi_factor(lead);
    if let Some((base, cof)) = factored {
        let psi_pow = if matches!(&cof, C::One) || matches!(&cof, C::Nat(1)) {
            base
        } else {
            C::Pow(Box::new(base), Box::new(cof))
        };
        let rest_sum = c_sum(rest);
        // ω^{α+β} = ω^α × ω^β
        if is_c_zero(&rest_sum) {
            normalize(psi_pow)
        } else {
            normalize(c_mul(psi_pow, C::OmegaPow(Box::new(rest_sum))))
        }
    } else {
        C::OmegaPow(Box::new(e))
    }
}

// ════════════════════════════════════════════════════════════════
// Structural normalization (semantic, reusable outside rendering)
// ════════════════════════════════════════════════════════════════

/// Structural normalization of a MOCF value: rewrites the expression
/// using ordinal identities, not just display conventions.  Core rule:
/// ω^{ψ_a(b)} = ψ_a(b) for ANY a and b (every ψ-value is a fixed point
/// of ω^), plus its compositional closure:
///   - ω^{ψ_a(b)·k} = ψ_a(b)^k and ω^{ψ_a(b)^j} = ψ_a(b)^{ψ_a(b)^{j-1}}
///     factored out of ω-exponents at any position (ω^{X+Y} = ω^X·ω^Y);
///   - ω^u·ω^v = ω^{u+v} in products;
///   - ω^e·ψ_a(b) = ψ_a(b) and e + ψ_a(b)·rest = ψ_a(b)·rest whenever
///     e < ψ_a(b) (conservatively judged), in products and sums;
///   - adjacent same-base ψ-powers merge;
///   - left absorption: ψ_a(x) + ψ_a(y) = ψ_a(y) when x < y are natural.
pub fn mocf_normalize(c: &C) -> C {
    let mut cur = mocf_normalize_once(c);
    for _ in 0..16 {
        let next = mocf_normalize_once(&cur);
        if render(&next) == render(&cur) {
            break;
        }
        cur = next;
    }
    cur
}

fn mocf_normalize_once(c: &C) -> C {
    match c {
        C::OmegaPow(e) => {
            let en = mocf_normalize_once(e);
            let fs = factor_omega_pow(&en);
            if fs.len() == 1 {
                fs.into_iter().next().unwrap()
            } else {
                normalize_product(fs)
            }
        }
        C::Mul(a, b) => {
            let na = mocf_normalize_once(a);
            let nb = mocf_normalize_once(b);
            let mut fs = Vec::new();
            flatten_product(&na, &mut fs);
            flatten_product(&nb, &mut fs);
            normalize_product(fs)
        }
        C::Sum(terms) => {
            let mut out: Vec<C> = Vec::new();
            for t in terms {
                let nt = mocf_normalize_once(t);
                absorb_small_before(&mut out, &nt);
                out.push(nt);
            }
            c_sum(out)
        }
        C::Psi(v, a) => C::Psi(
            v.as_ref().map(|x| Box::new(mocf_normalize_once(x))),
            Box::new(mocf_normalize_once(a)),
        ),
        C::Pow(b, e) => {
            let nb = mocf_normalize_once(b);
            let ne = mocf_normalize_once(e);
            if matches!(&ne, C::One) || matches!(&ne, C::Nat(1)) {
                nb
            } else {
                C::Pow(Box::new(nb), Box::new(ne))
            }
        }
        C::OmegaSub(a) => C::OmegaSub(Box::new(mocf_normalize_once(a))),
        other => other.clone(),
    }
}

enum PsiCoef {
    One,
    Exp(C),
}

/// Decompose an exponent block b so that ω^b is a power of a ψ-value:
/// ψ_a(b) itself, ψ_a(b)·k, or ψ_a(b)^j.
fn as_psi_fixed_block(b: &C) -> Option<(C, PsiCoef)> {
    match b {
        C::Psi(..) => Some((b.clone(), PsiCoef::One)),
        C::Mul(a, k) if matches!(a.as_ref(), C::Psi(..)) => {
            Some(((**a).clone(), PsiCoef::Exp((**k).clone())))
        }
        C::Pow(p, j) if matches!(p.as_ref(), C::Psi(..)) => match j.as_ref() {
            C::Nat(n) if *n >= 2 => Some((
                (**p).clone(),
                PsiCoef::Exp(C::Pow(p.clone(), Box::new(c_nat(n - 1)))),
            )),
            C::Nat(_) => Some(((**p).clone(), PsiCoef::One)),
            other => Some((
                (**p).clone(),
                PsiCoef::Exp(C::Pow(p.clone(), Box::new(other.clone()))),
            )),
        },
        _ => None,
    }
}

/// Factor ω^{Σ blocks} into a product, pulling every ψ-value block out of
/// the exponent via ω^{ψ_a(b)·k} = ψ_a(b)^k.
fn factor_omega_pow(e: &C) -> Vec<C> {
    let mut factors: Vec<C> = Vec::new();
    let mut plain: Vec<C> = Vec::new();
    for b in flatten_c_sum(e) {
        if let Some((base, coef)) = as_psi_fixed_block(&b) {
            if !plain.is_empty() {
                factors.push(C::OmegaPow(Box::new(c_sum(std::mem::take(&mut plain)))));
            }
            factors.push(match coef {
                PsiCoef::One => base,
                PsiCoef::Exp(k) => C::Pow(Box::new(base), Box::new(k)),
            });
        } else {
            plain.push(b);
        }
    }
    if !plain.is_empty() {
        factors.push(C::OmegaPow(Box::new(c_sum(plain))));
    }
    factors
}

/// The subscript of the leading ψ-value of a factor, if the factor is a
/// ψ-power block: None subscript means MOCF ψ, Some(v) means ψ_v.
fn lead_psi_sub(f: &C) -> Option<Option<C>> {
    match f {
        C::Psi(v, _) => Some(v.as_ref().map(|x| (**x).clone())),
        C::Pow(b, _) => lead_psi_sub(b),
        C::Mul(a, _) => lead_psi_sub(a),
        _ => None,
    }
}

fn contains_psi(c: &C) -> bool {
    match c {
        C::Psi(..) => true,
        C::OmegaSub(a) | C::OmegaPow(a) => contains_psi(a),
        C::Pow(a, b) => contains_psi(a) || contains_psi(b),
        C::Mul(a, b) => contains_psi(a) || contains_psi(b),
        C::Sum(ts) => ts.iter().any(contains_psi),
        _ => false,
    }
}

/// Conservative check for e < ψ_sub(·), which makes ω^e absorbable into
/// the ψ-value: ψ_v (v ≥ 1) is ≥ Ω, hence above every countable ω^e;
/// MOCF ψ-values are ≥ ε_0, hence above every ψ-free ω^e.
fn exp_absorbed_by_psi(e: &C, sub: Option<&C>) -> bool {
    match sub {
        Some(_) => is_below_c(e),
        None => !contains_psi(e),
    }
}

/// Conservative ψ_a(x) < ψ_a(y): same subscript, both arguments natural
/// with x < y (ψ is strictly increasing on standard arguments).
fn psi_block_lt(a: &C, b: &C) -> bool {
    match (a, b) {
        (C::Psi(va, xa), C::Psi(vb, xb)) => match (va, vb) {
            (None, None) => nat_lt(xa, xb),
            (Some(x), Some(y)) => render(x) == render(y) && nat_lt(xa, xb),
            _ => false,
        },
        _ => false,
    }
}

fn nat_lt(a: &C, b: &C) -> bool {
    fn val(c: &C) -> Option<i32> {
        match c {
            C::Zero => Some(0),
            C::One => Some(1),
            C::Nat(n) => Some(*n),
            _ => None,
        }
    }
    matches!((val(a), val(b)), (Some(x), Some(y)) if x < y)
}

/// Drop trailing terms absorbed by the upcoming ψ-block t:
/// ω^e (or a finite n) with ω^e < t, and bare ψ-blocks proved smaller.
fn absorb_small_before(out: &mut Vec<C>, t: &C) {
    let sub = match lead_psi_sub(t) {
        Some(s) => s,
        None => return,
    };
    while let Some(last) = out.last() {
        let drop_it = match last {
            C::OmegaPow(e) => exp_absorbed_by_psi(e, sub.as_ref()),
            C::Nat(_) | C::One => true,
            C::Psi(..) => psi_block_lt(last, t),
            _ => false,
        };
        if drop_it {
            out.pop();
        } else {
            break;
        }
    }
}

/// Canonicalize product factors: merge adjacent ω-powers (ω^a·ω^b =
/// ω^{a+b}), absorb small factors into following ψ-blocks, and merge
/// adjacent same-base ψ-powers.
fn normalize_product(factors: Vec<C>) -> C {
    let mut merged: Vec<C> = Vec::new();
    for f in factors {
        if let (Some(C::OmegaPow(u)), C::OmegaPow(v)) = (merged.last(), &f) {
            let sum = c_add_ord((**u).clone(), (**v).clone());
            *merged.last_mut().unwrap() = C::OmegaPow(Box::new(sum));
        } else {
            merged.push(f);
        }
    }
    let mut abs: Vec<C> = Vec::new();
    for f in merged {
        absorb_small_before(&mut abs, &f);
        abs.push(f);
    }
    let mut acc = C::One;
    for f in abs {
        acc = merge_product(acc, f);
    }
    acc
}

// ════════════════════════════════════════════════════════════════
// Rendering
// ════════════════════════════════════════════════════════════════

fn render(c: &C) -> String {
    match c {
        C::Zero => "0".into(),
        C::One => "1".into(),
        C::Nat(n) => n.to_string(),
        C::Omega => "\\Omega".into(),
        C::OmegaSub(a) => {
            let s = render(a);
            if s == "1" { "\\Omega".into() } else { format!("\\Omega_{{{}}}", s) }
        }
        C::OmegaPow(a) => {
            match a.as_ref() {
                C::Zero => "1".into(),
                C::One => "\\omega".into(),
                C::Nat(1) => "\\omega".into(),
                _ => format!("\\omega^{{{}}}", render(a)),
            }
        }
        C::Psi(v, a) => match v {
            None => format!("\\psi({})", render(a)),
            Some(v) => format!("\\psi_{{{}}}({})", render(v), render(a)),
        },
        C::Pow(b, e) => {
            let bs = render(b);
            match e.as_ref() {
                C::One => bs,
                C::Nat(n) => format!("{}^{{{}}}", bs, n),
                _ => format!("{}^{{{}}}", bs, render(e)),
            }
        }
        C::Mul(a, b) => format!("{}{}", render(a), render(b)),
        C::Sum(terms) => {
            let mut s = String::new();
            for (i, t) in terms.iter().enumerate() {
                if i > 0 { s.push_str(" + "); }
                s.push_str(&render(t));
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(s: &str) -> String { bocf_to_mocf(s).unwrap() }

    #[test]
    fn standardness_of_nested_omega_tower() {
        for s in ["ψ(Ω^Ω^Ω)", "ψ(Ω^{Ω^ω})", "Ω^Ω^Ω", "ψ(Ω^Ω)"] {
            let ast = crate::parser::parse_bocf(s).unwrap();
            let raw = crate::parser::eval_raw_ast(&ast).unwrap();
            assert_eq!(
                crate::term::check_bocf_standardness(&raw),
                crate::term::BocfStandardness::Standard,
                "raw eval of {} judged non-standard",
                s
            );
            let sf = crate::term::standard_form(&raw);
            assert_eq!(
                crate::term::check_bocf_standardness(&sf),
                crate::term::BocfStandardness::Standard,
                "input {} judged non-standard",
                s
            );
        }
    }

    #[test]
    fn standardness_of_ascending_sums() {
        let check = |s: &str| {
            let ast = crate::parser::parse_bocf(s).unwrap();
            crate::parser::bocf_input_standardness(&ast)
        };
        use crate::term::BocfStandardness::*;
        assert_eq!(check("1+ω"), NonStandardButNormalizable, "1+ω judged standard");
        assert_eq!(check("1+Ω"), NonStandardButNormalizable, "1+Ω judged standard");
        assert_eq!(check("ψ(0)+ω"), NonStandardButNormalizable, "ψ(0)+ω judged standard");
        assert_eq!(
            check("ψ(Ω^(1+Ω))"),
            NonStandardButNormalizable,
            "ψ(Ω^(1+Ω)) judged standard"
        );
        assert_eq!(
            check("ψ(Ω^(1+ω))"),
            NonStandardButNormalizable,
            "ψ(Ω^(1+ω)) judged standard"
        );
        assert_eq!(check("ω+1"), Standard, "ω+1 judged non-standard");
        assert_eq!(check("Ω+ω"), Standard, "Ω+ω judged non-standard");
        assert_eq!(check("ψ(Ω^(Ω+1))"), Standard, "ψ(Ω^(Ω+1)) judged non-standard");
        assert_eq!(check("ψ(Ω^Ω^Ω)"), Standard, "ψ(Ω^Ω^Ω) judged non-standard");
    }


    #[test]
    fn structural_normalize() {
        let n = |c: C| render(&mocf_normalize(&c));
        let psi0 = |a: C| C::Psi(None, Box::new(a));
        let psiv = |v: C, a: C| C::Psi(Some(Box::new(v)), Box::new(a));
        let omegapow = |e: C| C::OmegaPow(Box::new(e));

        // Rule 1: ω^{ψ_a(b)} = ψ_a(b), regardless of a and b.
        assert_eq!(n(omegapow(psi0(C::Zero))), "\\psi(0)");
        assert_eq!(n(omegapow(psi0(C::Nat(3)))), "\\psi(3)");
        assert_eq!(
            n(omegapow(psiv(C::Nat(2), C::Zero))),
            "\\psi_{2}(0)"
        );
        assert_eq!(
            n(omegapow(psiv(C::OmegaPow(Box::new(C::One)), C::Omega))),
            "\\psi_{\\omega}(\\Omega)"
        );

        // ψ-coefficients and ψ-powers in ω-exponents.
        assert_eq!(
            n(omegapow(C::Mul(Box::new(psi0(C::Zero)), Box::new(C::Nat(2))))),
            "\\psi(0)^{2}"
        );
        assert_eq!(
            n(omegapow(C::Pow(Box::new(psi0(C::Zero)), Box::new(C::Nat(2))))),
            "\\psi(0)^{\\psi(0)}"
        );
        // ω^{ω+ψ(0)} = ω^ω·ψ(0) = ψ(0) (left absorption).
        assert_eq!(
            n(omegapow(c_sum(vec![
                omegapow(C::One),
                psi0(C::Zero),
            ]))),
            "\\psi(0)"
        );
        // ω^{ψ(0)·2+ω} = ψ(0)^2·ω^ω (no absorption to the right).
        assert_eq!(
            n(omegapow(c_sum(vec![
                C::Mul(Box::new(psi0(C::Zero)), Box::new(C::Nat(2))),
                omegapow(C::One),
            ]))),
            "\\psi(0)^{2}\\omega^{\\omega}"
        );

        // ω^u·ω^v = ω^{u+v}; ψ-powers merge.
        assert_eq!(
            n(C::Mul(
                Box::new(omegapow(psi0(C::Zero))),
                Box::new(omegapow(psi0(C::Zero))),
            )),
            "\\psi(0)^{2}"
        );

        // Absorption in products: ω^ω·ψ_1(0) = ψ_1(0), 5 + ψ(0) = ψ(0).
        assert_eq!(
            n(C::Mul(
                Box::new(omegapow(C::One)),
                Box::new(psiv(C::Nat(1), C::Zero)),
            )),
            "\\psi_{1}(0)"
        );
        assert_eq!(
            n(c_sum(vec![C::Nat(5), psi0(C::Zero)])),
            "\\psi(0)"
        );

        // Counter-cases: ω^{ψ(0)}·ψ(0) = ψ(0)^2 (equal exponent, no absorption).
        assert_eq!(
            n(C::Mul(
                Box::new(omegapow(psi0(C::Zero))),
                Box::new(psi0(C::Zero)),
            )),
            "\\psi(0)^{2}"
        );
        // ω^{ψ(0)} + ψ(1) = ψ(1) (ψ(1) > ψ(0) absorbs the left term).
        assert_eq!(
            n(c_sum(vec![omegapow(psi0(C::Zero)), psi0(C::Nat(1))])),
            "\\psi(1)"
        );
        // Ω + ψ(1) stays: Ω = ψ_1(0) is not below ψ(1).
        assert_eq!(
            n(c_sum(vec![C::Omega, psi0(C::Nat(1))])),
            "\\Omega + \\psi(1)"
        );
    }

    #[test]
    fn psi_omega_region() {
        assert_eq!(conv("ψ(Ω)"), "\\psi(0)");
        assert_eq!(conv("ψ(Ω+1)"), "\\psi(0)\\omega");
        assert_eq!(conv("ψ(Ω+ψ(Ω))"), "\\psi(0)^{2}");
        assert_eq!(conv("ψ(Ω×2)"), "\\psi(1)");
        assert_eq!(conv("ψ(Ω×3)"), "\\psi(2)");
        assert_eq!(conv("ψ(Ω×ψ(1))"), "\\psi(\\omega)");
        assert_eq!(conv("ψ(Ω×ψ(Ω))"), "\\psi(\\psi(0))");
        assert_eq!(conv("ψ(Ω^2)"), "\\psi(\\Omega)");
        assert_eq!(conv("ψ(Ω^2×2)"), "\\psi(\\Omega2)");
        assert_eq!(conv("ψ(Ω^3)"), "\\psi(\\Omega^{2})");
        assert_eq!(conv("ψ(Ω^3+Ω^2)"), "\\psi(\\Omega^{2} + \\Omega)");
        assert_eq!(conv("ψ(Ω^2+Ω)"), "\\psi(\\Omega + 1)");
        assert_eq!(conv("ψ(Ω^2+1)"), "\\psi(\\Omega)\\omega");
        assert_eq!(conv("ψ(Ω^2+ψ(Ω))"), "\\psi(\\Omega)\\psi(0)");
        assert_eq!(conv("ψ(Ω^2+ψ(Ω^2))"), "\\psi(\\Omega)^{2}");
    }

    #[test]
    fn psi_omega_power_multiplier() {
        assert_eq!(conv("ψ(Ω×ψ(1))"), "\\psi(\\omega)");
        assert_eq!(conv("ψ(Ω×ψ(Ω×2))"), "\\psi(\\psi(1))");
    }

    #[test]
    fn fixed_region() {
        assert_eq!(conv("ψ(Ω^ω)"), "\\psi(\\Omega^{\\omega})");
        assert_eq!(conv("ψ(Ω^ω+Ω)"), "\\psi(\\Omega^{\\omega} + 1)");
        assert_eq!(conv("ψ(Ω^ω+Ω^2)"), "\\psi(\\Omega^{\\omega} + \\Omega)");
        assert_eq!(conv("ψ(Ω^ω×2)"), "\\psi(\\Omega^{\\omega}2)");
        assert_eq!(conv("ψ(Ω^Ω)"), "\\psi(\\Omega^{\\Omega})");
        assert_eq!(conv("ψ(Ω^{ψ(Ω)})"), "\\psi(\\Omega^{\\psi(0)})");
    }

    #[test]
    fn level_shift_region() {
        assert_eq!(conv("ψ(Ω_2)"), "\\psi(\\psi_{1}(0))");
        assert_eq!(conv("ψ(Ω_2×2)"), "\\psi(\\psi_{1}(1))");
        assert_eq!(conv("ψ(Ω_2×3)"), "\\psi(\\psi_{1}(2))");
        assert_eq!(conv("ψ(Ω_3)"), "\\psi(\\psi_{2}(0))");
        assert_eq!(conv("ψ(Ω_3×2)"), "\\psi(\\psi_{2}(1))");
        assert_eq!(conv("ψ(Ω_2^2)"), "\\psi(\\Omega_{2})");
        // successor vs limit subscript
        assert_eq!(conv("ψ(Ω_{Ω+1})"), "\\psi(\\psi_{\\Omega}(0))");
        assert_eq!(conv("ψ(Ω_{ω+1})"), "\\psi(\\psi_{\\omega}(0))");
        assert_eq!(conv("ψ(Ω_Ω)"), "\\psi(\\Omega_{\\Omega})");
        assert_eq!(conv("ψ(Ω_{Ω+ω})"), "\\psi(\\Omega_{\\Omega + \\omega})");
        // higher successor cardinals collapse inside ψ_v (row 472, 516)
        assert_eq!(conv("ψ(Ω_3+ψ_1(Ω_3))"), "\\psi(\\psi_{2}(0) + \\psi_{1}(\\psi_{2}(0)))");
        assert_eq!(conv("ψ(Ω_ω+ψ_1(Ω_3))"), "\\psi(\\Omega_{\\omega} + \\psi_{1}(\\psi_{2}(0)))");
    }

    #[test]
    fn omega_omega_fixed() {
        assert_eq!(conv("ψ(Ω_ω)"), "\\psi(\\Omega_{\\omega})");
        assert_eq!(conv("ψ(Ω_ω+Ω)"), "\\psi(\\Omega_{\\omega} + 1)");
        assert_eq!(conv("ψ(Ω_ω+Ω^2)"), "\\psi(\\Omega_{\\omega} + \\Omega)");
        assert_eq!(conv("ψ(Ω_ω+1)"), "\\psi(\\Omega_{\\omega})\\omega");
    }

    #[test]
    fn more_csv_rows() {
        // Row 54: ψ(Ω·2+ψ(Ω)) → ψ(1)·ψ(0)
        assert_eq!(conv("ψ(Ω×2+ψ(Ω))"), "\\psi(1)\\psi(0)");
        // Row 56: ψ(Ω·2+ψ(Ω·2)·2) → ψ(1)^3
        assert_eq!(conv("ψ(Ω×2+ψ(Ω×2)×2)"), "\\psi(1)^{3}");
        // Row 90: ψ(Ω²+ψ(Ω)) → ψ(Ω)+ψ(0)
        assert_eq!(conv("ψ(Ω^2+ψ(Ω))"), "\\psi(\\Omega)\\psi(0)");
        // Row 96: ψ(Ω²+ψ(Ω²)) → ψ(Ω)^2
        assert_eq!(conv("ψ(Ω^2+ψ(Ω^2))"), "\\psi(\\Omega)^{2}");
        // Row 185: ψ(Ω^ω·2) → ψ(Ω^ω·2)
        assert_eq!(conv("ψ(Ω^ω×2)"), "\\psi(\\Omega^{\\omega}2)");
        // Row 196: ψ(Ω^{ψ(2)}) → ψ(Ω^{ω²})
        assert_eq!(conv("ψ(Ω^{ψ(2)})"), "\\psi(\\Omega^{\\omega^{2}})");
        // Row 515: ψ(Ω_ω+ψ_1(Ω_2)) → ψ(Ω_ω+ψ_1(0))
        assert_eq!(conv("ψ(Ω_ω+ψ_1(Ω_2))"), "\\psi(\\Omega_{\\omega} + \\psi_{1}(0))");
        // Row 181: ψ(Ω^ω+Ω^3) → ψ(Ω^ω+Ω²)
        assert_eq!(conv("ψ(Ω^ω+Ω^3)"), "\\psi(\\Omega^{\\omega} + \\Omega^{2})");
        // Row 101: ψ(Ω²+Ω) → ψ(Ω+1)
        assert_eq!(conv("ψ(Ω^2+Ω)"), "\\psi(\\Omega + 1)");
    }

    #[test]
    fn omega_omega_region() {
        // Row 198: ψ(Ω^{ψ(Ω)}) → ψ(Ω^{ψ(0)})
        assert_eq!(conv("ψ(Ω^{ψ(Ω)})"), "\\psi(\\Omega^{\\psi(0)})");
        // Row 209 (corrected): tail Ω·ψ(Ω^Ω) becomes ψ(Ω^Ω)
        assert_eq!(conv("ψ(Ω^Ω+Ω×ψ(Ω^Ω))"), "\\psi(\\Omega^{\\Omega} + \\psi(\\Omega^{\\Omega}))");
        // Row 218: tail Ω^{ψ(Ω^Ω)}·1 stays
        assert_eq!(conv("ψ(Ω^Ω+Ω^{ψ(Ω^Ω)})"), "\\psi(\\Omega^{\\Omega} + \\Omega^{\\psi(\\Omega^{\\Omega})})");
        // Pure Term: ψ(0) evaluates to 1, so Ω^{ψ(0)} = Ω
        assert_eq!(conv("ψ(Ω^Ω+Ω^{ψ(0)})"), "\\psi(\\Omega^{\\Omega} + 1)");
        // Row 233: Ω^Ω·ψ(0) = Ω^Ω
        assert_eq!(conv("ψ(Ω^Ω×ψ(0))"), "\\psi(\\Omega^{\\Omega})");
        // Row 278: Ω^{Ω·ψ(0)} = Ω^Ω
        assert_eq!(conv("ψ(Ω^{Ω×ψ(0)})"), "\\psi(\\Omega^{\\Omega})");
        // Row 323: trailing Ω behind a fixed head is absorbed
        assert_eq!(conv("ψ(Ω^{Ω^ω+Ω})"), "\\psi(\\Omega^{\\Omega^{\\omega} + 1})");
    }

    #[test]
    fn psi1_e_machinery() {
        // Row 412
        assert_eq!(conv("ψ(Ω_2+ψ_1(Ω_2+1))"), "\\psi(\\psi_{1}(0)\\omega)");
        // Row 416
        assert_eq!(conv("ψ(Ω_2+ψ_1(Ω_2+ψ_1(Ω_2)))"), "\\psi(\\psi_{1}(0)^{2})");
        // Row 417
        assert_eq!(conv("ψ(Ω_2+ψ_1(Ω_2+ψ_1(Ω_2))×2)"), "\\psi(\\psi_{1}(0)^{2}2)");
        // Row 419
        assert_eq!(conv("ψ(Ω_2+ψ_1(Ω_2+ψ_1(Ω_2)×2))"), "\\psi(\\psi_{1}(0)^{3})");
        // Row 420
        assert_eq!(conv("ψ(Ω_2+ψ_1(Ω_2+ψ_1(Ω_2+1)))"), "\\psi(\\psi_{1}(0)^{\\omega})");
        // Row 425: different bases do not merge
        assert_eq!(conv("ψ(Ω_2×2+ψ_1(Ω_2))"), "\\psi(\\psi_{1}(1) + \\psi_{1}(0))");
        // Row 427
        assert_eq!(conv("ψ(Ω_2×2+ψ_1(Ω_2×2+ψ_1(Ω_2×2)))"), "\\psi(\\psi_{1}(1)^{2})");
    }

    #[test]
    fn omega2_squared_region() {
        // Row 435
        assert_eq!(conv("ψ(Ω_2^2+ψ_1(Ω_2^2))"), "\\psi(\\Omega_{2} + \\psi_{1}(\\Omega_{2}))");
        // Row 436
        assert_eq!(conv("ψ(Ω_2^2+Ω_2)"), "\\psi(\\Omega_{2} + \\psi_{1}(\\Omega_{2} + 1))");
        // Row 437
        assert_eq!(conv("ψ(Ω_2^2+Ω_2×ψ_1(Ω_2^2))"), "\\psi(\\Omega_{2} + \\psi_{1}(\\Omega_{2} + \\psi_{1}(\\Omega_{2})))");
        // Row 440
        assert_eq!(conv("ψ(Ω_2^2×ψ_1(Ω_2^2))"), "\\psi(\\Omega_{2}\\psi_{1}(\\Omega_{2}))");
    }

    #[test]
    fn omega4_region() {
        // Row 491
        assert_eq!(conv("ψ(Ω_4+Ω×ψ(Ω_4))"), "\\psi(\\psi_{3}(0) + \\psi(\\psi_{3}(0)))");
        // Row 494
        assert_eq!(conv("ψ(Ω_4+Ω_2×ψ_1(Ω_4))"), "\\psi(\\psi_{3}(0) + \\psi_{1}(\\psi_{3}(0)))");
        // Row 495
        assert_eq!(conv("ψ(Ω_4+Ω_2^2)"), "\\psi(\\psi_{3}(0) + \\Omega_{2})");
        // Row 496 (corrected): follows the s=2,3 pattern
        assert_eq!(conv("ψ(Ω_4×2)"), "\\psi(\\psi_{3}(1))");
        // Row 518
        assert_eq!(conv("ψ(Ω_ω+Ω_2)"), "\\psi(\\Omega_{\\omega} + \\psi_{1}(\\Omega_{\\omega} + 1))");
    }
}

// Scratch harness to audit CSV coverage.
#[cfg(test)]
mod csv_audit {
    use super::bocf_to_mocf;

    #[test]
    fn audit_all_rows() {
        let content = std::fs::read_to_string("/data/data/com.termux/files/home/bms-analyzer-enhanced-main/bocf vs mocf.csv")
            .expect("csv not found");
        let mut errors: Vec<String> = Vec::new();
        let mut mismatches: Vec<(usize, String, String)> = Vec::new();
        let mut nonstandard: Vec<(usize, String, String)> = Vec::new();
        let mut structural: Vec<(usize, String, String)> = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if idx == 0 || line.trim().is_empty() { continue; }
            // parse two quoted CSV fields
            let fields = split_csv(&line);
            if fields.len() < 2 { continue; }
            let input = fields[0].replace("\\cdot", "*");
            let expected = fields[1].clone();
            if let Ok(ast) = crate::parser::parse_bocf(&input) {
                if let Ok(t) = crate::parser::eval_ast(&ast) {
                    let sf = crate::term::standard_form(&t);
                    if !crate::term::is_bocf_standard(&sf) {
                        nonstandard.push((
                            idx + 1,
                            fields[0].clone(),
                            crate::term::term_to_string(false, &sf),
                        ));
                    }
                }
            }
            match bocf_to_mocf(&input) {
                Err(e) => errors.push(format!("row {}: {} → ERR: {}", idx + 1, fields[0], e)),
                Ok(got) => {
                    let norm = |s: String| s.replace("\\cdot", "").chars().filter(|c| !matches!(c, '{' | '}' | ' ' | '\t')).collect::<String>();
                    let got_n = norm(got);
                    let exp_n = norm(expected);
                    if got_n != exp_n {
                        mismatches.push((idx + 1, got_n, exp_n));
                    }
                }
            }
            if let Ok(c) = super::bocf_to_c(&input) {
                let sn = super::mocf_normalize(&c);
                if super::render(&sn) != super::render(&c) {
                    structural.push((idx + 1, super::render(&c), super::render(&sn)));
                }
            }
        }
        println!("== NONSTANDARD INPUTS: {} ==", nonstandard.len());
        for (r, raw, std) in &nonstandard {
            println!("row {}: {}\n   std: {}", r, raw, std);
        }
        println!("== PARSE/CONVERT ERRORS: {} ==", errors.len());
        for e in &errors { println!("{}", e); }
        println!("== MISMATCHES: {} ==", mismatches.len());
        for (r, g, e) in &mismatches {
            println!("row {}:\n   got  {}\n   want {}", r, g, e);
        }
        println!("== STRUCTURAL-NORMALIZE CHANGES: {} ==", structural.len());
        for (r, before, after) in &structural {
            println!("row {}:\n   raw {}\n   nf  {}", r, before, after);
        }
    }

    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_q = false;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' { in_q = !in_q; }
            else if c == ',' && !in_q { out.push(cur.clone()); cur.clear(); }
            else { cur.push(c); }
            i += 1;
        }
        out.push(cur);
        out
    }
}
