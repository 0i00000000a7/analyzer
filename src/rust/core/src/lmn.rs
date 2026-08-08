//! LMN (Lifting M-Notation) rendering of BOCF terms.
//!
//! Ported from `bms转m记号(ebo前).html`. That page's BMS→BOCF assignment is
//! the same as `bms::bms_to_bocf`, so only the notation rendering is ported
//! here: the `p_…` form, the `p_0(0)`-expanded simple form, and the
//! `0(…)`/`1(…)` lifting bracket form (the M-notation proper).
//!
//! The reference operates on its `[a, b, c] = ψ_a(b)+c` term arrays, which
//! match this crate's `Term` nodes `(a, b, c)` exactly.

use crate::term::{self as tm, Term};

/// The M-notation renderings of a standard-form BOCF term.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LmnForms {
    /// `p_2(p_1(0))+…` form.
    pub p: String,
    /// `p_0(0)`-expanded form.
    pub p_simple: String,
    /// `0(…)`/`1(…)` lifting bracket form.
    pub bracket: String,
    /// Full ψ form of the bracket notation: `0(` ↦ `ψ_0(`, `1(` ↦ `ψ_1(`,
    /// bare digits ↦ `ψ_n(0)`.
    pub full: String,
}

/// Render a standard-form BOCF term in all M-notation forms.
pub fn term_to_lmn(t: &Term) -> LmnForms {
    let bracket = render_bracket(t);
    LmnForms {
        p: render_p(t),
        p_simple: render_simple_p(t),
        full: expand_bracket(&bracket),
        bracket,
    }
}

/// The `p_…` form: natural subscripts as digits, other subscripts braced.
pub fn term_to_lmn_p(t: &Term) -> String {
    render_p(t)
}

/// The simple form: every natural written as a sum of `p_0(0)`.
pub fn term_to_lmn_simple(t: &Term) -> String {
    render_simple_p(t)
}

/// The lifting bracket form: `0(…)` heads with non-natural subscripts lifted
/// into `1(…)` wrappers.
pub fn term_to_lmn_bracket(t: &Term) -> String {
    render_bracket(t)
}

/// The full ψ form of the bracket notation.
pub fn term_to_lmn_full(t: &Term) -> String {
    expand_bracket(&render_bracket(t))
}

/// Expand the bracket form: a digit followed by `(` becomes `ψ_n(`, a bare
/// digit becomes `ψ_n(0)` (so `1+1+1` renders `ψ_1(0)+ψ_1(0)+ψ_1(0)`).
fn expand_bracket(bracket: &str) -> String {
    let chars: Vec<char> = bracket.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            if chars.get(i + 1) == Some(&'(') {
                out.push_str(&format!("\\psi_{}(", c));
                i += 2;
                continue;
            }
            out.push_str(&format!("\\psi_{}(0)", c));
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn render_p(t: &Term) -> String {
    let mut parts = Vec::new();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        // A trailing run of ψ₀(0) heads is a finite ordinal: render its digit.
        if tm::is_ordinal_finite(&cur) {
            parts.push(tm::length1(&cur).to_string());
            break;
        }
        let node = cur.as_ref().unwrap();
        let sub = if tm::is_ordinal_finite(&node.a) {
            tm::length1(&node.a).to_string()
        } else {
            format!("{{{}}}", render_p(&node.a))
        };
        parts.push(format!("p_{}({})", sub, render_p(&node.b)));
        cur = node.c.clone();
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("+")
    }
}

fn render_simple_p(t: &Term) -> String {
    if tm::is_zero(t) {
        return "0".to_string();
    }
    if tm::is_ordinal_finite(t) {
        return vec!["p_0(0)"; tm::length1(t) as usize].join("+");
    }
    walk_heads(t, |subscript, argument| {
        let sub = simple_subscript(subscript);
        format!("p_{}({})", sub, render_simple_p(argument))
    })
}

fn simple_subscript(subscript: &Term) -> String {
    let s = render_simple_p(subscript);
    if s.contains('+') || s.contains('(') {
        format!("{{{}}}", s)
    } else {
        s
    }
}

