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
        Ast::Psi(None, _, ..) => true,
        Ast::Pow(b, _) => is_below_omega1(b),
        Ast::Mul(l, r) => is_below_omega1(l) && is_below_omega1(r),
        Ast::Omega(_, ..) => false,
        Ast::Psi(Some(_), _, ..) => false,
        Ast::Add(l, r) => is_below_omega1(l) && is_below_omega1(r),
    }
}

/// Conservative ordinal comparison a < b on subscript-shaped ASTs.
/// Purely structural — never re-evaluates ψ (BOCF and MOCF assign different
/// values to the same ψ-syntax, so processed values must not be re-evaluated
/// for comparison).
fn sub_ord_lt(a: &Ast, b: &Ast) -> bool {
    if ast_eq(a, b) {
        return false;
    }
    match (a, b) {
        (Ast::Num(x), Ast::Num(y)) => x < y,
        (Ast::Num(_), Ast::W) => true,
        (Ast::Num(_), Ast::Omega(_, ..)) => true,
        (Ast::Num(_), Ast::Psi(Some(_), _, ..)) => true,
        (Ast::Num(_), Ast::Add(l, _)) => sub_ord_leq(a, l),
        (Ast::W, Ast::Omega(_, ..)) => true,
        (Ast::W, Ast::Psi(Some(_), _, ..)) => true,
        (Ast::W, Ast::Add(l, _)) => sub_ord_leq(a, l),
        (Ast::Omega(Some(x), ..), Ast::Omega(Some(y), ..)) => sub_ord_lt(x, y),
        (Ast::Omega(Some(x), ..), Ast::Psi(Some(u), _, ..)) => sub_ord_lt(x, u),
        (Ast::Omega(Some(_), ..), Ast::Add(l, _)) => sub_ord_leq(a, l),
        (Ast::Omega(None, ..), Ast::Add(l, _)) => sub_ord_leq(a, l),
        (Ast::Psi(Some(u), _, ..), Ast::Omega(Some(y), ..)) => sub_ord_lt(u, y),
        (Ast::Psi(Some(_), _, ..), Ast::Add(l, _)) => sub_ord_leq(a, l),
        (Ast::Add(l, _), _) => sub_ord_lt(l, b),
        // Two multiples of the same leading term compare by multiplier
        // (ω×2 < ω×3 ⟺ 2 < 3).
        (Ast::Mul(l1, r1), Ast::Mul(l2, r2)) if ast_eq(l1, l2) => sub_ord_lt(r1, r2),
        // A power vs a product: same-base powers compare exponents
        // (ω^3 > ω^2×2, ω^2 < ω^2×2); otherwise compare the base.
        (Ast::Pow(b1, e1), Ast::Mul(l2, _)) => match l2.as_ref() {
            Ast::Pow(b2, e2) if ast_eq(b1, b2) => !sub_ord_lt(e2, e1),
            _ => sub_ord_lt(b1, l2),
        },
        // a < l·r (a limit-multiple) reduces to a ≤ l when a is not a
        // multiple of l with its own multiplier (ω+1 < ω×2 ⟺ ω+1 ≤ ω×2).
        (_, Ast::Mul(l, _)) => sub_ord_leq(a, l),
        // a < b^e reduces to a ≤ b (Add/Mul left sides are reduced by the
        // arms above; ω+1 < ω^2 ⟺ ω+1 ≤ ω^2 via the Add arm).
        (_, Ast::Pow(b, _)) => sub_ord_leq(a, b),
        (Ast::Mul(l, _), _) => sub_ord_lt(l, b),
        (Ast::Pow(base, e1), Ast::Pow(base2, e2)) if ast_eq(base, base2) => {
            sub_ord_lt(e1, e2)
        }
        (Ast::Pow(base, _), _) => sub_ord_lt(base, b),
        _ => false,
    }
}

fn sub_ord_leq(a: &Ast, b: &Ast) -> bool {
    ast_eq(a, b) || sub_ord_lt(a, b)
}

/// Term-aware subscript comparison: when both embedded original-BOCF
/// subscript terms are available, compare them with the term layer's
/// complete ordering (term::lt). This correctly orders values the structural
/// comparison cannot (e.g. ψ_4(Ω_5) > Ω_4). Falls back to the structural
/// comparison when a term is missing.
fn sub_ord_lt_t(
    a: &Ast,
    ta: Option<&crate::term::Term>,
    b: &Ast,
    tb: Option<&crate::term::Term>,
) -> bool {
    if ast_eq(a, b) {
        return false;
    }
    if let (Some(x), Some(y)) = (ta, tb) {
        return crate::term::lt(x, y);
    }
    sub_ord_lt(a, b)
}

fn sub_ord_leq_t(
    a: &Ast,
    ta: Option<&crate::term::Term>,
    b: &Ast,
    tb: Option<&crate::term::Term>,
) -> bool {
    if ast_eq(a, b) {
        return true;
    }
    if let (Some(x), Some(y)) = (ta, tb) {
        return crate::term::lt(x, y) || crate::term::eq(x, y);
    }
    sub_ord_lt(a, b)
}

/// Extract the embedded original-BOCF subscript term from a lead block
/// (Ω_s, Ω_s×k or Ω_s^e), digging through the product/power to the Ω node.
fn block_sub_term(block: &Ast) -> Option<&crate::term::Term> {
    match block {
        Ast::Omega(_, t) => t.as_ref(),
        Ast::Mul(b, _) => block_sub_term(b),
        Ast::Pow(b, _) => block_sub_term(b),
        _ => None,
    }
}