/// The bracket form: `ψ_n(β)` renders `0(n)` / `0(n+β)`, `ψ_0(β)` renders
/// `0` / `0(β)`, and a non-natural subscript is lifted into `1(…)`.
fn render_bracket(t: &Term) -> String {
    if tm::is_zero(t) {
        return "0".to_string();
    }
    walk_heads(t, |subscript, argument| {
        if let Some(n) = finval(subscript) {
            if n == 0 {
                if tm::is_zero(argument) {
                    "0".to_string()
                } else {
                    format!("0({})", render_bracket(argument))
                }
            } else if tm::is_zero(argument) {
                format!("0({})", unary_nat(n))
            } else {
                format!("0({}+{})", unary_nat(n), render_bracket(argument))
            }
        } else {
            let lifted = render_lifted(subscript);
            if tm::is_zero(argument) {
                format!("0({})", lifted)
            } else {
                format!("0({}+{})", lifted, render_bracket(argument))
            }
        }
    })
}

/// Lift a non-natural subscript: each head `ψ_a(b)` becomes `1`, `1(b)` for
/// `a = 0`, and `1(<bracket of ψ_a(b)>)` otherwise.
fn render_lifted(subscript: &Term) -> String {
    if let Some(n) = finval(subscript) {
        return unary_nat(n);
    }
    walk_heads(subscript, |inner_sub, inner_arg| {
        if tm::is_zero(inner_sub) && tm::is_zero(inner_arg) {
            return "1".to_string();
        }
        if tm::is_zero(inner_sub) {
            return format!("1({})", render_bracket(inner_arg));
        }
        let head = tm::t(inner_sub.clone(), inner_arg.clone(), tm::zero());
        format!("1({})", render_bracket(&head))
    })
}

/// A natural rendered in unary digits: `0` for 0, `1+1+…` otherwise.
fn unary_nat(n: i32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    vec!["1"; n as usize].join("+")
}

/// Finite ordinal as an int, if any.
fn finval(t: &Term) -> Option<i32> {
    if tm::is_ordinal_finite(t) {
        Some(tm::length1(t))
    } else {
        None
    }
}