/// True if b < Ω_{v+1}.  Any ψ_a(b) and its operations with values below
/// Ω_{a+1} stay below Ω_{a+1}, so ψ-blocks with a ≤ v (and products,
/// powers and sums of such blocks) qualify.
fn below_next_cardinal(v: &Ast, b: &Ast) -> bool {
    match b {
        Ast::Num(_) | Ast::W | Ast::Psi(None, _, ..) => true,
        Ast::Psi(Some(u), _, ..) => sub_ord_leq(u, v),
        Ast::Omega(None, ..) => !matches!(v, Ast::Num(0)),
        Ast::Omega(Some(t), ..) => sub_ord_leq(t, v),
        Ast::Pow(base, _) => below_next_cardinal(v, base),
        Ast::Mul(l, r) => below_next_cardinal(v, l) && below_next_cardinal(v, r),
        Ast::Add(l, r) => below_next_cardinal(v, l) && below_next_cardinal(v, r),
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
        Ast::Omega(None, ..) => Some(Head::OmegaPow(Ast::Num(1))),
        Ast::Omega(Some(s), ..) => Some(Head::Cardinal((**s).clone())),
        Ast::Pow(b, e) => match b.as_ref() {
            Ast::Omega(None, ..) => Some(Head::OmegaPow((**e).clone())),
            Ast::Omega(Some(s), ..) => Some(Head::CardinalPow((**s).clone(), (**e).clone())),
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
        Ast::Omega(None, ..) => Ast::Num(1),
        Ast::Mul(b, k) if matches!(b.as_ref(), Ast::Omega(None, ..)) => (**k).clone(),
        Ast::Mul(b, _)
            if matches!(b.as_ref(), Ast::Omega(Some(sub), ..) if is_successor_ord(sub)) =>
        {
            match b.as_ref() {
                Ast::Omega(Some(_), ..) => match block {
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
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(None, ..)) => {
            if let Some(n) = as_nat(e) {
                Ast::Pow(Box::new(Ast::Omega(None, None)), Box::new(Ast::Num(n - 1)))
            } else {
                block.clone()
            }
        }
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(Some(_), ..)) => {
            // True-limit subscripts (Ω_ω, Ω_Ω, …) keep their powers;
            // successor subscripts lower a finite exponent by one.
            let is_limit_sub = match b.as_ref() {
                Ast::Omega(Some(sub), ..) => !is_successor_ord(sub) && !matches!(sub.as_ref(), Ast::Num(_)),
                _ => false,
            };
            if is_limit_sub {
                return block.clone();
            }
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

/// Lower a finite Ω-cardinal power exponent by one; exponent 2 becomes the
/// bare cardinal (Ω_s^2 → Ω_s, not Ω_s^1). A multiplied power Ω_s^e×k lowers
/// the power and keeps the multiplier (Ω_s^2×k → Ω_s×k).
fn lower_cardpow_once(block: &Ast) -> Ast {
    match block {
        Ast::Mul(p, k)
            if matches!(p.as_ref(), Ast::Pow(b, _) if matches!(b.as_ref(), Ast::Omega(Some(_), ..))) =>
        {
            Ast::Mul(Box::new(lower_cardpow_once(p)), k.clone())
        }
        Ast::Pow(base, e) => match as_nat(e) {
            Some(2) => (**base).clone(),
            Some(n) if n > 2 => Ast::Pow(base.clone(), Box::new(Ast::Num(n - 1))),
            _ => block.clone(),
        },
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
        Ast::Psi(Some(sub), arg, ..) => {
            let arg = rewrite_nonstandard_psi(arg);
            let sub = rewrite_nonstandard_psi(sub);
            let node = Ast::Psi(Some(Box::new(sub.clone())), Box::new(arg), None);
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
        Ast::Psi(None, arg, ..) => Ast::Psi(None, Box::new(rewrite_nonstandard_psi(arg)), None),
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
        let part = if tm::is_ordinal_finite(&run) {
            Ast::Num(count)
        } else {
            term_block_ast(&head, count)
        };
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
        Ast::Omega(None, None)
    } else {
        Ast::Omega(Some(Box::new(term_to_ast(a))), Some(a.clone()))
    }
}

fn term_block_ast(p: &crate::term::Term, count: i32) -> Ast {
    use crate::term as tm;
    let node = p.as_ref().unwrap();
    let a = &node.a;
    let b = &node.b;
    let psi_form = || {
        if tm::is_zero(a) {
            Ast::Psi(None, Box::new(term_to_ast(b)), None)
        } else {
            Ast::Psi(Some(Box::new(term_to_ast(a))), Box::new(term_to_ast(b)), Some(a.clone()))
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
        Ast::Omega(None, ..) => C::Omega,
        Ast::Omega(Some(s), ..) => C::OmegaSub(Box::new(conv_ord(s))),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(n, &mut blocks);
            c_sum(blocks.iter().map(conv_ord).collect())
        }
        Ast::Mul(l, r) => c_mul(conv_ord(l), conv_ord(r)),
        Ast::Pow(b, e) => conv_pow(b, e),
        Ast::Psi(sub, arg, st) => conv_psi(sub.as_deref(), arg, st.as_ref()),
    }
}

fn conv_pow(base: &Ast, exp: &Ast) -> C {
    match base {
        Ast::W => C::OmegaPow(Box::new(conv_ord(exp))),
        Ast::Omega(None, ..) => C::Pow(Box::new(C::Omega), Box::new(conv_ord(exp))),
        Ast::Omega(Some(_), ..) => C::Pow(Box::new(conv_ord(base)), Box::new(conv_ord(exp))),
        _ => C::Pow(Box::new(conv_ord(base)), Box::new(conv_ord(exp))),
    }
}

/// Convert ψ_v(a).  v = None is the level-0 collapse (the main region).
/// `sub_term` is the embedded original-BOCF term of the subscript v (when
/// available), used for term-layer ordinal comparisons.
fn conv_psi(sub: Option<&Ast>, arg: &Ast, sub_term: Option<&crate::term::Term>) -> C {
    match sub {
        None => conv_psi0(arg),
        Some(v) => {
            if let Some(c) = collapse_psi_next_cardinal(v, arg, sub_term) {
                return c;
            }
            // General rule: ψ_v(X + β) = ψ_v(X)·ω^β for any trailing
            // β < Ω_{v+1} (any ψ_a(b) and its operations with values below
            // Ω_{a+1} stay below Ω_{a+1}); finite n peels unconditionally.
            let mut blocks = Vec::new();
            flatten_add(arg, &mut blocks);
            if !blocks.is_empty() {
                let last = blocks[blocks.len() - 1].clone();
                let last_n = match &last {
                    Ast::Num(n) => Some(*n),
                    Ast::Mul(l, r) => match (l.as_ref(), r.as_ref()) {
                        (Ast::Num(1), Ast::Num(n)) | (Ast::Num(n), Ast::Num(1)) => Some(*n),
                        _ => None,
                    },
                    _ => None,
                };
                let peel = match last_n {
                    Some(n) if n >= 1 => true,
                    _ => blocks.len() >= 2 && below_next_cardinal(v, &last),
                };
                if peel {
                    let x = sum_of(&blocks[..blocks.len() - 1]);
                    let base = conv_psi(Some(v), &x, sub_term);
                    return c_mul(base, C::OmegaPow(Box::new(conv_ord(&last))));
                }
            }
            // Level-shift: the subscript collapses the next cardinal.
            let vc = conv_ord(v);
            let argc = conv_at_level(v, arg);
            C::Psi(Some(Box::new(vc)), Box::new(argc))
        }
    }
}

/// ψ_v(Ω_{v+1}·k + r) at any level v: the Ω_{v+1}·k lead collapses to
/// ψ_v(σ(k)); ψ_v(Ω_{v+1}·j + y)·m blocks of the tail become multiplicative
/// factors (recursively collapsed), parts below Ω become an ω^ factor
/// (conv_sym keeps ψ(0) symbolic so ω^{ψ(0)} absorbs to ψ(0)), and Ω-power
/// parts stay as factors.  A bare Ω_{v+1}^e lead collapses by lowering a
/// finite exponent (e ≥ 2) and keeps limit exponents.
/// True if s is a limit-multiple subscript: λ·k with λ limit, k ≥ 2
/// (ω×2, Ω×2, Ω_ω×2, …). These keep their lead instead of σ-collapsing.
fn is_limit_multiple(s: &Ast) -> bool {
    match s {
        Ast::Mul(l, r) => {
            !matches!(l.as_ref(), Ast::Num(_))
                && !is_successor_ord(l)
                && as_nat(r).map_or(false, |n| n >= 2)
        }
        _ => false,
    }
}

fn collapse_psi_next_cardinal(
    v: &Ast,
    arg: &Ast,
    v_term: Option<&crate::term::Term>,
) -> Option<C> {
    let mut blocks = Vec::new();
    flatten_add(arg, &mut blocks);
    if blocks.is_empty() {
        return None;
    }
    let (head_opt, mult) = split_head_mult(&blocks[0]);
    let vc = conv_ord(v);
    // Embedded original-BOCF terms for term-layer comparison of subscripts.
    let lead_s_term = block_sub_term(&blocks[0]);
    let next_sub_term: Option<crate::term::Term> = v_term.map(crate::term::succ);
    match head_opt {
        Some(Head::CardinalPow(s, e)) => {
            // Successor-cardinal powers lower their exponent by one
            // (translate_down); limit exponents stay. Applies both to the
            // collapse cardinal Ω_{v+1} and to successor leads above it.
            let is_next = ast_eq(&pred_ord(&s), v) && !is_limit_multiple(&s);
            let is_limit_lead = !is_successor_ord(&s) && !matches!(&s, Ast::Num(_));
            let v_nat = as_nat(v);
            let collapse_sub = v_nat.map(|vn| vn + 1);
            let next_sub: Ast = match v {
                Ast::Num(n) => Ast::Num(n + 1),
                other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
            };
            let is_above = is_successor_ord(&s)
                && !is_next
                && sub_ord_lt_t(&next_sub, next_sub_term.as_ref(), &s, lead_s_term);
            if !is_next && !is_above && !is_limit_lead {
                return None;
            }
            if is_limit_lead {
                // Limit-subscript lead (e.g. Ω_ω^2): the lead stays, the tail
                // becomes factors (ψ_1(Ω_ω^2+1) → ψ_1(Ω_ω^2)·ω).
                let base_arg = conv_ord(&blocks[0]);
                return Some(finish_limit_tail(
                    v, &vc, &s, &blocks[0], base_arg, &blocks[1..], false, true, collapse_sub,
                ));
            }
            if !matches!(mult, Ast::Num(1)) && as_nat(&e).map_or(true, |n| n < 2) {
                return None;
            }
            if blocks.len() > 1 || !matches!(mult, Ast::Num(1)) {
                if is_next && as_nat(&e).is_none() {
                    // Limit-exponent lead Ω_{v+1}^λ stays; an Ω_{v+1}-built
                    // tail translates down into a ψ₀-argument:
                    // ψ_1(Ω_2^ω + Ω_2·k) → ψ(Ω_2^ω + k),
                    // ψ_1(Ω_2^ω + Ω_2^2) → ψ(Ω_2^ω + Ω_2).
                    let mut parts: Vec<C> = vec![conv_ord(&blocks[0])];
                    for b in &blocks[1..] {
                        let (h2, _) = split_head_mult(b);
                        let ok = match &h2 {
                            Some(Head::Cardinal(t)) => ast_eq(t, &s),
                            Some(Head::CardinalPow(t, _)) => ast_eq(t, &s),
                            _ => false,
                        };
                        if !ok {
                            return None;
                        }
                        // Ω_s → 1, Ω_s·k → k, Ω_s^e → Ω_s^{e-1}
                        let tb = match b {
                            Ast::Omega(Some(_), ..) => Ast::Num(1),
                            Ast::Mul(p, k) if matches!(p.as_ref(), Ast::Omega(Some(_), ..)) => {
                                (**k).clone()
                            }
                            _ => translate_down(b),
                        };
                        parts.push(conv_ord(&tb));
                    }
                    return Some(C::Psi(Some(Box::new(vc)), Box::new(c_sum(parts))));
                }
                if is_above && as_nat(&e).is_none() {
                    // Limit-exponent is_above lead Ω_s^λ stays; an
                    // Ω_{v+1}·m tail collapses to ψ_v(lead + m)
                    // (ψ_1(Ω_3^ω+Ω_2) → ψ_1(Ω_3^ω+ψ_1(Ω_3^ω+1))).
                    let lead = conv_ord(&blocks[0]);
                    let vp1: Ast = match v {
                        Ast::Num(n) => Ast::Num(n + 1),
                        other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
                    };
                    let mut parts: Vec<C> = vec![lead.clone()];
                    for b in &blocks[1..] {
                        let (h2, m2) = split_head_mult(b);
                        let is_vp1 = match &h2 {
                            Some(Head::Cardinal(t)) => ast_eq(t, &vp1),
                            Some(Head::CardinalPow(t, _)) => ast_eq(t, &vp1),
                            _ => false,
                        };
                        if !is_vp1 {
                            return None;
                        }
                        parts.push(C::Psi(
                            Some(Box::new(vc.clone())),
                            Box::new(c_sum(vec![lead.clone(), conv_ord(&m2)])),
                        ));
                    }
                    return Some(C::Psi(Some(Box::new(vc)), Box::new(c_sum(parts))));
                }
                if (is_next || is_above) && as_nat(&e).map_or(false, |n| n >= 2) {
                    // Finite-exponent lead lowers by one, keeping the
                    // multiplier; Ω_s-built tails divide by Ω_s
                    // (ψ_1(Ω_2^2+Ω_2·X) → ψ_1(Ω_2+X),
                    //  ψ_1(Ω_3^2+Ω_3) → ψ_1(Ω_3+1)):
                    //   Ω_s → 1, Ω_s·X → conv(X), Ω_s^e → Ω_s^{e-1}.
                    let new_lead = c_mul(
                        make_cardinalpow(&conv_ord(&s), &conv_ord(&pred_ord(&e))),
                        conv_ord(&mult),
                    );
                    let vp1: Ast = match v {
                        Ast::Num(n) => Ast::Num(n + 1),
                        other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
                    };
                    let mut parts: Vec<C> = vec![new_lead.clone()];
                    for b in &blocks[1..] {
                        let (h2, m2) = split_head_mult(b);
                        let is_vp1 = match &h2 {
                            Some(Head::Cardinal(t)) => ast_eq(t, &vp1),
                            Some(Head::CardinalPow(t, _)) => ast_eq(t, &vp1),
                            _ => false,
                        };
                        if is_vp1 {
                            // Ω_{v+1}·k tail folds to +k
                            // (ψ_1(Ω_3^2+Ω_2) → ψ_1(Ω_3+1)); an
                            // Ω_{v+1}^e power tail (e ≥ 2) lowers by one
                            // (ψ_1(Ω_3^2+Ω_2^2) → ψ_1(Ω_3+Ω_2)).
                            let folded = match b {
                                Ast::Pow(_, e) if as_nat(e).map_or(false, |n| n >= 2) => {
                                    translate_down(b)
                                }
                                _ => m2.clone(),
                            };
                            parts.push(conv_ord(&folded));
                            continue;
                        }
                        // Ω_{s'}·X tail (s' > v+1) → ψ_{s'-1}(lead + M(X));
                        // a ψ_{s'-1} factor X fully collapses. For s'=s this
                        // is ψ_0's Ω_s·X rule; for s'≠s (e.g. ψ_1(Ω_4^2+Ω_3))
                        // the lowered lead Ω_s is used.
                        let s_tail = match &h2 {
                            Some(Head::Cardinal(t)) => Some((*t).clone()),
                            Some(Head::CardinalPow(t, _)) => Some((*t).clone()),
                            _ => None,
                        };
                        if let Some(t) = s_tail {
                            if is_successor_ord(&t) && sub_ord_lt(&vp1, &t) {
                                let pred_t = pred_ord(&t);
                                let is_psi_pred_factor = match &m2 {
                                    Ast::Psi(Some(sub), _, ..) => ast_eq(sub, &pred_t),
                                    Ast::Mul(p, _) => matches!(p.as_ref(),
                                        Ast::Psi(Some(sub), _, ..) if ast_eq(sub, &pred_t)),
                                    _ => false,
                                };
                                let xc = if is_psi_pred_factor {
                                    conv_ord(&m2)
                                } else {
                                    conv_sym(&card_arg_shift(&t, &m2))
                                };
                                parts.push(C::Psi(
                                    Some(Box::new(conv_ord(&pred_t))),
                                    Box::new(c_sum(vec![new_lead.clone(), xc])),
                                ));
                                continue;
                            }
                        }
                        return None;
                    }
                    return Some(C::Psi(Some(Box::new(vc)), Box::new(c_sum(parts))));
                }
                return None;
            }
            let translated = translate_down(&blocks[0]);
            Some(C::Psi(
                Some(Box::new(vc)),
                Box::new(conv_ord(&translated)),
            ))
        }
        Some(Head::Cardinal(s)) => {
            // s = v+1 → the lead collapses to ψ_v(σ(k)); a limit subscript
            // (e.g. Ω_ω) keeps the base ψ_v(Ω_s·k) unchanged; a successor
            // lead above Ω_{v+1} collapses to ψ_{s-1}(σ(k)) as ψ_v's argument.
            let is_next = ast_eq(&pred_ord(&s), v) && !is_limit_multiple(&s);
            let is_limit_lead = !is_successor_ord(&s) && !matches!(&s, Ast::Num(_));
            let v_nat = as_nat(v);
            let collapse_sub = v_nat.map(|vn| vn + 1);
            let next_sub: Ast = match v {
                Ast::Num(n) => Ast::Num(n + 1),
                other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
            };
            let is_above = is_successor_ord(&s)
                && !is_next
                && sub_ord_lt_t(&next_sub, next_sub_term.as_ref(), &s, lead_s_term);
            if !is_next && !is_limit_lead && !is_above {
                return None;
            }
            // Ω_s·k + Ω_s·m + rest = Ω_s·(k+m) + rest: merge contiguous
            // Ω_s-multiple tail blocks into the multiplier
            // (ψ_1(Ω_2·ψ_1(Ω_2)+Ω_2) → ψ_1(ψ_1(0)+1)).
            let mut merged_mult = mult.clone();
            let mut rest_start = 1usize;
            if is_next {
                let mut extra: Vec<Ast> = Vec::new();
                for b in &blocks[1..] {
                    let mc = match b {
                        Ast::Omega(Some(t), ..) if ast_eq(t, &s) => Some(Ast::Num(1)),
                        Ast::Mul(p, k) => match p.as_ref() {
                            Ast::Omega(Some(t), ..) if ast_eq(t, &s) => Some((**k).clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(c) = mc {
                        extra.push(c);
                        rest_start += 1;
                    } else {
                        break;
                    }
                }
                if !extra.is_empty() {
                    let mut all = vec![merged_mult];
                    all.extend(extra);
                    merged_mult = sum_of(&all);
                }
            }
            let mut rest_blocks = &blocks[rest_start..];
            let vp1: Ast = match v {
                Ast::Num(n) => Ast::Num(n + 1),
                other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
            };
            // is_above: a leading tail successor cardinal Ω_{s'} above
            // Ω_{v+1} collapses to ψ_{s'-1}(collapsed_lead + shift(m)), where
            // collapsed_lead = ψ_{s-1}(σ(mult)) (ψ_1(Ω_4+Ω_3) →
            // ψ_1(ψ_3(0)+ψ_2(ψ_3(0)+1))).
            let mut above_card_parts: Vec<C> = Vec::new();
            if is_above {
                let collapsed_lead = C::Psi(
                    Some(Box::new(conv_ord(&pred_ord(&s)))),
                    Box::new(sigma(&mult)),
                );
                let mut i = 0usize;
                while i < rest_blocks.len() {
                    let (h2, m2) = split_head_mult(&rest_blocks[i]);
                    let ht = match &h2 {
                        Some(Head::Cardinal(t))
                            if is_successor_ord(t) && sub_ord_lt(&vp1, t) =>
                        {
                            Some((*t).clone())
                        }
                        _ => None,
                    };
                    if let Some(t) = ht {
                        let xc = conv_sym(&card_arg_shift(&t, &m2));
                        above_card_parts.push(C::Psi(
                            Some(Box::new(conv_ord(&pred_ord(&t)))),
                            Box::new(c_sum(vec![collapsed_lead.clone(), xc])),
                        ));
                        i += 1;
                    } else {
                        break;
                    }
                }
                rest_blocks = &rest_blocks[i..];
            }
            // is_above: tail Ω_{v+1}·k folds to +k in the argument
            // (ψ_1(Ω_3+Ω_2) → ψ_1(ψ_2(0)+1)).
            let mut above_fold: Vec<Ast> = Vec::new();
            if is_above {
                let mut i = 0usize;
                while i < rest_blocks.len() {
                    let mc = match &rest_blocks[i] {
                        Ast::Omega(Some(t), ..) if ast_eq(t, &vp1) => Some(Ast::Num(1)),
                        Ast::Mul(p, k) => match p.as_ref() {
                            Ast::Omega(Some(t), ..) if ast_eq(t, &vp1) => Some((**k).clone()),
                            _ => None,
                        },
                        // Ω_{v+1}^e tail (e ≥ 2) folds lowered: +Ω_{v+1}^{e-1}
                        Ast::Pow(p, e) => match p.as_ref() {
                            Ast::Omega(Some(t), ..)
                                if ast_eq(t, &vp1)
                                    && as_nat(e).map_or(false, |n| n >= 2) =>
                            {
                                Some(translate_down(&rest_blocks[i]))
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(c) = mc {
                        above_fold.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                rest_blocks = &rest_blocks[i..];
            }
            let mut base_arg = if is_next {
                sigma(&merged_mult)
            } else if is_above {
                C::Psi(
                    Some(Box::new(conv_ord(&pred_ord(&s)))),
                    Box::new(sigma(&mult)),
                )
            } else {
                conv_ord(&blocks[0])
            };
            if !above_card_parts.is_empty() {
                let mut p = vec![base_arg];
                p.extend(above_card_parts);
                base_arg = c_sum(p);
            }
            if !above_fold.is_empty() {
                let fold_c = conv_ord(&sum_of(&above_fold));
                base_arg = c_sum(vec![base_arg, fold_c]);
            }
            let base = C::Psi(Some(Box::new(vc.clone())), Box::new(base_arg.clone()));
            let tail = sum_of(rest_blocks);
            if is_zero_ast(&tail) {
                return Some(base);
            }
            Some(finish_limit_tail(
                v, &vc, &s, &blocks[0], base_arg, rest_blocks, is_next, is_limit_lead, collapse_sub,
            ))
        }
        _ => None,
    }
}

/// Shared tail handling for a ψ_v(Ω_s·… + tail) lead: classify the tail
/// blocks, apply the exponent machinery for ψ_v-block tails under an
/// Ω_{v+1} lead, fold limit-lead tail cardinals into the argument, and
/// multiply remaining factors / small ω^ parts.
#[allow(clippy::too_many_arguments)]
fn finish_limit_tail(
    v: &Ast,
    vc: &C,
    s: &Ast,
    lead: &Ast,
    base_arg: C,
    tail_blocks: &[Ast],
    is_next: bool,
    is_limit_lead: bool,
    collapse_sub: Option<i32>,
) -> C {
    let mult = match split_head_mult(lead).1 {
        m => m,
    };
    let base = C::Psi(Some(Box::new(vc.clone())), Box::new(base_arg.clone()));
    let tail = sum_of(tail_blocks);
    let mut tblocks = Vec::new();
    flatten_add(&tail, &mut tblocks);
    let mut small: Vec<Ast> = Vec::new();
    let mut psi_blocks: Vec<Ast> = Vec::new();
    let mut factors: Vec<C> = Vec::new();
    let mut limit_arg_parts: Vec<C> = Vec::new();
    for b in &tblocks {
        if is_next && as_psi_card_block(b, v, s).is_some() {
            psi_blocks.push(b.clone());
        } else if is_below_omega1(b) {
            small.push(b.clone());
        } else if let Some((j, y, m)) = as_psi_card_block(b, v, s) {
            // limit-lead: ψ_v-block recurses as a plain factor
            let om_j = Ast::Mul(
                Box::new(Ast::Omega(Some(Box::new(s.clone())), None)),
                Box::new(j),
            );
            let farg = if is_zero_ast(&y) {
                om_j
            } else {
                Ast::Add(Box::new(om_j), Box::new(y))
            };
            let f = collapse_psi_next_cardinal(v, &farg, None).unwrap_or_else(|| {
                C::Psi(Some(Box::new(vc.clone())), Box::new(conv_at_level(v, &farg)))
            });
            let fc = c_mul(f, conv_ord(&m));
            if matches!(v, Ast::Num(_)) {
                factors.push(fc);
            } else {
                // ψ_ω-level: the factor enters as ω^{…}
                // (ψ_ω(Ω_{ω×2}+ψ_ω(Ω_{ω×2}+Ω)) → ψ_ω(Ω_{ω×2})^Ω).
                factors.push(normalize_omegapow(fc));
            }
        } else if let Some((lead_pow, j, y, m)) = as_psi_cardpow_block(b, v, s) {
            // limit-lead: ψ_v(Ω_s^e·j + y)·m recurses as a plain factor
            let om_j = Ast::Mul(Box::new(lead_pow), Box::new(j));
            let farg = if is_zero_ast(&y) {
                om_j
            } else {
                Ast::Add(Box::new(om_j), Box::new(y))
            };
            let f = collapse_psi_next_cardinal(v, &farg, None).unwrap_or_else(|| {
                C::Psi(Some(Box::new(vc.clone())), Box::new(conv_at_level(v, &farg)))
            });
            factors.push(c_mul(f, conv_ord(&m)));
        } else if is_limit_lead {
            // Limit-lead tail cardinals fold into the argument
            // (collapse_fixed_cardinal's pattern): the collapse
            // cardinal Ω_{v+1}·m → m; a higher cardinal Ω_s2 (s2>v+1)
            // → ψ_{s2-1}(Ω_s·k + m); anything else translates down.
            let (h2, m2) = split_head_mult(b);
            match &h2 {
                Some(Head::Cardinal(s2)) => {
                    let s2_nat = as_nat(s2);
                    let vp1: Ast = match v {
                        Ast::Num(n) => Ast::Num(n + 1),
                        other => Ast::Add(Box::new(other.clone()), Box::new(Ast::Num(1))),
                    };
                    let is_collapse_card = matches!((s2_nat, collapse_sub), (Some(a), Some(cs)) if a == cs)
                        || ast_eq(s2, &vp1);
                    let above_collapse = sub_ord_lt(&vp1, s2);
                    if is_limit_lead && !matches!(v, Ast::Num(0)) && !is_collapse_card && above_collapse {
                        // v ≥ 1: a tail cardinal above the collapse
                        // cardinal Ω_{v+1} (e.g. Ω_ω > Ω_2 inside ψ_1);
                        // Ω_{u+1}·ψ_u(X) becomes ψ_u(X′ + ψ_u(X′)), a bare
                        // successor cardinal Ω_{s2}·m collapses to
                        // ψ_{s2-1}(lead + m) using the limit lead
                        // (ψ_1(Ω_ω+Ω_3) → ψ_1(Ω_ω+ψ_2(Ω_ω+1))), and a
                        // limit-subscript cardinal stays whole in the
                        // argument.
                        if let Some(f) = collapse_card_mul_psi(b) {
                            limit_arg_parts.push(f);
                        } else if is_successor_ord(s2) {
                            limit_arg_parts.push(C::Psi(
                                Some(Box::new(conv_ord(&pred_ord(s2)))),
                                Box::new(c_sum(vec![
                                    conv_ord(lead),
                                    conv_ord(&m2),
                                ])),
                            ));
                        } else {
                            limit_arg_parts.push(conv_ord(b));
                        }
                        continue;
                    }
                    if is_collapse_card {
                        // Ω_{v+1}·m tail folds to +m at any level
                        // (ψ_ω(Ω_{ω×2}+Ω_{ω+1}) → ψ_ω(Ω_{ω×2}+1)).
                        limit_arg_parts.push(conv_ord(&m2));
                        continue;
                    }
                    match (s2_nat, collapse_sub) {
                        (Some(s2n), Some(csn)) if s2n == csn => {
                            limit_arg_parts.push(conv_ord(&m2));
                        }
                        (Some(s2n), Some(csn)) if s2n > csn => {
                            limit_arg_parts.push(C::Psi(
                                Some(Box::new(conv_ord(&pred_ord(s2)))),
                                Box::new(c_sum(vec![
                                    conv_ord(lead),
                                    conv_ord(&m2),
                                ])),
                            ));
                        }
                        _ => {
                            factors.push(conv_ord(b));
                        }
                    }
                }
                _ => {
                    if matches!(v, Ast::Num(_)) && below_next_cardinal(v, b) {
                        // natural level, tail below Ω_{v+1}: peel as ω^β
                        // (ψ_1(Ω_ω+Ω) → ψ_1(Ω_ω)·Ω).
                        factors.push(normalize_omegapow(conv_ord(b)));
                    } else if matches!(v, Ast::Num(_)) {
                        limit_arg_parts.push(conv_ord(&translate_down(b)));
                    } else {
                        // ψ_ω-level: tails below the collapse cardinal peel
                        // as ω^β (ψ_ω(Ω_{ω×2}+Ω) → ψ_ω(Ω_{ω×2})·Ω).
                        factors.push(normalize_omegapow(conv_ord(b)));
                    }
                }
            }
        } else {
            // At limit-subscript levels (ψ_ω, …), tail values contribute
            // as ω^β (ψ_ω(Ω_{ω+1}·Ω+Ω_ω²) → ψ_ω(Ω)·Ω_ω^{Ω_ω}); natural
            // levels keep the literal factor (ψ_1(Ω_2+Ω²) → ψ_1(0)·Ω²).
            if matches!(v, Ast::Num(_)) {
                factors.push(conv_ord(b));
            } else {
                factors.push(normalize_omegapow(conv_ord(b)));
            }
        }
    }
    // Ω_{v+1} lead (mult 1) with ψ_v-block tails: exponent machinery,
    // the level-v analogue of collapse_omega1's k==1 branch.
    if is_next && matches!(&mult, Ast::Num(1)) && !psi_blocks.is_empty() {
        let pv0 = C::Psi(Some(Box::new(vc.clone())), Box::new(C::Zero));
        let mut e: Option<C> = None;
        let mut has_deep = false;
        for b in &psi_blocks {
            let (_j, y, m) = as_psi_card_block(b, v, s).unwrap();
            let contrib = g_contrib_level(v, &pv0, &y, &m);
            e = Some(match e {
                None => contrib,
                Some(prev) => combine_e_level(&pv0, prev, &contrib),
            });
            if !is_zero_ast(&y) {
                has_deep = true;
            }
        }
        let e = e.unwrap();
        let t = if has_deep { c_mul(pv0.clone(), e) } else { e };
        let small_c = if small.is_empty() {
            C::Zero
        } else {
            conv_ord(&sum_of(&small))
        };
        let exp = if is_c_zero(&small_c) {
            t
        } else {
            c_sum(vec![t, small_c])
        };
        let mut result = c_mul(pv0, C::OmegaPow(Box::new(exp)));
        for f in factors {
            result = c_mul(result, f);
        }
        return result;
    }
    if is_next && !psi_blocks.is_empty() {
        // mult ≠ 1: ψ_v-block tails contribute as ω^{ψ_v(σ(j))·e·m}
        // (ψ_1(Ω_2×2+ψ_1(Ω_2)) → ψ_1(1)·ψ_1(0),
        //  ψ_1(Ω_2×2+ψ_1(Ω_2)×2) → ψ_1(1)·ψ_1(0)²).
        for b in &psi_blocks {
            let (j, y, m) = as_psi_card_block(b, v, s).unwrap();
            let base_j = C::Psi(Some(Box::new(vc.clone())), Box::new(sigma(&j)));
            let e = e_val_level(v, s, &y);
            factors.push(normalize_omegapow(c_mul(
                base_j,
                c_mul(e, conv_ord(&m)),
            )));
        }
        psi_blocks.clear();
    }
    // Limit-subscript level (ψ_ω, …) with a small-only tail:
    // ψ_v(Ω_s·k + n) = ψ_v(Ω_s·k + n−1)·ω (n ≥ 2); n = 1 stays in the
    // argument (rows 825-832).
    let own_floor = ast_eq(s, v);
    if !matches!(v, Ast::Num(_))
        && is_limit_lead
        && own_floor
        && psi_blocks.is_empty()
        && limit_arg_parts.is_empty()
        && factors.is_empty()
        && !small.is_empty()
    {
        let mut total = 0i32;
        let mut all_nat = true;
        for b in &small {
            if let Ast::Num(n) = b { total += n; } else { all_nat = false; break; }
        }
        if all_nat && total >= 1 {
            let n = total;
            if n == 1 {
                return C::Psi(
                    Some(Box::new(vc.clone())),
                    Box::new(c_sum(vec![base_arg, C::Nat(1)])),
                );
            }
            let argp = c_sum(vec![base_arg, c_nat(n - 1)]);
            return c_mul(
                C::Psi(Some(Box::new(vc.clone())), Box::new(argp)),
                C::OmegaPow(Box::new(C::Nat(1))),
            );
        }
    }
    let mut result = if is_limit_lead && !limit_arg_parts.is_empty() {
        let mut arg_parts = vec![base_arg];
        arg_parts.extend(limit_arg_parts);
        C::Psi(Some(Box::new(vc.clone())), Box::new(c_sum(arg_parts)))
    } else {
        base
    };
    for f in factors {
        result = c_mul(result, f);
    }
    let w = sum_of(&small);
    if !is_zero_ast(&w) {
        // conv_ord collapses ψ₀-values (ψ(Ω)=ε₀ → ψ(0)); ω^{ψ(0)}
        // then absorbs to ψ(0).
        result = c_mul(result, C::OmegaPow(Box::new(conv_ord(&w))));
    }
    result
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
            } else if is_successor_ord(&s)
                && (next.is_some() || sub_ord_lt(v, &pred_ord(&s)))
            {
                // Successor subscript above the collapse cardinal
                // (e.g. Ω_{ω+1} in a ψ_1-argument, Ω_{ω+2} in a
                // ψ_ω-argument): Ω_s·k ↦ ψ_{s-1}(σ(k)).
                let ps = pred_ord(&s);
                let inner = C::Psi(Some(Box::new(conv_ord(&ps))), Box::new(sigma(&mult)));
                c_sum(vec![inner, conv_ord(&tail)])
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
/// True if b < Ω_idx (conservative ordinal comparison for fold rules).
fn below_omega_idx(idx: &Ast, b: &Ast) -> bool {
    match b {
        Ast::Num(_) | Ast::W | Ast::Psi(None, _, ..) => true,
        Ast::Omega(None, ..) => sub_ord_lt(&Ast::Num(1), idx),
        Ast::Omega(Some(t), ..) => sub_ord_lt(t, idx),
        Ast::Psi(Some(u), _, ..) => sub_ord_lt(u, idx),
        Ast::Pow(base, _) => below_omega_idx(idx, base),
        Ast::Mul(l, r) => below_omega_idx(idx, l) && below_omega_idx(idx, r),
        Ast::Add(l, r) => below_omega_idx(idx, l) && below_omega_idx(idx, r),
    }
}

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
                        collapse_fixed_kind(make_cardinalpow(&conv_ord(&s), &conv_ord(&e)), &mult, &tail, true)
                    }
                }
            } else {
                // Limit-exponent lead Ω_s^λ: successor-cardinal tails
                // Ω_{u+1}·Y collapse to ψ_u(lead + …), including the
                // same-base tail (ψ(Ω_2^Ω_2+Ω_2) →
                // ψ(Ω_2^Ω_2+ψ_1(Ω_2^Ω_2+1))).
                collapse_fixed_kind(make_cardinalpow(&conv_ord(&s), &conv_ord(&e)), &mult, &tail, true)
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
        Ast::Psi(None, arg, ..) => match arg.as_ref() {
            Ast::Omega(None, ..) => Some((Ast::Num(0), m)),
            Ast::Add(l, r) if matches!(l.as_ref(), Ast::Omega(None, ..)) => Some(((**r).clone(), m)),
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
    if let Ast::Psi(None, inner, ..) = y {
        if let Ast::Omega(None, ..) = inner.as_ref() {
            // ψ(Ω+ψ(Ω))·m → ψ(0)·m
            return c_mul(psi0_c(), mv);
        }
        if let Ast::Add(l, r) = inner.as_ref() {
            if matches!(l.as_ref(), Ast::Omega(None, ..)) {
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

/// Level-v exponent contribution of a ψ_v(Ω_{v+1}·j + y)·m block: y = 0 gives
/// ψ_v(0)·m, otherwise ω^{conv(y)}·m (the level-v analogue of g_contrib).
fn g_contrib_level(v: &Ast, pv0: &C, y: &Ast, m: &Ast) -> C {
    let _ = v;
    let mv = conv_ord(m);
    if is_zero_ast(y) {
        return c_mul(pv0.clone(), mv);
    }
    c_mul(C::OmegaPow(Box::new(conv_ord(y))), mv)
}

/// Combine a trailing ψ_v(Ω_{v+1})·j contribution (as ψ_v(0)·j) into the
/// exponent E (the level-v analogue of combine_e).
fn combine_e_level(pv0: &C, prev: C, contrib: &C) -> C {
    let pv0_s = render(pv0);
    let j = match contrib {
        C::Mul(a, b) if render(a) == pv0_s => (**b).clone(),
        _ if render(contrib) == pv0_s => C::One,
        _ => return c_sum(vec![prev, contrib.clone()]),
    };
    let prev_small = match &prev {
        C::OmegaPow(_) => true,
        C::Mul(a, _) => matches!(a.as_ref(), C::OmegaPow(_)),
        _ => false,
    };
    if prev_small {
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
    if let Ast::Psi(None, arg, ..) = b {
        if let Ast::Pow(bb, e) = arg.as_ref() {
            if matches!(bb.as_ref(), Ast::Omega(None, ..)) {
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
    let convk = if matches!(s, Ast::Num(_)) {
        conv_sym(&card_arg_shift(s, mult))
    } else {
        conv_ord(mult)
    };
    let lead = c_mul(make_cardinalpow(&conv_ord(s), &c_nat(n - 1)), convk);
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_c: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    let mut i = 0usize;
    while i < blocks.len() {
        let b = &blocks[i];
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            i += 1;
            continue;
        }
        let psi_pred = match b {
            Ast::Mul(p, k) => match p.as_ref() {
                Ast::Psi(Some(sub), arg, ..) if ast_eq(sub, &pred) => {
                    Some(((**arg).clone(), (**k).clone()))
                }
                _ => None,
            },
            Ast::Psi(Some(sub), arg, ..) if ast_eq(sub, &pred) => {
                Some(((**arg).clone(), Ast::Num(1)))
            }
            _ => None,
        };
        if let Some((parg, pm)) = psi_pred {
            // Convert the ψ_{pred}-argument fully (σ-collapse, exponent
            // lowering and peel all apply):
            // ψ_1(Ω_2) → ψ_1(0), ψ_1(Ω_2²+Ω_2) → ψ_1(Ω_2+1),
            // ψ_ω(Ω_{ω+1}²+1) → ψ_ω(Ω_{ω+1})·ω.
            let xc = conv_psi(Some(&pred), &parg, None);
            if matches!(pm, Ast::Num(1)) {
                x_c.push(xc);
            } else {
                x_c.push(c_mul(xc, conv_sym(&card_arg_shift(s, &pm))));
            }
            i += 1;
            continue;
        }
        let (h0, m0) = split_head_mult(b);
        let g_sub = match &h0 {
            Some(Head::Cardinal(t)) if is_successor_ord(t) => {
                Some(((*t).clone(), m0.clone(), false))
            }
            Some(Head::CardinalPow(t, e))
                if is_successor_ord(t) && as_nat(e).map_or(false, |n| n >= 2) =>
            {
                // Ω_{s'}^e×k counts as an Ω_{s'}-block carrying its lowering
                Some(((*t).clone(), lower_cardpow_once(b), true))
            }
            _ => None,
        };
        if let Some((g_s, g_m, g_pow)) = g_sub {
            // Group contiguous Ω_{g_s}-built tail blocks; a power block
            // contributes its translate_down lowering.
            let mut mults: Vec<(Ast, bool)> = vec![(g_m, g_pow)];
            let mut j = i + 1;
            while j < blocks.len() {
                let (hj, mj) = split_head_mult(&blocks[j]);
                let cand = match &hj {
                    Some(Head::Cardinal(t)) if ast_eq(t, &g_s) => {
                        Some((mj.clone(), false))
                    }
                    Some(Head::CardinalPow(t, ej))
                        if ast_eq(t, &g_s) && as_nat(ej).map_or(false, |n| n >= 2) =>
                    {
                        Some((lower_cardpow_once(&blocks[j]), true))
                    }
                    _ => None,
                };
                match cand {
                    Some(c) => {
                        mults.push(c);
                        j += 1;
                    }
                    _ => break,
                }
            }
            let sub_idx = if ast_eq(&g_s, s) { pred.clone() } else { pred_ord(&g_s) };
            let shift_s = if ast_eq(&g_s, s) { s.clone() } else { g_s.clone() };
            let mut outer: Vec<C> = Vec::new();
            // The ψ_{s'-1} argument accumulates the lead plus all previously
            // collapsed tail terms (ψ(Ω_3^2+Ω_3+Ω_2) →
            // ψ(Ω_3+ψ_2(Ω_3+1)+ψ_1(Ω_3+ψ_2(Ω_3+1)+1))).
            let mut arg_parts: Vec<C> = vec![lead.clone()];
            arg_parts.extend(x_c.iter().cloned());
            let mut has_bare = false;
            for (m, is_pow) in &mults {
                // A power block lowers into the outer argument and also joins
                // the ψ_{s'-1} argument (ψ(Ω_3^2+Ω_2^2+Ω_2) →
                // ψ(Ω_3+Ω_2+ψ_1(Ω_3+Ω_2+1)), ψ(Ω_2^3+Ω_2^2×2+Ω_2) →
                // ψ(Ω_2^2+Ω_2·2+ψ_1(Ω_2^2+Ω_2·2+1))). With no bare block it
                // only reaches the outer argument (ψ(Ω_2^3+Ω_2^2) →
                // ψ(Ω_2^2+Ω_2)). A bare Ω_{s'}·k block supplies the σ-fold.
                if *is_pow {
                    let mc = conv_ord(m);
                    outer.push(mc.clone());
                    arg_parts.push(mc);
                    continue;
                }
                has_bare = true;
                let is_psi_pred_factor = match m {
                    Ast::Psi(Some(sub), _, ..) => ast_eq(sub, &sub_idx),
                    Ast::Mul(p, _) => matches!(p.as_ref(),
                        Ast::Psi(Some(sub), _, ..) if ast_eq(sub, &sub_idx)),
                    _ => false,
                };
                let xc = if is_psi_pred_factor {
                    conv_ord(m)
                } else {
                    conv_sym(&card_arg_shift(&shift_s, m))
                };
                arg_parts.push(xc);
            }
            x_c.extend(outer);
            if has_bare {
                x_c.push(C::Psi(
                    Some(Box::new(conv_ord(&sub_idx))),
                    Box::new(c_sum(arg_parts)),
                ));
            }
            i = j;
            continue;
        }
        x_c.push(conv_ord(&translate_down(b)));
        i += 1;
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
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(Some(x), ..) if ast_eq(x, s)) => {
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
        Ast::Psi(sub, arg, ..) => Ast::Psi(sub.clone(), Box::new(card_arg_shift(s, arg)), None),
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

    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    // Ω_s·k + Ω_s·m… = Ω_s·(k+m…): merge contiguous Ω_s-multiple tail
    // blocks into the multiplier (rows 1132-1135:
    // ψ(Ω_{ω+1}·Ω+Ω_{ω+1}) → ψ(ψ_ω(Ω+1))).
    let mut merged = mult.clone();
    while !blocks.is_empty() {
        let (h, m) = split_head_mult(&blocks[0]);
        // Only ψ-free coefficients merge into the multiplier;
        // ψ-containing tails stay and become factors
        // (ψ_1(Ω_2×2+ψ_1(Ω_2)) → ψ_1(1)·ψ_1(0)).
        if matches!(&h, Some(Head::Cardinal(t)) if ast_eq(t, s)) && !contains_psi_ast(&m) {
            merged = Ast::Add(Box::new(merged), Box::new(m));
            blocks.remove(0);
        } else {
            break;
        }
    }
    if is_limit_multiple(s) {
        // λ·k subscripts: no σ-collapse; the lead stays
        // (ψ(Ω_{ω×2}+ψ_ω(Ω_{ω×2})) keeps Ω_{ω×2}). Bare ψ_0(Ω_{λ·k})
        // collapses to ψ_λ(Ω_λ·(k−1)).
        let lam = match s { Ast::Mul(l, _) => (**l).clone(), _ => unreachable!() };
        let kk = match s { Ast::Mul(_, r) => as_nat(r).unwrap_or(1), _ => 1 };
        let lead = c_mul(C::OmegaSub(Box::new(conv_ord(s))), conv_ord(&merged));
        if blocks.is_empty() {
            if matches!(merged, Ast::Num(1)) {
                let argp = if kk - 1 <= 1 {
                    C::OmegaSub(Box::new(conv_ord(&lam)))
                } else {
                    c_mul(C::OmegaSub(Box::new(conv_ord(&lam))), c_nat(kk - 1))
                };
                return C::Psi(Some(Box::new(conv_ord(&lam))), Box::new(argp));
            }
            return C::Psi(None, Box::new(lead));
        }
        let mut x_c: Vec<C> = Vec::new();
        let mut w_parts2: Vec<Ast> = Vec::new();
        for b in &blocks {
            if is_below_omega1(b) {
                w_parts2.push(b.clone());
                continue;
            }
            let (h, m) = split_head_mult(b);
            match &h {
                Some(Head::Cardinal(s2)) if is_successor_ord(s2) && !matches!(s2, Ast::Num(1)) && as_nat(s2).map_or(true, |n| n >= 2) => {
                    let mut parts = vec![lead.clone()];
                    parts.extend(x_c.iter().cloned());
                    parts.push(conv_ord(&m));
                    x_c.push(C::Psi(
                        Some(Box::new(conv_ord(&pred_ord(s2)))),
                        Box::new(c_sum(parts)),
                    ));
                }
                _ => {
                    x_c.push(conv_ord(&translate_down(b)));
                }
            }
        }
        let mut parts = vec![lead];
        parts.extend(x_c);
        let psi = C::Psi(None, Box::new(c_sum(parts)));
        let w = sum_of(&w_parts2);
        return if is_zero_ast(&w) { psi } else { c_mul(psi, C::OmegaPow(Box::new(conv_ord(&w)))) };
    }
    let inner = C::Psi(Some(Box::new(vc.clone())), Box::new(sigma(&merged)));
    let mut w_parts: Vec<Ast> = Vec::new();
    let mut arg_parts: Vec<C> = vec![inner];
    for b in &blocks {
        if let Some((j, y, m)) = as_psi_card_block(b, &sub_idx, s) {
            let base_j = C::Psi(Some(Box::new(vc.clone())), Box::new(sigma(&j)));
            let e = e_val_level(&sub_idx, s, &y);
            arg_parts.push(c_mul(base_j, c_mul(e, conv_ord(&m))));
        } else if is_below_omega1(b) {
            w_parts.push(b.clone());
        } else {
            let (h, m) = split_head_mult(b);
            let succ_fold = match &h {
                Some(Head::Cardinal(s2)) => {
                    is_successor_ord(s2)
                        && !matches!(s2, Ast::Num(1))
                        && as_nat(s2).map_or(true, |n| n >= 2)
                        && !contains_psi_ast(&m)
                }
                _ => false,
            };
            if succ_fold {
                let s2 = match &h { Some(Head::Cardinal(s2)) => s2.clone(), _ => unreachable!() };
                let mut parts = arg_parts.clone();
                parts.push(conv_ord(&m));
                arg_parts.push(C::Psi(
                    Some(Box::new(conv_ord(&pred_ord(&s2)))),
                    Box::new(c_sum(parts)),
                ));
            } else {
                // Non-fold cardinal tails (e.g. Ω_2^ω) accumulate in order so
                // a later succ_fold ψ nests them (ψ(Ω_3+Ω_2^ω+Ω_2) →
                // ψ(ψ_2(0)+Ω_2^ω+ψ_1(ψ_2(0)+Ω_2^ω+1))).
                arg_parts.push(conv_ord(&translate_down(b)));
            }
        }
    }
    let parts: Vec<C> = arg_parts;
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
        Ast::Psi(Some(sub), arg, ..) if ast_eq(sub, v_ast) => {
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

/// Like as_psi_card_block but the argument leads with Ω_s^e (a CardinalPow)
/// instead of Ω_s. Returns (lead_pow_ast, j, y, m) with the block equal to
/// ψ_v(Ω_s^e·j + y)·m.
fn as_psi_cardpow_block(b: &Ast, v_ast: &Ast, s_ast: &Ast) -> Option<(Ast, Ast, Ast, Ast)> {
    let (inner, m) = match b {
        Ast::Mul(p, k) => ((**p).clone(), (**k).clone()),
        _ => (b.clone(), Ast::Num(1)),
    };
    match &inner {
        Ast::Psi(Some(sub), arg, ..) if ast_eq(sub, v_ast) => {
            let mut ablocks = Vec::new();
            flatten_add(arg, &mut ablocks);
            if ablocks.is_empty() {
                return None;
            }
            let (h, j) = split_head_mult(&ablocks[0]);
            match h {
                Some(Head::CardinalPow(s2, e)) if ast_eq(&s2, s_ast) => {
                    let lead = Ast::Pow(
                        Box::new(Ast::Omega(Some(Box::new(s2)), None)),
                        Box::new(e),
                    );
                    Some((lead, j, sum_of(&ablocks[1..]), m))
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
    if !matches!(v_ast, Ast::Num(_)) {
        // Limit levels (ψ_ω, …): full conversion, then ω^ normalization
        // (ω^{Ω_ω²} → Ω_ω^{Ω_ω}, ω^{ψ-block} absorbs to the ψ-block).
        return normalize_omegapow(conv_ord(y));
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
        Ast::Omega(Some(x), ..) if ast_eq(x, s_ast) => {
            C::Psi(Some(Box::new(conv_ord(v_ast))), Box::new(C::Zero))
        }
        Ast::Psi(sub, arg, ..) => C::Psi(
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
/// Ω_{u+1}·ψ_u(X) → ψ_u(T(X) + ψ_u(T(X))) (rows 1108-style tails).
fn collapse_card_mul_psi(b: &Ast) -> Option<C> {
    if let Ast::Mul(p, k) = b {
        if let Ast::Omega(Some(idx), ..) = p.as_ref() {
            if is_successor_ord(idx) {
                let u = pred_ord(idx);
                if let Ast::Psi(Some(sub), x, ..) = k.as_ref() {
                    if ast_eq(sub, &u) {
                        let tx = translate_down(x);
                        let inner = conv_psi(Some(&u), &tx, None);
                        let argc = conv_ord(&tx);
                        return Some(C::Psi(
                            Some(Box::new(conv_ord(&u))),
                            Box::new(c_sum(vec![argc, inner])),
                        ));
                    }
                }
            }
        }
    }
    None
}

fn collapse_fixed(lead: C, mult: &Ast, tail: &Ast) -> C {
    collapse_fixed_kind(lead, mult, tail, false)
}

fn collapse_fixed_kind(lead: C, mult: &Ast, tail: &Ast, limit_card_pow: bool) -> C {
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
        let (h, m) = split_head_mult(b);
        if limit_card_pow {
            if let Some(Head::Cardinal(s2)) = &h {
                if is_successor_ord(s2) && !matches!(s2, Ast::Num(1)) && as_nat(s2).map_or(true, |n| n >= 2) {
                    // Ω_{u+1}·Y tail → ψ_u(lead + preceding + conv(Y))
                    let mut parts = vec![lead.clone()];
                    parts.extend(x_c.iter().cloned());
                    parts.push(conv_ord(&m));
                    x_c.push(C::Psi(
                        Some(Box::new(conv_ord(&pred_ord(s2)))),
                        Box::new(c_sum(parts)),
                    ));
                    continue;
                }
            }
        }
        let is_psi_block = match b {
            Ast::Psi(..) => true,
            Ast::Mul(p, _) if matches!(p.as_ref(), Ast::Psi(..)) => true,
            _ => false,
        };
        let tb = translate_down(b);
        // A ψ anywhere in the block (e.g. inside a cardinal-power exponent,
        // Ω_2^{ψ_1(Ω_2)}) is collapsed, not kept symbolic:
        // ψ(Ω_2^Ω_2+Ω_2^{ψ_1(Ω_2)}) → ψ(Ω_2^Ω_2+Ω_2^{ψ_1(0)}).
        x_c.push(if is_psi_block || contains_psi_ast(b) {
            conv_ord(&tb)
        } else {
            conv_sym(&tb)
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
/// Flatten a power lead: T(Ω_s^e·k) = Ω_s·(e·k) for finite e; T(Ω_s·k) = Ω_s·k.
/// Used for ψ_s(0)-tails under same-subscript limit leads (rows 869-875, 942).
fn tail_lead_flat(s: &Ast, lead: &C) -> C {
    let es = conv_ord(s);
    match lead {
        C::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
            (C::OmegaSub(x), C::Nat(n)) if render(x) == render(&es) => {
                C::Mul(Box::new(C::OmegaSub(x.clone())), Box::new(C::Nat(*n)))
            }
            _ => lead.clone(),
        },
        C::Mul(a, b) => {
            let fa = tail_lead_flat(s, a);
            c_mul(fa, (**b).clone())
        }
        _ => lead.clone(),
    }
}

/// F(β) for a ψ_s-tail argument β ≥ Ω_s under lead L (rows 877, 945, 960):
/// β = Ω_s^e·m (e ≥ 2) stays ψ_s(β); β = Ω_s·j + n folds as
/// ψ_s(L+1) (n = 1) or ψ_{s+1}(L+n−1) (n ≥ 2).
fn fold_tail_arg(s: &Ast, lead: &C, x: &Ast) -> C {
    let mut bs = Vec::new();
    flatten_add(x, &mut bs);
    if bs.is_empty() {
        return conv_ord(x);
    }
    let (h0, _) = split_head_mult(&bs[0]);
    let lead_pow = matches!(&h0, Some(Head::CardinalPow(t, e)) if ast_eq(t, s) && as_nat(e).map_or(false, |n| n >= 2));
    if lead_pow {
        return C::Psi(Some(Box::new(conv_ord(s))), Box::new(conv_ord(x)));
    }
    if !matches!(&h0, Some(Head::Cardinal(t)) if ast_eq(t, s)) {
        return conv_ord(x);
    }
    let mut idx = 1usize;
    while idx < bs.len() {
        let (h2, _) = split_head_mult(&bs[idx]);
        if matches!(&h2, Some(Head::Cardinal(t)) if ast_eq(t, s)) {
            idx += 1;
        } else {
            break;
        }
    }
    let smalls = &bs[idx..];
    let mut total = 0i32;
    let mut all_nat = true;
    for b in smalls {
        if let Ast::Num(n) = b { total += n; } else { all_nat = false; break; }
    }
    if !all_nat || total == 0 {
        return C::Psi(Some(Box::new(conv_ord(s))), Box::new(conv_ord(x)));
    }
    if total == 1 {
        C::Psi(
            Some(Box::new(conv_ord(s))),
            Box::new(c_sum(vec![lead.clone(), C::Nat(1)])),
        )
    } else {
        let s1 = Ast::Add(Box::new(s.clone()), Box::new(Ast::Num(1)));
        C::Psi(
            Some(Box::new(conv_ord(&s1))),
            Box::new(c_sum(vec![lead.clone(), c_nat(total - 1)])),
        )
    }
}

fn collapse_fixed_cardinal(s: &Ast, mult: &Ast, tail: &Ast) -> C {
    let leadc = C::OmegaSub(Box::new(conv_ord(s)));
    let lead = c_mul(leadc.clone(), conv_ord(mult));
    let mut blocks = Vec::new();
    flatten_add(tail, &mut blocks);
    let mut x_c: Vec<C> = Vec::new();
    let mut w_parts: Vec<Ast> = Vec::new();
    let mut i = 0usize;
    while i < blocks.len() {
        let b = &blocks[i];
        if is_below_omega1(b) {
            w_parts.push(b.clone());
            i += 1;
            continue;
        }
        // ψ_s(x) tail (same subscript as the limit lead):
        // → ψ_s(T(lead))·ω^x when x < Ω_s; ψ_s(lead + F(x)) otherwise.
        let psi_tail = match b {
            Ast::Psi(Some(u), x, ..) if ast_eq(u, s) => Some(((**x).clone(), None)),
            Ast::Pow(p, e) => match p.as_ref() {
                Ast::Psi(Some(u), x, ..) if ast_eq(u, s) => Some(((**x).clone(), Some((**e).clone()))),
                _ => None,
            },
            _ => None,
        };
        if let Some((x, epow)) = psi_tail {
            let flat = tail_lead_flat(s, &lead);
            let base = C::Psi(Some(Box::new(conv_ord(s))), Box::new(flat));
            let base = match epow {
                Some(e) => C::Pow(Box::new(base), Box::new(conv_ord(&e))),
                None => base,
            };
            let contrib = if below_omega_idx(s, &x) {
                if is_zero_ast(&x) {
                    base
                } else {
                    c_mul(base, normalize_omegapow(conv_ord(&x)))
                }
            } else {
                let f = fold_tail_arg(s, &lead, &x);
                C::Psi(
                    Some(Box::new(conv_ord(s))),
                    Box::new(c_sum(vec![lead.clone(), f])),
                )
            };
            x_c.push(contrib);
            i += 1;
            continue;
        }
        let (h, m) = split_head_mult(b);
        // Ω_s·j tail with small rest: folds into ψ_s (rows 876, 885).
        if matches!(&h, Some(Head::Cardinal(t)) if ast_eq(t, s)) {
            let mut j = i + 1;
            while j < blocks.len() {
                let (hj, _) = split_head_mult(&blocks[j]);
                if matches!(&hj, Some(Head::Cardinal(t)) if ast_eq(t, s)) {
                    j += 1;
                } else {
                    break;
                }
            }
            let mut smalls: Vec<Ast> = Vec::new();
            let mut k = j;
            while k < blocks.len() && is_below_omega1(&blocks[k]) {
                smalls.push(blocks[k].clone());
                k += 1;
            }
            let mut total = 0i32;
            let mut all_nat = true;
            for sb in &smalls {
                if let Ast::Num(n) = sb { total += n; } else { all_nat = false; break; }
            }
            let (contrib, consumed) = if all_nat && total == 1 {
                (C::Psi(
                    Some(Box::new(conv_ord(s))),
                    Box::new(c_sum(vec![lead.clone(), C::Nat(1)])),
                ), true)
            } else if all_nat && total >= 2 {
                let s1 = Ast::Add(Box::new(s.clone()), Box::new(Ast::Num(1)));
                (C::Psi(
                    Some(Box::new(conv_ord(&s1))),
                    Box::new(c_sum(vec![lead.clone(), c_nat(total - 1)])),
                ), true)
            } else {
                (conv_ord(&translate_down(b)), false)
            };
            x_c.push(contrib);
            i = if consumed { k } else { i + 1 };
            continue;
        }
        match &h {
            Some(Head::Cardinal(s2)) if is_successor_ord(s2) && !matches!(s2, Ast::Num(1)) && as_nat(s2).map_or(true, |n| n >= 2) => {
                // Ω_{u+1}·Y tail → ψ_u(lead + preceding + conv(Y))
                let mut parts = vec![lead.clone()];
                parts.extend(x_c.iter().cloned());
                parts.push(conv_ord(&m));
                x_c.push(C::Psi(
                    Some(Box::new(conv_ord(&pred_ord(s2)))),
                    Box::new(c_sum(parts)),
                ));
            }
            _ => {
                x_c.push(conv_ord(&translate_down(b)));
            }
        }
        i += 1;
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
        Ast::Psi(None, arg, ..)
            if !matches!(arg.as_ref(), Ast::Num(0)) && is_below_omega1(arg) =>
        {
            conv_psi0(arg)
        }
        Ast::Num(k) => c_nat(*k),
        Ast::W => c_omega(),
        Ast::Omega(None, ..) => C::Omega,
        Ast::Omega(Some(s), ..) => C::OmegaSub(Box::new(conv_sym(s))),
        Ast::Add(_, _) => {
            let mut blocks = Vec::new();
            flatten_add(a, &mut blocks);
            c_sum(blocks.iter().map(conv_sym).collect())
        }
        Ast::Mul(l, r) => c_mul(conv_sym(l), conv_sym(r)),
        Ast::Pow(b, e) => C::Pow(Box::new(conv_sym(b)), Box::new(conv_sym(e))),
        Ast::Psi(sub, arg, ..) => C::Psi(
            sub.as_ref().map(|s| Box::new(conv_sym(s))),
            Box::new(conv_sym(arg)),
        ),
    }
}

fn ast_eq(a: &Ast, b: &Ast) -> bool {
    match (a, b) {
        (Ast::Num(x), Ast::Num(y)) => x == y,
        (Ast::W, Ast::W) => true,
        (Ast::Omega(sa, _), Ast::Omega(sb, _)) => match (sa, sb) {
            (None, None) => true,
            (Some(x), Some(y)) => ast_eq(x, y),
            _ => false,
        },
        (Ast::Psi(sa, aa, _), Ast::Psi(sb, ab, _)) => {
            let se = match (sa, sb) {
                (None, None) => true,
                (Some(x), Some(y)) => ast_eq(x, y),
                _ => false,
            };
            se && ast_eq(aa, ab)
        }
        (Ast::Add(l1, r1), Ast::Add(l2, r2))
        | (Ast::Mul(l1, r1), Ast::Mul(l2, r2))
        | (Ast::Pow(l1, r1), Ast::Pow(l2, r2)) => ast_eq(l1, l2) && ast_eq(r1, r2),
        _ => false,
    }
}

/// Deep translation inside a fixed-point argument: a bare trailing Ω becomes
/// 1; ψ-blocks recurse into their arguments; everything else is kept.
fn translate_deep(a: &Ast, _f: &Ast) -> Ast {
    match a {
        Ast::Add(l, r) => {
            if matches!(r.as_ref(), Ast::Omega(None, ..)) {
                Ast::Add(l.clone(), Box::new(Ast::Num(1)))
            } else {
                Ast::Add(l.clone(), Box::new(translate_deep(r, _f)))
            }
        }
        Ast::Psi(sub, arg, ..) => Ast::Psi(sub.clone(), Box::new(translate_deep(arg, _f)), None),
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
            let shifted = matches!(last, Ast::Omega(None, ..)) && is_fixed_block(&blocks[0]);
            let mut out: Vec<Ast> = blocks.clone();
            if shifted {
                let n = out.len();
                out[n - 1] = Ast::Num(1);
            }
            sum_of(&out)
        }
        Ast::Pow(b, e) if matches!(b.as_ref(), Ast::Omega(None, ..)) => {
            Ast::Pow(b.clone(), Box::new(exp_shift(e)))
        }
        _ => a.clone(),
    }
}

/// Convert the exponent of a fixed-point lead Ω^e: a top-level ψ is fully
/// evaluated (rows 196-203); anything else is kept symbolically, with a
/// trailing bare Ω absorbed to 1 behind a fixed-point head (row 323).
fn conv_exp(e: &Ast) -> C {
    if let Ast::Psi(None, arg, ..) = e {
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
            Ast::Pow(Box::new(Ast::Omega(None, None)), Box::new(Ast::Num(n - 1)))
        };
    }
    Ast::Pow(Box::new(Ast::Omega(None, None)), Box::new(translate_deep(ee, f)))
}

/// ψ₀(Ω^λ·k + r) with λ a limit: the argument is kept symbolically; tails
/// Ω^e shift down one level and Ω^{ψ(F)} collapses to ψ(F).
fn collapse_fixed_omegapow(e: &Ast, mult: &Ast, tail: &Ast) -> C {
    let f_ast = Ast::Pow(Box::new(Ast::Omega(None, None)), Box::new(e.clone()));
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
                let is_psi_f = matches!(ee, Ast::Psi(None, ref inner, ..) if ast_eq(inner.as_ref(), &f_ast));
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
        // l·k ↦ l·(k−1); in particular l·2 ↦ l and the degenerate l·1 ↦ l.
        Ast::Mul(l, r) => {
            let rp = pred_ord(r);
            if matches!(&rp, Ast::Num(0)) || matches!(&rp, Ast::Num(1)) {
                (**l).clone()
            } else {
                Ast::Mul(l.clone(), Box::new(rp))
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
                // Nested products: (ψ·c₁)·c₂ = ψ·(c₁·c₂) (left absorption).
                _ => split_block_coeff(a)
                    .map(|(base, coeff)| (base, c_mul(coeff, (**b).clone()))),
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
                                let mc = match (&lcoeff, &coeff) {
                                    (C::Nat(1), big) if !is_below_c(big) => big.clone(),
                                    _ => c_add_ord(lcoeff, coeff),
                                };
                                // A Sum coefficient would render without
                                // parentheses inside the product; keep the
                                // summands separate instead.
                                if !matches!(mc, C::Sum(_)) {
                                    *last = rebuild_coeff(lbase, mc);
                                    merged = true;
                                }
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
            let prod = merge_product(na, nb);
            // ψ_v(x)·β with β ≥ Ω_{v+1}: absorb β into the argument.
            let mut fs = Vec::new();
            flatten_product(&prod, &mut fs);
            if fs.len() > 1 {
                if let Some(C::Psi(Some(v), x)) = fs.first() {
                    let mut acc = C::One;
                    for f in fs[1..].iter() {
                        acc = merge_product(acc, f.clone());
                    }
                    if big_factor(&acc, v) {
                        return C::Psi(
                            Some(v.clone()),
                            Box::new(normalize(c_sum(vec![(**x).clone(), acc]))),
                        );
                    }
                }
            }
            prod
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

/// Extract a cardinal factor Ω_s from lead, returning (Ω_s, cof) with
/// lead = Ω_s·cof. Used for ω^{Ω_s·cof} = Ω_s^cof (a cardinal is a fixed
/// point of ω^). Also handles a power of a cardinal Ω_s^a:
/// ω^{Ω_s^a} = Ω_s^{Ω_s^{a-1}} (finite a ≥ 1), Ω_s^{Ω_s^a} (limit a).
fn extract_cardinal_factor(lead: &C) -> Option<(C, C)> {
    let mut factors: Vec<C> = Vec::new();
    flatten_product(lead, &mut factors);
    let idx = factors.iter().position(|f| {
        matches!(f, C::Omega | C::OmegaSub(_))
            || matches!(f, C::Pow(b, _) if matches!(b.as_ref(), C::Omega | C::OmegaSub(_)))
    })?;
    let f = factors.remove(idx);
    let cof_rest = product_of(&factors);
    match f {
        C::Omega | C::OmegaSub(_) => Some((f, cof_rest)),
        C::Pow(b, a) => {
            let base = (*b).clone();
            let exp = match a.as_ref() {
                C::Nat(n) if *n >= 1 => {
                    if *n == 1 {
                        C::One
                    } else {
                        C::Pow(b.clone(), Box::new(c_nat(n - 1)))
                    }
                }
                _ => C::Pow(b.clone(), a.clone()), // limit a: cof = Ω_s^a
            };
            Some((base, c_mul(exp, cof_rest)))
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
    } else if let Some((card, cof)) = extract_cardinal_factor(lead) {
        // Ω_s is a fixed point of ω^: ω^{Ω_s·cof + rest} = Ω_s^cof · ω^{rest}.
        let card_pow = if matches!(&cof, C::One) || matches!(&cof, C::Nat(1)) {
            card
        } else {
            C::Pow(Box::new(card), Box::new(cof))
        };
        let rest_sum = c_sum(rest);
        if is_c_zero(&rest_sum) {
            normalize(card_pow)
        } else {
            normalize(c_mul(card_pow, C::OmegaPow(Box::new(rest_sum))))
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

/// True if a subscript value s is strictly above v (v a natural).
fn level_gt(v: &C, s: &C) -> bool {
    match (v, s) {
        (C::Nat(a), C::Nat(b)) => b > a,
        (C::Nat(_), _) => true,
        _ => false,
    }
}

/// True if beta >= Ω_{v+1} (conservatively, by its leading factor), which
/// lets ψ_v(x)·beta absorb into the ψ_v-argument.
fn big_factor(beta: &C, v: &C) -> bool {
    match beta {
        C::OmegaSub(s) => level_gt(v, s),
        C::Psi(Some(u), _) => level_gt(v, u),
        C::Mul(a, _) => big_factor(a, v),
        C::Pow(b, _) => big_factor(b, v),
        _ => false,
    }
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
            // ψ_v(x)·β with β ≥ Ω_{v+1}: absorb β into the argument
            // (ψ_1(ψ_ω(Ω))·ψ_ω(Ω) → ψ_1(ψ_ω(Ω)+ψ_ω(Ω))).
            if let Some(C::Psi(Some(v), x)) = fs.first() {
                if fs.len() > 1 {
                    let rest: Vec<C> = fs[1..].to_vec();
                    let mut acc = C::One;
                    for f in rest { acc = merge_product(acc, f); }
                    if big_factor(&acc, v) {
                        return C::Psi(
                            Some(v.clone()),
                            Box::new(c_sum(vec![(**x).clone(), acc])),
                        );
                    }
                }
            }
            normalize_product(fs)
        }
        C::Sum(terms) => {
            let mut out: Vec<C> = Vec::new();
            for t in terms {
                let nt = mocf_normalize_once(t);
                // a + a·β → a·β when β is infinite (a absorbs).
                if let Some(prev) = out.last() {
                    let (lead_r, infinite) = match &nt {
                        C::Mul(x, k) => (render(x), !matches!(k.as_ref(), C::Nat(_))),
                        C::Pow(x, k) => (render(x), !matches!(k.as_ref(), C::Nat(_))),
                        _ => (String::new(), false),
                    };
                    if infinite && !lead_r.is_empty() && render(prev) == lead_r {
                        out.pop();
                    }
                }
                absorb_small_before(&mut out, &nt);
                out.push(nt);
            }
            // Merge consecutive equal terms: x + x → x·2.
            let mut merged: Vec<C> = Vec::new();
            for t in out {
                let r = render(&t);
                if let Some(last) = merged.last() {
                    let (base_r, cnt) = match last {
                        C::Mul(x, k) => match k.as_ref() {
                            C::Nat(n) => (render(x), *n),
                            _ => (String::new(), 0),
                        },
                        other => (render(other), 1),
                    };
                    if cnt > 0 && base_r == r {
                        let base = match last {
                            C::Mul(x, _) => (**x).clone(),
                            other => other.clone(),
                        };
                        let n = merged.len();
                        merged[n - 1] = C::Mul(Box::new(base), Box::new(c_nat(cnt + 1)));
                        continue;
                    }
                }
                merged.push(t);
            }
            c_sum(merged)
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

/// A cardinal is a fixed point of ω^: ω^{Ω_s} = Ω_s and
/// ω^{Ω_s^a} = Ω_s^{Ω_s^{a-1}} (finite a ≥ 1), Ω_s^{Ω_s^a} (limit a).
fn as_cardinal_fixed_block(b: &C) -> Option<(C, PsiCoef)> {
    match b {
        C::Omega | C::OmegaSub(_) => Some((b.clone(), PsiCoef::One)),
        C::Pow(p, j) if matches!(p.as_ref(), C::Omega | C::OmegaSub(_)) => match j.as_ref() {
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
        let fixed = as_psi_fixed_block(&b).or_else(|| as_cardinal_fixed_block(&b));
        if let Some((base, coef)) = fixed {
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

fn contains_psi_ast(a: &Ast) -> bool {
    match a {
        Ast::Psi(..) => true,
        Ast::Add(l, r) | Ast::Mul(l, r) | Ast::Pow(l, r) => {
            contains_psi_ast(l) || contains_psi_ast(r)
        }
        _ => false,
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
    fn zz_probe_sub() {
        let ast = crate::parser::parse_bocf("ψ_(ω+1)(Ω_(ω+2)+1)").unwrap();
        let t = crate::parser::eval_ast(&ast).unwrap();
        let s = crate::term::standard_form(&t);
        let rt = term_to_ast(&s);
        println!("PROBE parsed = {:?}", ast);
        println!("PROBE roundtrip = {:?}", rt);
        if let Ast::Psi(Some(v), arg, ..) = &rt {
            println!("PROBE v = {:?}", v);
            let mut blocks = Vec::new();
            flatten_add(arg, &mut blocks);
            let (h, _m) = split_head_mult(&blocks[0]);
            if let Some(Head::Cardinal(sa)) = h {
                println!("PROBE s_ast = {:?}, pred = {:?}", sa, pred_ord(&sa));
                println!("PROBE ast_eq = {}", ast_eq(&pred_ord(&sa), v));
            } else {
                println!("PROBE head = {:?}", h);
            }
        }
    }

    #[test]
    fn psi_subscript_bare_number() {
        // ψ1(… ≡ ψ_1(…: a bare number right after ψ is the subscript.
        assert_eq!(conv("ψ1(Ω_2)"), conv("ψ_1(Ω_2)"));
        assert_eq!(conv("ψ2(Ω_3)"), conv("ψ_2(Ω_3)"));
        assert_eq!(conv("ψ12(Ω_13)"), conv("ψ_12(Ω_13)"));
        assert_eq!(conv("p1(Ω_2+1)"), conv("ψ_1(Ω_2+1)"));
        assert_eq!(conv("\\psi1(Ω_2)"), conv("ψ_1(Ω_2)"));
        assert_eq!(conv("ψ(Ω_2^2+ψ1(Ω_2))"), conv("ψ(Ω_2^2+ψ_1(Ω_2))"));
        // ψω(… ≡ ψ_ω(…: a bare ω right after ψ is the subscript.
        assert_eq!(conv("ψω(1)"), conv("ψ_ω(1)"));
        assert_eq!(conv("ψω(Ω_(ω+1))"), conv("ψ_ω(Ω_(ω+1))"));
        // ψ(σ)(α) ≡ ψ_σ(α): a parenthesized subscript before the argument.
        assert_eq!(conv("ψ(ω+1)(0)"), conv("ψ_(ω+1)(0)"));
        assert_eq!(conv("ψ(1)(Ω_2)"), conv("ψ_1(Ω_2)"));
        // A single group is still the argument.
        assert_eq!(conv("ψ(ω+1)"), conv("ψ(ω+1)"));
        // Ω2 ≡ Ω×2, ω2 ≡ ω×2: a bare number after Ω/ω is a multiplier.
        assert_eq!(conv("ψ(Ω2)"), conv("ψ(Ω×2)"));
        assert_eq!(conv("ψ(ω2)"), conv("ψ(ω×2)"));
        assert_eq!(conv("ψ(Ω_2^2+Ω2)"), conv("ψ(Ω_2^2+Ω×2)"));
        assert_eq!(conv("ψ(W2)"), conv("ψ(Ω×2)"));
    }

    #[test]
    fn top_level_psi_next_cardinal_tails() {
        // ψ_v(Ω_{v+1} + small) → ψ_v(0)·ω^{small}
        assert_eq!(conv("ψ_1(Ω_2+1)"), "\\psi_{1}(0)\\omega");
        assert_eq!(conv("ψ_1(Ω_2+2)"), "\\psi_{1}(0)\\omega^{2}");
        assert_eq!(conv("ψ_1(Ω_2+ω)"), "\\psi_{1}(0)\\omega^{\\omega}");
        // ε₀ tail ψ(Ω) collapses to ψ(0): ω^{ψ(0)} absorbs to ψ(0)
        assert_eq!(conv("ψ_1(Ω_2+ψ(Ω))"), "\\psi_{1}(0)\\psi(0)");
        // Ω-power tails stay as factors
        assert_eq!(conv("ψ_1(Ω_2+Ω)"), "\\psi_{1}(0)\\Omega");
        assert_eq!(conv("ψ_1(Ω_2+Ω^2)"), "\\psi_{1}(0)\\Omega^{2}");
        assert_eq!(conv("ψ_1(Ω_2+Ω^ω)"), "\\psi_{1}(0)\\Omega^{\\omega}");
        // ψ_v(Ω_{v+1}) tail collapses recursively
        assert_eq!(conv("ψ_1(Ω_2+ψ_1(Ω_2))"), "\\psi_{1}(0)^{2}");
        // ψ_v-block tails: exponent machinery → powers of ψ_v(0)
        assert_eq!(conv("ψ_1(Ω_2+ψ_1(Ω_2)×2)"), "\\psi_{1}(0)^{3}");
        assert_eq!(conv("ψ_1(Ω_2+ψ_1(Ω_2+1))"), "\\psi_{1}(0)^{\\omega}");
        // Ω_{v+1}·k with no tail → ψ_v(σ(k))
        assert_eq!(conv("ψ_1(Ω_2×2)"), "\\psi_{1}(1)");
        assert_eq!(conv("ψ_1(Ω_2×3)"), "\\psi_{1}(2)");
        assert_eq!(conv("ψ_1(Ω_2×ω)"), "\\psi_{1}(\\omega)");
        // Ω_{v+1}·k (k ≥ 2) with ψ_v-block tails: tail becomes a factor
        // (ψ_1(Ω_2×2+ψ_1(Ω_2)) → ψ_1(1)·ψ_1(0))
        assert_eq!(conv("ψ_1(Ω_2×2+ψ_1(Ω_2))"), "\\psi_{1}(1)\\psi_{1}(0)");
        assert_eq!(conv("ψ_1(Ω_2×2+ψ_1(Ω_2)×2)"), "\\psi_{1}(1)\\psi_{1}(0)^{2}");
        assert_eq!(conv("ψ_1(Ω_2×2+ψ_1(Ω_2+1))"), "\\psi_{1}(1)\\psi_{1}(0)^{\\omega}");
        assert_eq!(conv("ψ_1(Ω_2×3+ψ_1(Ω_2))"), "\\psi_{1}(2)\\psi_{1}(0)");
        // Ω_{v+1}^2 lead with Ω_{v+1} tail: exponent lowers, tail folds +1
        assert_eq!(conv("ψ_1(Ω_2^2+Ω_2)"), "\\psi_{1}(\\Omega_{2} + 1)");
        assert_eq!(
            conv("ψ(Ω_2^2+Ω_2×ψ_1(Ω_2^2+Ω_2)+ψ_1(Ω_2^2+Ω_2))"),
            "\\psi(\\Omega_{2} + \\psi_{1}(\\Omega_{2} + \\psi_{1}(\\Omega_{2} + 1)) + \\psi_{1}(\\Omega_{2} + 1))"
        );
        // Ω_{v+1}^e leads: finite exponent lowers, limit stays
        assert_eq!(conv("ψ_1(Ω_2^2)"), "\\psi_{1}(\\Omega_{2})");
        assert_eq!(conv("ψ_1(Ω_2^ω)"), "\\psi_{1}(\\Omega_{2}^{\\omega})");
        // higher levels
        assert_eq!(conv("ψ_2(Ω_3+1)"), "\\psi_{2}(0)\\omega");
        assert_eq!(conv("ψ_3(Ω_4+ω)"), "\\psi_{3}(0)\\omega^{\\omega}");
        assert_eq!(conv("ψ_(ω+1)(Ω_(ω+2)+1)"), "\\psi_{\\omega + 1}(0)\\omega");
        // limit-cardinal lead: base ψ_v(Ω_ω) stays; tail cardinals fold into
        // the argument (collapse_fixed_cardinal pattern): Ω_{v+1} → 1,
        // Ω_s2 (s2>v+1) → ψ_{s2-1}(Ω_ω+…).
        assert_eq!(conv("ψ_1(Ω_ω)"), "\\psi_{1}(\\Omega_{\\omega})");
        assert_eq!(conv("ψ_1(Ω_ω+1)"), "\\psi_{1}(\\Omega_{\\omega})\\omega");
        assert_eq!(conv("ψ_1(Ω_ω+ω)"), "\\psi_{1}(\\Omega_{\\omega})\\omega^{\\omega}");
        assert_eq!(conv("ψ_1(Ω_ω+Ω)"), "\\psi_{1}(\\Omega_{\\omega})\\Omega");
        assert_eq!(conv("ψ_1(Ω_ω+Ω_2)"), "\\psi_{1}(\\Omega_{\\omega} + 1)");
        // successor tail above the collapse cardinal under a limit lead
        // collapses to ψ_{s2-1}(lead + m), lead = the limit lead
        // (ψ_1(Ω_ω+Ω_3) → ψ_1(Ω_ω+ψ_2(Ω_ω+1))).
        assert_eq!(
            conv("ψ_1(Ω_ω+Ω_3)"),
            "\\psi_{1}(\\Omega_{\\omega} + \\psi_{2}(\\Omega_{\\omega} + 1))"
        );
        assert_eq!(
            conv("ψ_1(Ω_ω+Ω_3×2)"),
            "\\psi_{1}(\\Omega_{\\omega} + \\psi_{2}(\\Omega_{\\omega} + 2))"
        );
        // successor lead above the collapse cardinal → ψ_{s-1}(0) as argument
        assert_eq!(conv("ψ_1(Ω_3+Ω)"), "\\psi_{1}(\\psi_{2}(0))\\Omega");
        assert_eq!(conv("ψ_1(Ω_(ω+1))"), "\\psi_{1}(\\psi_{\\omega}(0))");
        assert_eq!(conv("ψ_1(Ω_(ω+1)+Ω)"), "\\psi_{1}(\\psi_{\\omega}(0))\\Omega");
        // successor-cardinal powers above the collapse cardinal: exponent lowers
        assert_eq!(conv("ψ_1(Ω_(ω+1)^2)"), "\\psi_{1}(\\Omega_{\\omega + 1})");
        assert_eq!(conv("ψ_1(Ω_(ω+1)^3)"), "\\psi_{1}(\\Omega_{\\omega + 1}^{2})");
        assert_eq!(conv("ψ_1(Ω_(ω+1)^ω)"), "\\psi_{1}(\\Omega_{\\omega + 1}^{\\omega})");
        // ω^Ω simplifies to Ω (a cardinal is a fixed point of ω^)
        assert_eq!(conv("ψ_1(Ω_2+ψ_1(Ω_2+Ω))"), "\\psi_{1}(0)^{\\Omega}");
        assert_eq!(conv("ψ_1(Ω_2+ψ_1(Ω_2+Ω)×2)"), "\\psi_{1}(0)^{\\Omega2}");
        // general rule ψ_a(X+1) → ψ_a(X)·ω; limit-subscript cardinal powers
        assert_eq!(conv("ψ_1(Ω_ω^2+1)"), "\\psi_{1}(\\Omega_{\\omega}^{2})\\omega");
        assert_eq!(conv("ψ_1(Ω_ω^2+ω)"), "\\psi_{1}(\\Omega_{\\omega}^{2})\\omega^{\\omega}");
        assert_eq!(conv("ψ_1(Ω_ω^3+1)"), "\\psi_{1}(\\Omega_{\\omega}^{3})\\omega");
        assert_eq!(conv("ψ_1(Ω_ω^ω+1)"), "\\psi_{1}(\\Omega_{\\omega}^{\\omega})\\omega");
        assert_eq!(conv("ψ(Ω^2+1)"), "\\psi(\\Omega)\\omega");
        assert_eq!(conv("ψ_2(Ω_ω+1)"), "\\psi_{2}(\\Omega_{\\omega})\\omega");
        // ψ_v(Ω_s^e + ψ_v(Ω_s^e)) → ψ_v(Ω_s^e)^2 (CardinalPow-lead ψ-block tails)
        assert_eq!(conv("ψ_1(Ω_ω^2+ψ_1(Ω_ω^2))"), "\\psi_{1}(\\Omega_{\\omega}^{2})^{2}");
        assert_eq!(conv("ψ_1(Ω_ω^2+ψ_1(Ω_ω^2)×2)"), "\\psi_{1}(\\Omega_{\\omega}^{2})^{2}2");
        assert_eq!(conv("ψ_1(Ω_ω^2+ψ_1(Ω_ω^2+1))"), "\\psi_{1}(\\Omega_{\\omega}^{2})^{2}\\omega");
        assert_eq!(conv("ψ_1(Ω_ω+ψ_1(Ω_ω))"), "\\psi_{1}(\\Omega_{\\omega})^{2}");
    }

    #[test]
    fn cardinal_power_omega_exponent() {
        use super::C;
        let om2 = || C::OmegaSub(Box::new(C::Nat(2)));
        // ω^{Ω_2^2} → Ω_2^{Ω_2}
        let w = C::OmegaPow(Box::new(C::Pow(
            Box::new(om2()),
            Box::new(C::Nat(2)),
        )));
        assert_eq!(
            super::render(&super::mocf_normalize(&w)),
            "\\Omega_{2}^{\\Omega_{2}}"
        );
        // ω^{Ω_2^3} → Ω_2^{Ω_2^2}
        let w3 = C::OmegaPow(Box::new(C::Pow(
            Box::new(om2()),
            Box::new(C::Nat(3)),
        )));
        assert_eq!(
            super::render(&super::mocf_normalize(&w3)),
            "\\Omega_{2}^{\\Omega_{2}^{2}}"
        );
        // ω^{Ω_2^ω} → Ω_2^{Ω_2^ω} (limit exponent stays)
        let ww = C::OmegaPow(Box::new(C::Pow(
            Box::new(om2()),
            Box::new(C::OmegaPow(Box::new(C::One))),
        )));
        assert_eq!(
            super::render(&super::mocf_normalize(&ww)),
            "\\Omega_{2}^{\\Omega_{2}^{\\omega}}"
        );
    }

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
    fn omega2_tail_collapse_cardinal() {
        // A ψ_{s-1}(Ω_s) tail collapses its Ω_s lead: ψ_1(Ω_2) → ψ_1(0).
        assert_eq!(conv("ψ(Ω_2^2+ψ_1(Ω_2))"), "\\psi(\\Omega_{2} + \\psi_{1}(0))");
        assert_eq!(conv("ψ(Ω_2^3+ψ_1(Ω_2))"), "\\psi(\\Omega_{2}^{2} + \\psi_{1}(0))");
        // General rule ψ_v(X + n) = ψ_v(X)·ω^n, unconditionally.
        assert_eq!(conv("ψ_1(Ω_2^ω+1)"), "\\psi_{1}(\\Omega_{2}^{\\omega})\\omega");
        assert_eq!(conv("ψ_1(Ω_2^ω+2)"), "\\psi_{1}(\\Omega_{2}^{\\omega})\\omega^{2}");
        assert_eq!(conv("ψ_1(Ω_2^2+2)"), "\\psi_{1}(\\Omega_{2})\\omega^{2}");
        assert_eq!(conv("ψ_1(Ω_2+2)"), "\\psi_{1}(0)\\omega^{2}");
        assert_eq!(conv("ψ_1(Ω_2^ω+ω)"), "\\psi_{1}(\\Omega_{2}^{\\omega})\\omega^{\\omega}");
        // An Ω_s·ψ_{s-1} tail factor fully collapses its ψ-argument.
        assert_eq!(
            conv("ψ(Ω_2^2+Ω_2×ψ_1(Ω_2))"),
            "\\psi(\\Omega_{2} + \\psi_{1}(\\Omega_{2} + \\psi_{1}(0)))"
        );
        assert_eq!(
            conv("ψ(Ω_ω+ψ_1(Ω_2^ω+ω))"),
            "\\psi(\\Omega_{\\omega} + \\psi_{1}(\\Omega_{2}^{\\omega})\\omega^{\\omega})"
        );
        // Ω < Ω_{v+1} peels as ×ω^Ω = ×Ω.
        assert_eq!(conv("ψ_1(Ω_2^ω+Ω)"), "\\psi_{1}(\\Omega_{2}^{\\omega})\\Omega");
        // Under a limit lead in ψ_v (v ≥ 1), a tail cardinal above the
        // collapse cardinal stays whole in the argument (Ω_ω > Ω_2 cannot
        // be pulled out of ψ_1); Ω_{v+1} folds, its powers translate down.
        assert_eq!(conv("ψ_1(Ω_ω×Ω+Ω_ω)"), "\\psi_{1}(\\Omega_{\\omega}\\Omega + \\Omega_{\\omega})");
        assert_eq!(conv("ψ_1(Ω_ω×Ω+Ω_2)"), "\\psi_{1}(\\Omega_{\\omega}\\Omega + 1)");
        assert_eq!(conv("ψ_1(Ω_ω×Ω+Ω_2^2)"), "\\psi_{1}(\\Omega_{\\omega}\\Omega + \\Omega_{2})");
        // Ω_s-built tails under a limit-exponent lead translate down,
        // keeping the ψ_v subscript (corrected data rows 1243-1245).
        assert_eq!(conv("ψ_1(Ω_2^ω+Ω_2)"), "\\psi_{1}(\\Omega_{2}^{\\omega} + 1)");
        assert_eq!(conv("ψ_1(Ω_2^ω+Ω_2×2)"), "\\psi_{1}(\\Omega_{2}^{\\omega} + 2)");
        assert_eq!(conv("ψ_1(Ω_2^ω+Ω_2^2)"), "\\psi_{1}(\\Omega_{2}^{\\omega} + \\Omega_{2})");
        // Finite-exponent lowering also applies above the collapse cardinal;
        // Ω_s tails follow the ψ_0 logic (Ω_s·X → ψ_{s-1}(Ω_s+M(X))).
        assert_eq!(conv("ψ_1(Ω_3^2+Ω_3)"), "\\psi_{1}(\\Omega_{3} + \\psi_{2}(\\Omega_{3} + 1))");
        assert_eq!(
            conv("ψ_1(Ω_3^2+Ω_3+Ω_2)"),
            "\\psi_{1}(\\Omega_{3} + \\psi_{2}(\\Omega_{3} + 1) + 1)"
        );
        assert_eq!(conv("ψ_1(Ω_(ω+1)^2+Ω_(ω+1))"), "\\psi_{1}(\\Omega_{\\omega + 1} + \\psi_{\\omega}(\\Omega_{\\omega + 1} + 1))");
        // Ω_{s'} tail (s' ≠ s) → ψ_{s'-1}(lead + M(X)), both ψ_0 and ψ_v.
        assert_eq!(conv("ψ_1(Ω_4^2+Ω_3)"), "\\psi_{1}(\\Omega_{4} + \\psi_{2}(\\Omega_{4} + 1))");
        assert_eq!(conv("ψ(Ω_4^2+Ω_3)"), "\\psi(\\Omega_{4} + \\psi_{2}(\\Omega_{4} + 1))");
        assert_eq!(conv("ψ_1(Ω_4^2+Ω_3×2)"), "\\psi_{1}(\\Omega_{4} + \\psi_{2}(\\Omega_{4} + 2))");
        assert_eq!(conv("ψ(Ω_4^2+Ω_3×2)"), "\\psi(\\Omega_{4} + \\psi_{2}(\\Omega_{4} + 2))");
        // Exponent ≥ 3: the Ω_s tail uses the lowered lead Ω_s^{n-1}
        // (ψ(Ω_2^3+Ω_2) → ψ(Ω_2^2+ψ_1(Ω_2^2+1))).
        assert_eq!(conv("ψ(Ω_2^3+Ω_2)"), "\\psi(\\Omega_{2}^{2} + \\psi_{1}(\\Omega_{2}^{2} + 1))");
        assert_eq!(conv("ψ(Ω_2^3+Ω_2×2)"), "\\psi(\\Omega_{2}^{2} + \\psi_{1}(\\Omega_{2}^{2} + 2))");
        // is_above lead with an Ω_{v+1} tail folds to +k
        // (ψ_1(Ω_3+Ω_2) → ψ_1(ψ_2(0)+1)).
        assert_eq!(conv("ψ_1(Ω_3+Ω_2)"), "\\psi_{1}(\\psi_{2}(0) + 1)");
        assert_eq!(conv("ψ_1(Ω_3+Ω_2×2)"), "\\psi_{1}(\\psi_{2}(0) + 2)");
        assert_eq!(conv("ψ_1(Ω_4+Ω_2)"), "\\psi_{1}(\\psi_{3}(0) + 1)");
        assert_eq!(conv("ψ_1(Ω_3×2+Ω_2)"), "\\psi_{1}(\\psi_{2}(1) + 1)");
        // Finite-exponent lowering at ψ_v level (source rows 1084-1094).
        assert_eq!(conv("ψ_1(Ω_2^2+Ω_2)"), "\\psi_{1}(\\Omega_{2} + 1)");
        assert_eq!(conv("ψ_1(Ω_2^2×2)"), "\\psi_{1}(\\Omega_{2}2)");
        assert_eq!(
            conv("ψ_1(Ω_2^2+Ω_2×ψ_1(Ω_2^2))"),
            "\\psi_{1}(\\Omega_{2} + \\psi_{1}(\\Omega_{2}))"
        );
        // Ω_s·k lead: contiguous Ω_s tails merge into the multiplier.
        assert_eq!(
            conv("ψ_1(Ω_2×ψ_1(Ω_2)+Ω_2)"),
            "\\psi_{1}(\\psi_{1}(0) + 1)"
        );
        // Limit-subscript leads: Ω_{u+1}·Y tails fold as ψ_u(lead+preceding+Y),
        // Ω_{u+1}·ψ_u(X) becomes ψ_u(X′+ψ_u(X′)) (source rows 1095-1109).
        assert_eq!(
            conv("ψ(Ω_ω×Ω+Ω_2)"),
            "\\psi(\\Omega_{\\omega}\\Omega + \\psi_{1}(\\Omega_{\\omega}\\Omega + 1))"
        );
        assert_eq!(
            conv("ψ(Ω_ω×Ω_2+Ω_2×ψ_1(Ω_ω))"),
            "\\psi(\\Omega_{\\omega}\\Omega_{2} + \\psi_{1}(\\Omega_{\\omega}\\Omega_{2} + \\psi_{1}(\\Omega_{\\omega})))"
        );
        assert_eq!(
            conv("ψ(Ω_ω^2+Ω_2×ψ_1(Ω_ω^2))"),
            "\\psi(\\Omega_{\\omega}^{2} + \\psi_{1}(\\Omega_{\\omega}^{2} + \\psi_{1}(\\Omega_{\\omega}^{2})))"
        );
        // True-limit subscript powers keep their exponent under translate_down.
        assert_eq!(
            conv("ψ(Ω_(ω+1)+Ω_ω^2)"),
            "\\psi(\\psi_{\\omega}(0) + \\Omega_{\\omega}^{2})"
        );
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
        // Ω_{v+1}^e tails lower into the ψ_v argument instead of folding
        // to their multiplier.
        assert_eq!(conv("ψ_1(Ω_3+Ω_2^2)"), "\\psi_{1}(\\psi_{2}(0) + \\Omega_{2})");
        assert_eq!(conv("ψ_1(Ω_3^2+Ω_2^2)"), "\\psi_{1}(\\Omega_{3} + \\Omega_{2})");
        // Contiguous Ω_s·X tails merge into one ψ_{s-1} term; below-lead
        // power tails also contribute their lowering to the outer argument.
        assert_eq!(
            conv("ψ(Ω_2^3+Ω_2×(ω+1))"),
            "\\psi(\\Omega_{2}^{2} + \\psi_{1}(\\Omega_{2}^{2} + \\omega + 1))"
        );
        assert_eq!(
            conv("ψ(Ω_3^2+Ω_2^2+Ω_2)"),
            "\\psi(\\Omega_{3} + \\Omega_{2} + \\psi_{1}(\\Omega_{3} + \\Omega_{2} + 1))"
        );
        // Row 407 corrected: a natural-number lead folds an Ω_{s'} tail.
        assert_eq!(
            conv("ψ(Ω_3+Ω_2)"),
            "\\psi(\\psi_{2}(0) + \\psi_{1}(\\psi_{2}(0) + 1))"
        );
        // A multiplied power tail Ω_s^e×k lowers (Ω_s^{e-1}×k) into both the
        // outer argument and the ψ_{s-1} argument
        // (ψ(Ω_2^3+Ω_2^2×2+Ω_2) → ψ(Ω_2^2+Ω_2·2+ψ_1(Ω_2^2+Ω_2·2+1))).
        assert_eq!(
            conv("ψ(Ω_2^3+Ω_2^2×2+Ω_2)"),
            "\\psi(\\Omega_{2}^{2} + \\Omega_{2}2 + \\psi_{1}(\\Omega_{2}^{2} + \\Omega_{2}2 + 1))"
        );
        // A lone power tail lowers into the outer argument only, with no ψ
        // term (ψ(Ω_2^3+Ω_2^2) → ψ(Ω_2^2+Ω_2)).
        assert_eq!(conv("ψ(Ω_2^3+Ω_2^2)"), "\\psi(\\Omega_{2}^{2} + \\Omega_{2})");
        // A lower-subscript tail nests the previously collapsed ψ term into
        // its argument (ψ(Ω_3^2+Ω_3+Ω_2) →
        // ψ(Ω_3+ψ_2(Ω_3+1)+ψ_1(Ω_3+ψ_2(Ω_3+1)+1))).
        assert_eq!(
            conv("ψ(Ω_3^2+Ω_3+Ω_2)"),
            "\\psi(\\Omega_{3} + \\psi_{2}(\\Omega_{3} + 1) + \\psi_{1}(\\Omega_{3} + \\psi_{2}(\\Omega_{3} + 1) + 1))"
        );
        // ψ_0 succ_fold accumulates preceding cardinal tails into the ψ
        // argument (ψ(Ω_3+Ω_2^ω+Ω_2) →
        // ψ(ψ_2(0)+Ω_2^ω+ψ_1(ψ_2(0)+Ω_2^ω+1))).
        assert_eq!(
            conv("ψ(Ω_3+Ω_2^ω+Ω_2)"),
            "\\psi(\\psi_{2}(0) + \\Omega_{2}^{\\omega} + \\psi_{1}(\\psi_{2}(0) + \\Omega_{2}^{\\omega} + 1))"
        );
        // is_above limit-exponent power lead stays; Ω_{v+1}·m tail collapses
        // to ψ_v(lead + m) (ψ_1(Ω_3^ω+Ω_2) → ψ_1(Ω_3^ω+ψ_1(Ω_3^ω+1))).
        assert_eq!(
            conv("ψ_1(Ω_3^ω+Ω_2)"),
            "\\psi_{1}(\\Omega_{3}^{\\omega} + \\psi_{1}(\\Omega_{3}^{\\omega} + 1))"
        );
        // An above-collapse-cardinal tail Ω_{s'} (s' > v+1) collapses to
        // ψ_{s'-1}(collapsed_lead + m), where collapsed_lead = ψ_{s-1}(σ)
        // (ψ_1(Ω_4+Ω_3) → ψ_1(ψ_3(0)+ψ_2(ψ_3(0)+1))).
        assert_eq!(
            conv("ψ_1(Ω_4+Ω_3)"),
            "\\psi_{1}(\\psi_{3}(0) + \\psi_{2}(\\psi_{3}(0) + 1))"
        );
        assert_eq!(
            conv("ψ_1(Ω_4+Ω_3×2)"),
            "\\psi_{1}(\\psi_{3}(0) + \\psi_{2}(\\psi_{3}(0) + 2))"
        );
        // Row 380 corrected: a same-base bare cardinal tail under a
        // limit-exponent lead collapses to ψ_{s-1}(lead + 1).
        assert_eq!(
            conv("ψ(Ω_2^Ω_2+Ω_2)"),
            "\\psi(\\Omega_{2}^{\\Omega_{2}} + \\psi_{1}(\\Omega_{2}^{\\Omega_{2}} + 1))"
        );
        // Limit level v=ω: an above-collapse-cardinal finite-exponent lead
        // still lowers its exponent (is_above must hold for limit v).
        assert_eq!(conv("ψ_ω(Ω_(ω+2)^2)"), "\\psi_{\\omega}(\\Omega_{\\omega + 2})");
        // Limit-multiple subscripts compare correctly above the collapse
        // cardinal at limit levels (ψ_ω(Ω_{ω×2+1}) → ψ_ω(ψ_{ω×2}(0))),
        // while the λ·k lead itself stays (ψ(Ω_{ω×2}) row 864).
        assert_eq!(conv("ψ_ω(Ω_(ω×2+1))"), "\\psi_{\\omega}(\\psi_{\\omega2}(0))");
        assert_eq!(conv("ψ_ω(Ω_(ω×2))"), "\\psi_{\\omega}(\\Omega_{\\omega2})");
        assert_eq!(conv("ψ_ω(Ω_(ω^2+1))"), "\\psi_{\\omega}(\\psi_{\\omega^{2}}(0))");
        assert_eq!(conv("ψ_ω(Ω_(ω^2))"), "\\psi_{\\omega}(\\Omega_{\\omega^{2}})");
        // Term-layer comparison: ψ_4(Ω_5) > Ω_4 holds by value, not by
        // structural subscript comparison, so the lead collapses.
        assert_eq!(
            conv("ψ_Ω_4(Ω_(ψ_4(Ω_5)+1))"),
            "\\psi_{\\Omega_{4}}(\\psi_{\\psi_{4}(0)}(0))"
        );
        // Row 383 corrected: a ψ inside a cardinal-power tail exponent is
        // collapsed, not kept symbolic.
        assert_eq!(
            conv("ψ(Ω_2^Ω_2+Ω_2^ψ_1(Ω_2))"),
            "\\psi(\\Omega_{2}^{\\Omega_{2}} + \\Omega_{2}^{\\psi_{1}(0)})"
        );
    }
}

#[cfg(test)]
pub(crate) fn norm_mocf_latex(s: &str) -> String {
    // Replace product markers with spaces so token boundaries stay visible,
    // canonicalize Ω^e_s (source convention) to Ω_s^e, then strip
    // braces/whitespace.
    let t = s.replace("\\times", " ").replace("\\cdot", " ");
    let chars: Vec<char> = t.chars().collect();
    let n = chars.len();
    fn scan_token(chars: &[char], mut j: usize) -> usize {
        if j < chars.len() && chars[j] == '(' {
            let mut depth = 1;
            j += 1;
            while j < chars.len() && depth > 0 {
                if chars[j] == '(' { depth += 1; }
                if chars[j] == ')' { depth -= 1; }
                j += 1;
            }
            return j;
        }
        if j < chars.len() && chars[j] == '\\' {
            j += 1;
            while j < chars.len() && chars[j].is_ascii_alphabetic() { j += 1; }
            return j;
        }
        while j < chars.len() && chars[j].is_ascii_alphanumeric() { j += 1; }
        j
    }
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        if t[i..].starts_with("\\Omega^") {
            let j0 = i + "\\Omega^".len();
            let j1 = scan_token(&chars, j0);
            if j1 < n && chars[j1] == '_' {
                let k0 = j1 + 1;
                let k1 = scan_token(&chars, k0);
                out.push_str("\\Omega_");
                out.push_str(&t[k0..k1]);
                out.push('^');
                out.push_str(&t[j0..j1]);
                i = k1;
                continue;
            }
            out.push_str(&t[i..j1]);
            i = j1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out.chars().filter(|c| !matches!(c, '{' | '}' | ' ' | '\t')).collect()
}
#[cfg(test)]
mod datagen {
    use super::norm_mocf_latex;
    use crate::bms::bms_to_bocf;

    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_q = false;
        for c in line.chars() {
            if c == '"' { in_q = !in_q; }
            else if c == ',' && !in_q { out.push(cur.clone()); cur.clear(); }
            else { cur.push(c); }
        }
        out.push(cur);
        out
    }

    fn parse_matrix(s: &str) -> Vec<Vec<i32>> {
        let mut m = Vec::new();
        for part in s.split(')') {
            let p = part.trim_start_matches('(').trim();
            if p.is_empty() { continue; }
            let row: Vec<i32> = p.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if !row.is_empty() { m.push(row); }
        }
        m
    }

    pub(crate) fn latex_to_unicode(s: &str) -> String {
        insert_mul(&latex_symbols(s))
    }

    fn latex_symbols(s: &str) -> String {
        let mut r = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_alphabetic() { j += 1; }
                let cmd: String = chars[i + 1..j].iter().collect();
                match cmd.as_str() {
                    "psi" => r.push('ψ'),
                    "Omega" => r.push('Ω'),
                    "omega" => r.push('ω'),
                    "left" | "right" => {}
                    _ => r.push_str(&cmd),
                }
                i = j;
            } else {
                r.push(chars[i]);
                i += 1;
            }
        }
        r
    }

    /// Insert × for the renderer's implicit-multiplication juxtaposition.
    fn insert_mul(s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let starts_factor = |c: char| {
            c == 'ψ' || c == 'Ω' || c == 'ω' || c == '(' || c.is_ascii_digit()
        };
        let mut out = String::new();
        let mut i = 0;
        while i < n {
            let c = chars[i];
            if starts_factor(c) {
                if c == 'ψ' {
                    out.push(c);
                    i += 1;
                    // Optional subscript before the argument list.
                    if i < n && chars[i] == '_' {
                        out.push('_');
                        i += 1;
                        if i < n && chars[i] == '{' {
                            let start = i;
                            let mut depth = 1;
                            i += 1;
                            while i < n && depth > 0 {
                                if chars[i] == '{' { depth += 1; }
                                if chars[i] == '}' { depth -= 1; }
                                i += 1;
                            }
                            let inner: String = chars[start + 1..i - 1].iter().collect();
                            out.push('(');
                            out.push_str(&insert_mul(&inner));
                            out.push(')');
                        } else if i < n {
                            out.push(chars[i]);
                            i += 1;
                        }
                    }
                    if i >= n || chars[i] != '(' {
                        if i < n && starts_factor(chars[i]) { out.push('×'); }
                        continue;
                    }
                    let start = i;
                    let mut depth = 1;
                    i += 1;
                    while i < n && depth > 0 {
                        if chars[i] == '(' { depth += 1; }
                        if chars[i] == ')' { depth -= 1; }
                        i += 1;
                    }
                    let inner: String = chars[start + 1..i - 1].iter().collect();
                    out.push('(');
                    out.push_str(&insert_mul(&inner));
                    out.push(')');
                } else if c == '(' {
                    let start = i;
                    let mut depth = 1;
                    i += 1;
                    while i < n && depth > 0 {
                        if chars[i] == '(' { depth += 1; }
                        if chars[i] == ')' { depth -= 1; }
                        i += 1;
                    }
                    let inner: String = chars[start + 1..i - 1].iter().collect();
                    out.push('(');
                    out.push_str(&insert_mul(&inner));
                    out.push(')');
                } else if c.is_ascii_digit() {
                    while i < n && chars[i].is_ascii_digit() {
                        out.push(chars[i]);
                        i += 1;
                    }
                } else {
                    out.push(c);
                    i += 1;
                }
                // Consume attached _ / ^ chains.  Subscripts use the
                // parser's _(…) form with juxtaposition (no ×); exponents
                // keep braces and get × inserted.
                while i < n && (chars[i] == '_' || chars[i] == '^') {
                    let marker = chars[i];
                    out.push(marker);
                    i += 1;
                    if i < n && chars[i] == '{' {
                        let start = i;
                        let mut depth = 1;
                        i += 1;
                        while i < n && depth > 0 {
                            if chars[i] == '{' { depth += 1; }
                            if chars[i] == '}' { depth -= 1; }
                            i += 1;
                        }
                        let inner: String = chars[start + 1..i - 1].iter().collect();
                        if marker == '_' {
                            out.push('(');
                            out.push_str(&insert_mul(&inner));
                            out.push(')');
                        } else {
                            out.push('{');
                            out.push_str(&insert_mul(&inner));
                            out.push('}');
                        }
                    } else if i < n {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                if i < n && starts_factor(chars[i]) {
                    out.push('×');
                }
            } else {
                out.push(c);
                i += 1;
            }
        }
        out
    }



    #[test]
    fn generate_from_bms_source() {
        let root = "/data/data/com.termux/files/home/bms-analyzer-enhanced-main/";
        let Ok(content) = std::fs::read_to_string(format!("{}bms vs mocf.txt", root)) else {
            println!("== DATAGEN: source file absent, skipped ==");
            return;
        };
        let mut all_rows: Vec<(String, String)> = Vec::new();
        let mut matches: Vec<(String, String)> = Vec::new();
        let mut mismatches: Vec<(String, String, String)> = Vec::new();
        let mut errors: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with("(0,") || line.contains("\\dots") { continue; }
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() < 2 { continue; }
            let bms = parts[0].trim();
            let mocf_src = parts[parts.len() - 1].trim();
            if !seen.insert(bms.to_string()) { continue; }
            let m = parse_matrix(bms);
            let term = bms_to_bocf(&m);
            let raw = crate::term::term_to_string(false, &term);
            let bocf = latex_to_unicode(&raw);
            all_rows.push((bocf.clone(), mocf_src.to_string()));
            match super::bocf_to_mocf(&bocf) {
                Err(e) => errors.push((bocf.clone(), format!("{} :: {} :: {}", bms, raw, e))),
                Ok(got) => {
                    if norm_mocf_latex(&got) == norm_mocf_latex(mocf_src) {
                        matches.push((bocf, got));
                    } else {
                        mismatches.push((bocf, got, mocf_src.to_string()));
                    }
                }
            }
        }
        let mut out = String::from("\"\"Buchholz's OCF\",\"Madore's OCF (source)\",\"Madore's OCF (ours)\",\"status\"\"\n");
        for (b, s) in &all_rows {
            let mut status = String::from("(generated)");
            let mut ours = String::from("-");
            for (mb, mg) in &matches {
                if mb == b { ours = mg.clone(); status.push_str(" match"); break; }
            }
            if ours == "-" {
                for (mb2, mg2, _) in &mismatches {
                    if mb2 == b { ours = mg2.clone(); status.push_str(" MISMATCH"); break; }
                }
            }
            if ours == "-" {
                for (eb, ee) in &errors {
                    if eb == b { ours = ee.clone(); status.push_str(" ERROR"); break; }
                }
            }
            out.push_str(&format!("\"{}\",\"{}\",\"{}\",\"{}\"\n", b, s, ours, status));
        }
        std::fs::write(format!("{}bocf vs mocf generated2.csv", root), &out).unwrap();
        println!("== MATCHES: {} ==", matches.len());
        println!("== MISMATCHES: {} ==", mismatches.len());
        for (b, g, s) in mismatches.iter().take(10) {
            println!("  bocf {}\n    ours   {}\n    source {}", b, g, s);
        }
        println!("== ERRORS: {} ==", errors.len());
        for (b, e) in errors.iter().take(10) { println!("  {} → {}", b, e); }
    }
}

#[cfg(test)]
mod conflict_scan {
    use super::datagen::latex_to_unicode;
    use super::norm_mocf_latex;

    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_q = false;
        for c in line.chars() {
            if c == '"' { in_q = !in_q; }
            else if c == ',' && !in_q { out.push(cur.clone()); cur.clear(); }
            else { cur.push(c); }
        }
        out.push(cur);
        out
    }

    #[test]
    fn scan_conflicts() {
        let root = "/data/data/com.termux/files/home/bms-analyzer-enhanced-main/";
        let content = std::fs::read_to_string(format!("{}bocf vs mocf.csv", root)).unwrap();
        let mut map: std::collections::HashMap<String, Vec<(usize, String, String)>> =
            std::collections::HashMap::new();
        let mut parse_errs = 0usize;
        for (idx, line) in content.lines().enumerate() {
            if idx == 0 || line.trim().is_empty() { continue; }
            let fields = split_csv(line);
            if fields.len() < 2 { continue; }
            let input = fields[0].replace("\\cdot", "*").replace("\\times", "*");
            let key = match crate::parser::parse_bocf(&input) {
                Ok(ast) => match crate::parser::eval_ast(&ast) {
                    Ok(t) => crate::term::term_to_string(false, &crate::term::standard_form(&t)),
                    Err(e) => { parse_errs += 1; if parse_errs <= 16 { println!("EVAL FAIL: {} :: {}", input, e); } format!("ERR_EVAL {}", input) }
                },
                Err(e) => { parse_errs += 1; if parse_errs <= 8 { println!("PARSE FAIL: {} :: {}", input, e); } format!("ERR_PARSE {}", input) }
            };
            map.entry(key).or_default().push((idx + 1, fields[0].clone(), fields[1].clone()));
        }
        let mut conflicts = 0usize;
        let mut dup_groups = 0usize;
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for k in keys {
            let v = &map[k];
            if v.len() < 2 { continue; }
            dup_groups += 1;
            let first = norm_mocf_latex(&v[0].2);
            let clash = v.iter().any(|(_, _, m)| norm_mocf_latex(m) != first);
            if clash {
                conflicts += 1;
                println!("== CONFLICT (key {}) ==", k);
            }
            println!("== DUP GROUP ({} rows) ==", v.len());
            for (r, b, m) in v {
                println!("  row {}: input  {}
         expect {}", r, b, m);
            }
        }
        println!("== SCAN: total groups with same ordinal: {}, true conflicts: {}, parse errors: {} ==",
            dup_groups, conflicts, parse_errs);
    }
}

/// Deduplicate "bocf vs mocf.csv" by ordinal value (term standard form)
/// and sort by the parsed term. Run with:
///   cargo test --release -p bms-core dedup_sort -- --ignored --nocapture
#[cfg(test)]
mod dedup_sort {
    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_q = false;
        for c in line.chars() {
            if c == '"' { in_q = !in_q; }
            else if c == ',' && !in_q { out.push(cur.clone()); cur.clear(); }
            else { cur.push(c); }
        }
        out.push(cur);
        out
    }

    #[test]
    #[ignore]
    fn dedup_and_sort() {
        use crate::term as tm;
        let root = "/data/data/com.termux/files/home/bms-analyzer-enhanced-main/";
        let content = std::fs::read_to_string(format!("{}bocf vs mocf.csv", root)).unwrap();
        let mut header = String::new();
        // (term, original line); unparseable rows go to the end.
        let mut rows: Vec<(Option<crate::term::Term>, String)> = Vec::new();
        let mut unparseable: Vec<String> = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if idx == 0 { header = line.to_string(); continue; }
            if line.trim().is_empty() { continue; }
            let fields = split_csv(line);
            if fields.len() < 2 { continue; }
            let input = fields[0].replace("\\cdot", "*").replace("\\times", "*");
            let term = crate::parser::parse_bocf(&input)
                .ok()
                .and_then(|ast| crate::parser::eval_ast(&ast).ok())
                .map(|t| tm::standard_form(&t));
            match term {
                Some(t) => rows.push((Some(t), line.to_string())),
                None => unparseable.push(line.to_string()),
            }
        }
        let total = rows.len() + unparseable.len();
        // Deduplicate by canonical term string, keeping the first row.
        let mut seen = std::collections::HashSet::new();
        let mut kept: Vec<(crate::term::Term, String)> = Vec::new();
        let mut removed = 0usize;
        for (t, line) in rows {
            let t = t.unwrap();
            let key = tm::term_to_string(false, &t);
            if seen.insert(key) {
                kept.push((t, line));
            } else {
                removed += 1;
            }
        }
        kept.sort_by(|a, b| {
            if tm::lt(&a.0, &b.0) {
                std::cmp::Ordering::Less
            } else if tm::lt(&b.0, &a.0) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let mut out = header.clone();
        out.push('\n');
        for (_, line) in &kept {
            out.push_str(line);
            out.push('\n');
        }
        for line in &unparseable {
            out.push_str(line);
            out.push('\n');
        }
        std::fs::write(format!("{}bocf vs mocf sorted.csv", root), &out).unwrap();
        println!("== DEDUP: total {}, kept {}, removed {}, unparseable {} -> bocf vs mocf sorted.csv ==",
            total, kept.len(), removed, unparseable.len());
        for line in unparseable.iter().take(10) {
            println!("  UNPARSEABLE: {}", line);
        }
    }
}



// Scratch harness to audit CSV coverage.
#[cfg(test)]
mod csv_audit {
    use super::bocf_to_mocf;

    #[test]
    fn audit_all_rows() {
        let content = std::fs::read_to_string("../../../bocf vs mocf.csv")
            .expect("csv not found");
        let mut errors: Vec<String> = Vec::new();
        let mut mismatches: Vec<(usize, String, String)> = Vec::new();
        let mut nonstandard: Vec<(usize, String, String)> = Vec::new();
        let mut structural: Vec<(usize, String, String)> = Vec::new();
        let mut inputs: Vec<String> = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if idx == 0 || line.trim().is_empty() { continue; }
            // parse two quoted CSV fields
            let fields = split_csv(&line);
            if fields.len() < 2 { continue; }
            let input = fields[0].replace("\\cdot", "*");
            let expected = fields[1].clone();
            if inputs.len() <= idx + 1 { inputs.resize(idx + 2, String::new()); }
            inputs[idx + 1] = fields[0].clone();
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
                    let norm = |s: &str| super::norm_mocf_latex(s);
                    if norm(&got) != norm(&expected) {
                        mismatches.push((idx + 1, got, expected.clone()));
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
        // Write the full mismatch log (with inputs) to mismatch.txt.
        // GOT/WANT keep their original LaTeX (braces preserved); a
        // human-readable unicode rendering is added for each.
        {
            let mut log = String::new();
            log.push_str(&format!("MISMATCHES: {}\n\n", mismatches.len()));
            for (r, g, e) in &mismatches {
                let inp = inputs.get(*r).cloned().unwrap_or_default();
                log.push_str(&format!(
                    "row {}\n  IN   {}\n  GOT  {}\n       {}\n  WANT {}\n       {}\n\n",
                    r, inp, g, latex_preview(g), e, latex_preview(e)
                ));
            }
            let _ = std::fs::write("../../../mismatch.txt", &log);
        }
        println!("== STRUCTURAL-NORMALIZE CHANGES: {} ==", structural.len());
        for (r, before, after) in &structural {
            println!("row {}:\n   raw {}\n   nf  {}", r, before, after);
        }
    }

    /// Render LaTeX MOCF as readable unicode (braces → parentheses kept
    /// only where they carry grouping; \psi/\Omega/\omega substituted).
    fn latex_preview(s: &str) -> String {
        let mut out = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_alphabetic() { j += 1; }
                let cmd: String = chars[i + 1..j].iter().collect();
                match cmd.as_str() {
                    "psi" => out.push('ψ'),
                    "Omega" => out.push('Ω'),
                    "omega" => out.push('ω'),
                    "times" | "cdot" => out.push('×'),
                    _ => { out.push('\\'); out.push_str(&cmd); }
                }
                i = j;
            } else if c == '{' {
                // subscript/argument braces: keep grouping visible
                out.push('(');
                i += 1;
            } else if c == '}' {
                out.push(')');
                i += 1;
            } else {
                out.push(c);
                i += 1;
            }
        }
        out
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