fn walk_heads<F>(t: &Term, mut render: F) -> String
where
    F: FnMut(&Term, &Term) -> String,
{
    // Walk the c-chain node by node: each node is one ψ_a(b) head, so runs of
    // identical heads (finite ordinals) expand to one part per head.
    let mut parts = Vec::new();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let node = cur.as_ref().unwrap();
        parts.push(render(&node.a, &node.b));
        cur = node.c.clone();
    }
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nat(n: i32) -> Term {
        let mut t = tm::zero();
        for _ in 0..n {
            t = tm::succ(&t);
        }
        t
    }

    fn head(a: Term, b: Term) -> Term {
        tm::t(a, b, tm::zero())
    }

    fn assert_forms(term: &Term, p: &str, simple: &str, bracket: &str) {
        let f = term_to_lmn(term);
        assert_eq!(f.p, p, "p form of {}", tm::term_to_string(false, term));
        assert_eq!(f.p_simple, simple, "simple form of {}", tm::term_to_string(false, term));
        assert_eq!(f.bracket, bracket, "bracket form of {}", tm::term_to_string(false, term));
    }

    #[test]
    fn lmn_reference_cases() {
        let one = nat(1);
        let omega_c = head(nat(1), tm::zero()); // Ω = ψ₁(0)

        assert_forms(&tm::zero(), "0", "0", "0");
        assert_forms(&nat(3), "3", "p_0(0)+p_0(0)+p_0(0)", "0+0+0");
        assert_forms(&nat(1), "1", "p_0(0)", "0");
        // ω = ψ₀(1)
        assert_forms(&head(tm::zero(), one.clone()), "p_0(1)", "p_0(p_0(0))", "0(0)");
        // ε₀ = ψ₀(Ω)
        assert_forms(
            &head(tm::zero(), omega_c.clone()),
            "p_0(p_1(0))",
            "p_0(p_{p_0(0)}(0))",
            "0(0(1))",
        );
        // Ω = ψ₁(0)
        assert_forms(&omega_c, "p_1(0)", "p_{p_0(0)}(0)", "0(1)");
        // ψ₁(1)
        assert_forms(
            &head(one.clone(), one.clone()),
            "p_1(1)",
            "p_{p_0(0)}(p_0(0))",
            "0(1+0)",
        );
        // ψ₁(2)
        assert_forms(
            &head(one.clone(), nat(2)),
            "p_1(2)",
            "p_{p_0(0)}(p_0(0)+p_0(0))",
            "0(1+0+0)",
        );
        // ψ₂(0)
        assert_forms(&head(nat(2), tm::zero()), "p_2(0)", "p_{p_0(0)+p_0(0)}(0)", "0(1+1)");
        // ψ₃(1)
        assert_forms(
            &head(nat(3), one.clone()),
            "p_3(1)",
            "p_{p_0(0)+p_0(0)+p_0(0)}(p_0(0))",
            "0(1+1+1+0)",
        );
        // ψ₀(ψ₀(Ω))
        let eps0 = head(tm::zero(), omega_c.clone());
        assert_forms(
            &head(tm::zero(), eps0.clone()),
            "p_0(p_0(p_1(0)))",
            "p_0(p_0(p_{p_0(0)}(0)))",
            "0(0(0(1)))",
        );
        // ψ₀(ψ₁(1))
        let psi1_1 = head(one.clone(), one.clone());
        assert_forms(
            &head(tm::zero(), psi1_1),
            "p_0(p_1(1))",
            "p_0(p_{p_0(0)}(p_0(0)))",
            "0(0(1+0))",
        );
        // ψ₁(ψ₀(Ω))
        assert_forms(
            &head(one, eps0),
            "p_1(p_0(p_1(0)))",
            "p_{p_0(0)}(p_0(p_{p_0(0)}(0)))",
            "0(1+0(0(1)))",
        );
        // ψ_ω(0) = ψ_{ψ₀(1)}(0)
        let omega = head(tm::zero(), nat(1));
        assert_forms(
            &head(omega, tm::zero()),
            "p_{p_0(1)}(0)",
            "p_{p_0(p_0(0))}(0)",
            "0(1(0))",
        );
        // ψ₀(ω) = ψ₀(ψ₀(1))
        assert_forms(
            &head(tm::zero(), head(tm::zero(), nat(1))),
            "p_0(p_0(1))",
            "p_0(p_0(p_0(0)))",
            "0(0(0))",
        );
    }

    #[test]
    fn lmn_full_form() {
        let one = nat(1);
        let omega_c = head(nat(1), tm::zero());
        let eps0 = head(tm::zero(), omega_c);
        // 0 ↦ ψ_0(0); 0(1) ↦ ψ_0(ψ_1(0)); 1+1+1 ↦ ψ_1(0)+ψ_1(0)+ψ_1(0).
        assert_eq!(term_to_lmn_full(&tm::zero()), "\\psi_0(0)");
        assert_eq!(term_to_lmn_full(&head(tm::zero(), one.clone())), "\\psi_0(\\psi_0(0))");
        assert_eq!(
            term_to_lmn_full(&eps0),
            "\\psi_0(\\psi_0(\\psi_1(0)))"
        );
        assert_eq!(
            term_to_lmn_full(&head(nat(3), one)),
            "\\psi_0(\\psi_1(0)+\\psi_1(0)+\\psi_1(0)+\\psi_0(0))"
        );
    }

    #[test]
    fn lmn_end_to_end_default_matrix() {
        // The default matrix of the reference page; its BOCF ordinal is
        // ψ₀(ψ_{ψ_{ψ₃(0)+2}(0)+1}(0)) in both implementations.
        let cols: Vec<Vec<i32>> = [
            [0, 0, 0],
            [1, 1, 1],
            [2, 1, 1],
            [3, 1, 0],
            [1, 1, 1],
            [2, 1, 1],
            [3, 1, 0],
            [1, 1, 0],
            [2, 2, 1],
            [3, 2, 1],
            [4, 2, 0],
            [2, 2, 1],
            [3, 2, 1],
            [4, 2, 0],
            [2, 2, 0],
            [3, 3, 1],
            [4, 3, 1],
            [5, 3, 0],
            [3, 3, 1],
            [4, 3, 1],
            [5, 3, 0],
            [3, 3, 0],
            [4, 4, 1],
            [5, 4, 1],
            [6, 4, 0],
            [4, 4, 1],
            [5, 4, 1],
            [6, 3, 0],
            [5, 4, 0],
            [6, 5, 1],
            [7, 5, 1],
            [8, 5, 0],
            [6, 5, 0],
            [7, 6, 1],
            [8, 6, 1],
            [9, 6, 0],
            [7, 6, 0],
            [8, 7, 1],
            [9, 7, 1],
            [10, 6, 0],
            [9, 7, 0],
            [10, 8, 0],
        ]
        .iter()
        .map(|c| c.to_vec())
        .collect();
        let term = crate::bms::bms_to_bocf(&cols);
        assert_forms(
            &term,
            "p_0(p_{p_{p_3(0)+2}(0)+1}(0))",
            "p_0(p_{p_{p_{p_0(0)+p_0(0)+p_0(0)}(0)+p_0(0)+p_0(0)}(0)+p_0(0)}(0))",
            "0(0(1(0(1(0(1+1+1))+1+1))+1))",
        );
    }
}
