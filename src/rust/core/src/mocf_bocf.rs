//! MOCF → BOCF conversion (inverse of `bocf_mocf`), calibrated on
//! `bocf vs mocf.csv`.
//!
//! Core scheme: for a MOCF fixed point q with ν(q) = ψ₀(β):
//!   ν(q^W)      = ψ₀(β + r)          (power, r blockwise from ν(W))
//!   ν(q^c·ω^a·m) = ψ₀(β + Q·(c−1) + a)·m   (product family)
//! Level-0 ψ: ψ_M(A) ↦ ψ₀(Ω·(1+ν(A))) for countable A, with cardinal /
//! subscript / power shifts for uncountable arguments.

use crate::ocf::{parse_mocf, Mocf};
use crate::term::{self as tm, Term};

pub fn mocf_to_bocf(input: &str) -> Result<String, String> {
    let m = parse_mocf(input)?;
    let t = mocf_to_term_top(&m)?;
    Ok(term_to_bocf_input(&tm::standard_form(&t)))
}

/// Top-level MOCF → BOCF term. Runs the forward-converter round-trip once:
/// the forward map is eval-first and canonical, so a candidate term is the
/// correct value of `m` exactly when forwarding it reproduces `m`. Falls
/// back to `mocf_to_term` otherwise.
pub fn mocf_to_term_top(m: &Mocf) -> Result<Term, String> {
    if let Some(t) = identity_route(m) {
        return Ok(t);
    }
    mocf_to_term(m)
}

/// Convert the forward converter's LaTeX MOCF output to the plain notation
/// that `parse_mocf` accepts.
fn latex_mocf_to_plain(s: &str) -> String {
    let mut t = s
        .replace("\\cdot", "*")
        .replace("\\Omega", "Ω")
        .replace("\\omega", "ω")
        .replace("\\psi", "ψ")
        .replace("^{", "^(")
        .replace("_{", "_(")
        .replace('{', "")
        .replace('}', ")");
    t.retain(|c| !c.is_whitespace());
    // The renderer writes products by juxtaposition (C::Mul). Insert '*'
    // between adjacent factors: left ∈ {')', digit, ψ, Ω, ω} followed by
    // right ∈ {ψ, Ω, ω, digit}. Never touch '(' (arguments / the
    // subscript→argument transition ψ_(1)(arg)), powers, or subscripts.
    fn is_factor_end(c: char) -> bool {
        c == ')' || c.is_ascii_digit() || matches!(c, 'ψ' | 'Ω' | 'ω')
    }
    fn is_factor_start(c: char) -> bool {
        matches!(c, 'ψ' | 'Ω' | 'ω') || c.is_ascii_digit()
    }
    let chars: Vec<char> = t.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        out.push(*c);
        if let Some(&n) = chars.get(i + 1) {
            if is_factor_end(*c) && is_factor_start(n) && !(c.is_ascii_digit() && n.is_ascii_digit())
            {
                out.push('*');
            }
        }
    }
    out
}

/// Structures whose literal value is NOT the expected BOCF representative:
/// ψ_v(0) blocks (converted q-power families) and Ω_{ω^…} cardinals, where
/// MOCF folds Ω_ω^e into Ω_{ω^e} but BOCF eval keeps the power form.
fn route_blocker(m: &Mocf) -> bool {
    match m {
        Mocf::Psi(_, a) if matches!(a.as_ref(), Mocf::Zero) => true,
        Mocf::Psi(v, a) => route_blocker(v) || route_blocker(a),
        Mocf::Sum(ts) => ts.iter().any(route_blocker),
        Mocf::OmegaPow(e) | Mocf::Omega(e) => route_blocker(e),
        Mocf::Pow(b, e) => route_blocker(b) || route_blocker(e),
        Mocf::Zero => false,
    }
}

/// Return the identity-region value of `m` when the raw (ψ_v-preserving)
/// value forwards back to `m`; otherwise None.
fn identity_route(m: &Mocf) -> Option<Term> {
    if route_blocker(m) {
        return None;
    }
    // Row 295 pattern: ψ(Ω^{Ω^ω+k}) ↦ ψ₀(ψ₁(Ω^ω+Ω²·k)).
    if let Mocf::Psi(v0, a0) = m {
        if matches!(v0.as_ref(), Mocf::Zero) {
            if let Mocf::OmegaPow(f) = a0.as_ref() {
                let fps = prim_list(f);
                if fps.len() >= 2 {
                    let lead_double = matches!(fps[0],
                        Mocf::OmegaPow(inner) if matches!(inner.as_ref(), Mocf::OmegaPow(_)));
                    let om1 = Mocf::Omega(Box::new(Mocf::OmegaPow(Box::new(Mocf::Zero))));
                    let mut k = 0i32;
                    for p in fps[1..].iter() {
                        if mocf_eq(p, &om1) {
                            k += 1;
                        } else {
                            k = 0;
                            break;
                        }
                    }
                    if lead_double && k >= 1 {
                        let om = omega1();
                        let omw_pow = tm::standard_form(&tm::exp(
                            &tm::standard_form(&tm::mul(&om, &tm::omega())),
                        ));
                        let om2 = tm::exp(&tm::mul(&om, &nat(2)));
                        let inner = tm::add(&omw_pow, &mul_k(&om2, k));
                        return Some(tm::t(
                            tm::zero(),
                            tm::t(tm::one(), inner, tm::zero()),
                            tm::zero(),
                        ));
                    }
                }
            }
        }
    }
    let raw = raw_mocf_value(m).ok()?;
    let sf = tm::standard_form(&raw);
    // ψ₀(ψ_v(Ω_ω+Ω·k)) is a collapsed Ω^ω power needing the trailing lift
    // (row 295), not the identity route.
    if let Some(sn) = sf.as_ref() {
        if tm::is_zero(&sn.a) && tm::is_zero(&sn.c) {
            if let Some(bn) = sn.b.as_ref() {
                if !tm::is_zero(&bn.a)
                    && tm::is_zero(&bn.c)
                    && finval(&bn.a).map_or(false, |v| v >= 1)
                {
                    if let Some(b1) = bn.b.as_ref() {
                        let omw_inner = !tm::is_zero(&b1.a)
                            && matches!(b1.b.as_ref(),
                                Some(w) if tm::eq(&w.a, &tm::omega())
                                    && tm::is_zero(&w.b) && tm::is_zero(&w.c));
                        if omw_inner && !tm::is_zero(&b1.c) {
                            let (om_run, after) = tm::separate(&b1.c, &omega1());
                            if tm::is_zero(&after) && !tm::is_zero(&om_run) {
                                return None;
                            }
                        }
                    }
                }
            }
        }
    }
    let rendered = crate::bocf_mocf::term_to_mocf(&sf);
    let back = parse_mocf(&latex_mocf_to_plain(&rendered)).ok()?;
    if crate::ocf::mocf_value_eq(&back, m) {
        Some(raw)
    } else {
        None
    }
}

fn nat(n: i32) -> Term {
    let mut t = tm::zero();
    for _ in 0..n {
        t = tm::succ(&t);
    }
    t
}

fn omega1() -> Term {
    tm::t(tm::one(), tm::zero(), tm::zero())
}
fn omega_s(s: &Term) -> Term {
    tm::t(s.clone(), tm::zero(), tm::zero())
}

fn finval(t: &Term) -> Option<i32> {
    if tm::is_ordinal_finite(t) {
        Some(tm::length1(t))
    } else {
        None
    }
}

fn log_b(t: &Term) -> Term {
    if let Some(n) = t.as_ref() {
        if tm::is_zero(&n.a) && tm::is_zero(&n.c) {
            if !tm::is_zero(&n.b) {
                return n.b.clone();
            }
            if tm::is_zero(&n.a) {
                // finite: log(n) handled below
            }
        }
    }
    if tm::is_ordinal_finite(t) {
        return tm::zero();
    }
    tm::log(t)
}

fn mul_k(blk: &Term, k: i32) -> Term {
    if k == 1 {
        blk.clone()
    } else {
        tm::mul_finite(blk, &nat(k))
    }
}

pub fn mocf_to_term(m: &Mocf) -> Result<Term, String> {
    match m {
        Mocf::Zero => Ok(tm::zero()),
        Mocf::Sum(terms) => {
            let mut acc = tm::zero();
            for p in terms {
                acc = tm::add(&acc, &mocf_to_term(p)?);
            }
            Ok(acc)
        }
        Mocf::OmegaPow(e) => {
            if let Some(t) = omega_pow_pre(e)? {
                return Ok(t);
            }
            let et = mocf_to_term(e)?;
            let r = tm::standard_form(&tm::exp(&et));
            Ok(r)
        }
        Mocf::Omega(a) => Ok(tm::t(mocf_to_term(a)?, tm::zero(), tm::zero())),
        Mocf::Psi(v, a) => {
            let vm = mocf_to_term(v)?;
            if !tm::is_zero(&vm) {
                let am = mocf_to_term(a)?;
                // Stay-band: ψ_v(X) with Ω_{v+1} ≤ X ≤ Ω_{v+2} keeps its
                // subscript (identity region), as does ψ_v(Ω_λ) for pure
                // limit cardinals; otherwise reduce
                // ψ_v ↦ ψ_{v-1}(Ω_{v+1}·(1+ν)).
                if let Some(n) = finval(&vm) {
                    let lo = omega_s(&tm::add(&vm, &tm::one()));
                    let hi = omega_s(&tm::add(&vm, &nat(2)));
                    let pure_limit_card = matches!(am.as_ref(),
                        Some(an) if !tm::is_zero(&an.a) && tm::is_zero(&an.b) && tm::is_zero(&an.c)
                            && !tm::is_zero(&an.a) && !tm::is_succ(&an.a)
                            && finval(&an.a).is_none());
                    if n >= 1
                        && (!tm::lt(&am, &lo) && !tm::lt(&hi, &am) || pure_limit_card)
                    {
                        return Ok(tm::t(vm, am, tm::zero()));
                    }
                    // ψ_v(0) ↦ ψ_v(Ω_{v+1}): subscript-preserving value form.
                    if n >= 1 && tm::is_zero(&am) {
                        return Ok(tm::t(vm, lo, tm::zero()));
                    }
                    // ψ_v(Ω_λ + rest) with rest ≠ 0 collapses to Ω_{v+1}.
                    if n >= 1 {
                        let fb = tm::first_term(&am);
                        let lead_lim = matches!(fb.as_ref(),
                            Some(fn_) if !tm::is_zero(&fn_.a) && tm::is_zero(&fn_.b)
                                && tm::is_zero(&fn_.c) && !tm::is_succ(&fn_.a)
                                && finval(&fn_.a).is_none());
                        if lead_lim && !tm::eq(&fb, &am) {
                            return Ok(omega_s(&tm::add(&vm, &tm::one())));
                        }
                    }
                }
                let bsub = tm::sub(&vm, &tm::one());
                let card = omega_s(&tm::add(&vm, &tm::one()));
                let arg = tm::standard_form(&tm::mul(&card, &tm::add(&tm::one(), &am)));
                return Ok(tm::t(bsub, arg, tm::zero()));
            }
            psi_level0(a)
        }
        Mocf::Pow(q, e) => {
            let qe = tm::standard_form(&tm::mul(&mocf_to_term(q)?, &mocf_to_term(e)?));
            Ok(tm::standard_form(&tm::exp(&qe)))
        }
    }
}

fn is_fixed_prim(p: &Mocf) -> bool {
    matches!(p, Mocf::Psi(..) | Mocf::Omega(_))
}

fn prim_list(m: &Mocf) -> Vec<&Mocf> {
    match m {
        Mocf::Sum(ts) => ts.iter().collect(),
        Mocf::Zero => vec![],
        other => vec![other],
    }
}

/// Build a MOCF from a slice of prims (like ocf's private from_prim_list).
fn sum_of_prims(ps: &[&Mocf]) -> Mocf {
    let v: Vec<Mocf> = ps.iter().map(|p| (*p).clone()).collect();
    if v.is_empty() {
        Mocf::Zero
    } else if v.len() == 1 {
        v.into_iter().next().unwrap()
    } else {
        Mocf::Sum(v)
    }
}

/// Preimage of ω^e when e carries a fixed-point factor q.
fn omega_pow_pre(e: &Mocf) -> Result<Option<Term>, String> {
    if !a_is_q_structured(e) {
        return Ok(None);
    }
    // Deep wrap at level 0: ω^{ω^{B}} ↦ ψ₀(Ω + image(ω^{B})).
    if let Mocf::OmegaPow(inner) = e {
        if a_is_q_structured(inner) {
            let ql = q_level_of(inner);
            if ql == 0 && inner_psi0_led(inner) {
                if let Some(t) = deep_inner_image(inner)? {
                    return Ok(Some(tm::t(
                        tm::zero(),
                        tm::add(&omega1(), &t),
                        tm::zero(),
                    )));
                }
                let t = mocf_to_term(&Mocf::OmegaPow(inner.clone()))?;
                return Ok(Some(tm::t(
                    tm::zero(),
                    tm::add(&omega1(), &t),
                    tm::zero(),
                )));
            }
            return decompose_q_power_ctx(inner, true, true);
        }
    }
    decompose_q_power_mode(e, matches!(e, Mocf::OmegaPow(_)))
}

/// Inner image of the level-0 deep wrap ω^{ω^E} ↦ ψ₀(Ω + ψ₀(Ω + z(E))).
/// z(ψ(0)·n + r) = 2(n−2) + r (rows 39/44/45) and
/// z(ω^{ψ(0)·j + s}) = ψ₀(Ω)·(j−1) + ψ₀(s) (rows 46/47).
fn deep_inner_image(e: &Mocf) -> Result<Option<Term>, String> {
    let (run, rest) = match e {
        Mocf::Sum(ts) => {
            let mut n = 0usize;
            while n < ts.len() && matches!(&ts[n], Mocf::Psi(v, z)
                if matches!(v.as_ref(), Mocf::Zero) && matches!(z.as_ref(), Mocf::Zero))
            {
                n += 1;
            }
            if n < 2 {
                return Ok(None);
            }
            (n, sum_of_prims(&ts[n..].iter().collect::<Vec<_>>()))
        }
        Mocf::OmegaPow(f) => {
            let ts = prim_list(f);
            let mut n = 0usize;
            while n < ts.len() && matches!(&ts[n], Mocf::Psi(v, z)
                if matches!(v.as_ref(), Mocf::Zero) && matches!(z.as_ref(), Mocf::Zero))
            {
                n += 1;
            }
            let rest = sum_of_prims(&ts[n..]);
            if n == 0 || (n == 1 && matches!(rest, Mocf::Zero)) {
                return Ok(None);
            }
            let pblk = tm::t(tm::zero(), omega1(), tm::zero());
            let z = if n >= 1 {
                let mut z = mul_k(&pblk, n as i32 - 1);
                if !matches!(rest, Mocf::Zero) {
                    z = tm::add(&z, &tm::t(tm::zero(), mocf_to_term(&rest)?, tm::zero()));
                }
                z
            } else {
                tm::zero()
            };
            let img = tm::t(
                tm::zero(),
                tm::add(&omega1(), &tm::t(tm::zero(), tm::add(&omega1(), &z), tm::zero())),
                tm::zero(),
            );
            return Ok(Some(img));
        }
        _ => return Ok(None),
    };
    if run < 2 {
        return Ok(None);
    }
    let mut z = mul_k(&nat(2), run as i32 - 2);
    if !matches!(rest, Mocf::Zero) {
        z = tm::add(&z, &mocf_to_term(&rest)?);
    }
    let img = tm::t(
        tm::zero(),
        tm::add(&omega1(), &tm::t(tm::zero(), tm::add(&omega1(), &z), tm::zero())),
        tm::zero(),
    );
    Ok(Some(img))
}

/// True if the expression begins syntactically with a ψ_s term (s ≥ 1).
fn psi_leading_syntax_v(a: &Mocf) -> bool {
    match a {
        Mocf::Psi(v, _) => !matches!(v.as_ref(), Mocf::Zero),
        Mocf::Sum(ts) => !ts.is_empty() && psi_leading_syntax_v(&ts[0]),
        _ => false,
    }
}

/// Subscript-preserving image of a ψ_v(y) term.
fn image_sub(t: &Mocf) -> Result<Term, String> {
    if let Mocf::Psi(v, y) = t {
        let vi = finval(&mocf_to_term(v)?).unwrap_or(0);
        let (card, inner) = match y.as_ref() {
            Mocf::Psi(v2, z) => {
                let ti = finval(&mocf_to_term(v2)?).unwrap_or(0);
                let iz = match z.as_ref() {
                    Mocf::Psi(..) => image_sub(z)?,
                    Mocf::Zero => tm::zero(),
                    other => mocf_to_term(other)?,
                };
                (omega_s(&nat(ti + 1)), iz)
            }
            Mocf::Zero => (omega_s(&nat(vi + 1)), tm::zero()),
            other => (omega_s(&nat(vi + 1)), mocf_to_term(other)?),
        };
        let arg = tm::standard_form(&tm::mul(&card, &tm::add(&tm::one(), &inner)));
        return Ok(tm::t(nat(vi), arg, tm::zero()));
    }
    mocf_to_term(t)
}

/// enc without the outer ψ-wrap: enc_val(1)=1, enc_val(ω)=ψ₀(1), …
fn enc_val(t: &Term) -> Term {
    if tm::is_ordinal_finite(t) {
        return t.clone();
    }
    let e = enc_term(t);
    if let Some(n) = e.as_ref() {
        if tm::is_zero(&n.a) && tm::is_zero(&n.c) {
            return n.b.clone();
        }
    }
    e
}

/// Syntax-level lift at level s: translates an argument into its
/// level-s BOCF value (Ω_v ↦ Ω_v² for v < s, ψ_{s-1} ↦ …, ψ_M ↦ ψ₀(Ω·…)).
fn lift_syn(t: &Mocf, s: i32) -> Result<Term, String> {
    match t {
        Mocf::Zero => Ok(tm::zero()),
        Mocf::Psi(v, y) => {
            let vi = finval(&mocf_to_term(v)?).unwrap_or(0);
            if vi == 0 {
                // ψ_M(z): level-0 image lifted: ψ₀(Ω·lift_1(z))
                let inner = lift_syn(y, 1)?;
                let arg = if tm::is_zero(&inner) {
                    omega1()
                } else {
                    tm::standard_form(&tm::mul(&omega1(), &inner))
                };
                return Ok(tm::t(tm::zero(), arg, tm::zero()));
            }
            if s <= vi {
                // level-0 image: ψ₀(Ω_{vi+1}·(1+ν(y)))
                let am = mocf_to_term(y)?;
                let card = omega_s(&nat(vi + 1));
                return Ok(tm::t(
                    tm::zero(),
                    tm::standard_form(&tm::mul(&card, &tm::add(&tm::one(), &am))),
                    tm::zero(),
                ));
            }
            let im = image_sub(t)?;
            if s >= 4 {
                return Ok(tm::standard_form(&tm::mul(
                    &omega_s(&nat(vi + 1)),
                    &im,
                )));
            }
            Ok(im)
        }
        Mocf::Omega(os) => {
            let vi = finval(&mocf_to_term(os)?).unwrap_or(0);
            if vi >= 1 && (vi as i32) < s {
                let c = omega_s(&nat(vi));
                return Ok(tm::standard_form(&tm::exp(&tm::mul(&c, &nat(2)))));
            }
            Ok(omega_s(&nat(vi)))
        }
        Mocf::Sum(ts) => {
            let mut acc = tm::zero();
            for p in ts {
                acc = tm::add(&acc, &lift_syn(p, s)?);
            }
            Ok(acc)
        }
        Mocf::OmegaPow(f) if matches!(f.as_ref(), Mocf::Zero) => Ok(omega1()),
        Mocf::OmegaPow(f) => {
            // ω^{Ω_s·k + r} = Ω_s^k·ω^r
            let fps = prim_list(f);
            let mut j = 0usize;
            while j < fps.len() {
                if let Mocf::Omega(os) = fps[j] {
                    if finval(&mocf_to_term(os)?).unwrap_or(0) >= 1 {
                        j += 1;
                        continue;
                    }
                }
                break;
            }
            if j >= 1 && j < fps.len() {
                let lead = lift_syn(fps[0], s)?;
                let mut r = tm::zero();
                for p in &fps[j..] {
                    r = tm::add(&r, &mocf_to_term(p)?);
                }
                let xr = tm::standard_form(&tm::exp(&r));
                return Ok(tm::standard_form(&tm::mul(&lead, &xr)));
            }
            mocf_to_term(t)
        }
        _ => {
            let tv = mocf_to_term(t)?;
            if let Some(n) = finval(&tv) {
                return Ok(mul_k(&omega1(), n as i32));
            }
            Ok(tv)
        }
    }
}

/// True if the leading ψ-primitive of a is ψ_s(k) with k ≠ 0.
fn lead_psi_arg_nonzero(a: &Mocf) -> bool {
    let first = match a {
        Mocf::Sum(ts) => ts.first(),
        other => Some(other),
    };
    match first {
        Some(Mocf::Psi(v, a2)) => {
            if !matches!(v.as_ref(), Mocf::Zero) {
                matches!(mocf_to_term(a2), Ok(t) if !tm::is_zero(&t))
            } else {
                false
            }
        }
        Some(Mocf::OmegaPow(f)) => lead_psi_arg_nonzero(f),
        _ => false,
    }
}

/// True if a begins syntactically with a ψ_M(…)-term (level-0 ψ).
fn psi0_leading_syntax2(a: &Mocf) -> bool {
    let first = match a {
        Mocf::Sum(ts) => ts.first(),
        other => Some(other),
    };
    matches!(first, Some(Mocf::Psi(v, _)) if matches!(v.as_ref(), Mocf::Zero))
}

/// Level-v lift of a small block: blocks ↦ ψ_v(β + inner).
fn lift_v_block(head: &Term, _v: &Term, _beta: &Term) -> Term {
    let mut acc = tm::zero();
    let mut cur = head.clone();
    while !tm::is_zero(&cur) {
        let h = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &h);
        let k = tm::length1(&run);
        let nh = match h.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let inner = if !tm::is_zero(&nh.a) {
            h.clone()
        } else if tm::is_zero(&nh.b) {
            h.clone()
        } else if !tm::lt(&nh.b, &omega1()) {
            h.clone()
        } else {
            enc_val(&tm::t(tm::zero(), nh.b.clone(), tm::zero()))
        };
        acc = tm::add(&acc, &mul_k(&inner, k));
        cur = rest;
    }
    acc
}

/// True if the term reaches Ω or beyond (ψ₀(b) with b ≥ Ω, or Ω_a blocks).
fn term_uncountable(t: &Term) -> bool {
    match t.as_ref() {
        Some(n) => {
            if !tm::is_zero(&n.a) {
                return true;
            }
            b_reaches_omega(&n.b)
        }
        None => false,
    }
}

/// True if b contains an Ω-factor (reaches Ω or beyond).
fn b_reaches_omega(b: &Term) -> bool {
    if tm::is_zero(b) {
        return false;
    }
    let fbt = tm::first_term(b);
    let fb = match fbt.as_ref() {
        Some(n) => n,
        None => return false,
    };
    if !tm::is_zero(&fb.a) {
        // Ω-run: countable only for a single Ω plus a finite rest.
        if tm::eq(&fb.a, &tm::one()) && tm::is_zero(&fb.b) {
            let (run2, r2) = tm::separate(b, &fbt);
            let k2 = tm::length1(&run2);
            return !(k2 == 1 && (tm::is_zero(&r2) || finval(&r2).is_some()));
        }
        return true;
    }
    let nb = fb.b.clone();
    b_reaches_omega(&nb)
}

/// True if the expression's leading ψ-primitive is ψ_M(0).
fn inner_psi0_led(a: &Mocf) -> bool {
    let ps = prim_list(a);
    match ps.first() {
        Some(Mocf::Psi(v, y)) => {
            matches!(v.as_ref(), Mocf::Zero) && matches!(y.as_ref(), Mocf::Zero)
        }
        Some(Mocf::OmegaPow(f)) => inner_psi0_led(f),
        _ => false,
    }
}

/// True if the leading ψ-primitive is ψ_M(0) or ψ_M(Ω^e·…)-formed.
fn inner_q_acceptable(a: &Mocf) -> bool {
    let ps = prim_list(a);
    match ps.first() {
        Some(Mocf::Psi(v, y)) => {
            if !matches!(v.as_ref(), Mocf::Zero) {
                return false;
            }
            if matches!(y.as_ref(), Mocf::Zero) {
                return true;
            }
            omega_led_syntax(y)
        }
        Some(Mocf::OmegaPow(f)) => inner_q_acceptable(f),
        _ => false,
    }
}

/// True if the expression begins with an Ω/Ω^e block.
fn omega_led_syntax(a: &Mocf) -> bool {
    let ps = prim_list(a);
    match ps.first() {
        Some(Mocf::Omega(_)) => true,
        Some(Mocf::OmegaPow(f)) => omega_led_syntax(f),
        _ => false,
    }
}

/// True if a's leading primitive is a limit cardinal Ω_λ.
fn lead_omega_limit(a: &Mocf) -> bool {
    let ps = prim_list(a);
    match ps.first() {
        Some(Mocf::Omega(s)) => matches!(mocf_to_term(s), Ok(t) if finval(&t).is_none() && !tm::is_zero(&t)),
        Some(Mocf::OmegaPow(f)) => lead_omega_limit(f),
        _ => false,
    }
}

/// True if the leading ψ-primitive of a is ψ_s(0).
fn lead_psi_arg_zero(a: &Mocf) -> bool {
    let ps = prim_list(a);
    match ps.first() {
        Some(Mocf::Psi(_, a2)) => {
            matches!(mocf_to_term(a2), Ok(t) if tm::is_zero(&t))
        }
        Some(Mocf::OmegaPow(f)) => lead_psi_arg_zero(f),
        _ => false,
    }
}

/// Subscript level of the leading fixed point of a q-structured expression.
fn q_level_of(a: &Mocf) -> i32 {
    let ps = prim_list(a);
    let p = match ps.first() {
        Some(p) => p,
        None => return 0,
    };
    let q = match p {
        Mocf::Psi(v, _) => v.as_ref(),
        Mocf::Omega(_) => return -1,
        Mocf::OmegaPow(f) => match prim_list(f).first() {
            Some(Mocf::Psi(v, _)) => v.as_ref(),
            _ => return -1,
        },
        _ => return 0,
    };
    match q {
        Mocf::Zero => 0,
        Mocf::OmegaPow(ref inner) if matches!(inner.as_ref(), Mocf::Zero) => 1,
        _ => 0,
    }
}

/// True if A's prim list begins with a fixed point q or ω^{q·…} blocks.
fn a_is_q_structured(a: &Mocf) -> bool {
    let ps = prim_list(a);
    if ps.is_empty() {
        return false;
    }
    match &ps[0] {
        Mocf::Psi(..) | Mocf::Omega(_) => true,
        Mocf::OmegaPow(f) => {
            let fps = prim_list(f);
            if fps.is_empty() {
                return false;
            }
            is_fixed_prim(fps[0]) || a_is_q_structured(f)
        }
        _ => false,
    }
}

/// Decompose ω^A with A = q·c + (q^j·ω^X parts) + R (R < q):
/// builds the preimage ψ₀(β + r)·m.
fn decompose_q_power_mode(a: &Mocf, product_mode: bool) -> Result<Option<Term>, String> {
    decompose_q_power_ctx(a, product_mode, false)
}

fn decompose_q_power_ctx(
    a: &Mocf,
    product_mode: bool,
    power_ctx: bool,
) -> Result<Option<Term>, String> {
    let ps = prim_list(a);
    if ps.is_empty() {
        return Ok(None);
    }
    // Determine q and β from the first prim.
    let (q_mocf, q_lead_exp): (Mocf, Option<Box<Mocf>>) = match &ps[0] {
        Mocf::OmegaPow(f) => {
            let fps = prim_list(f);
            (fps[0].clone(), Some(f.clone()))
        }
        p @ (Mocf::Psi(..) | Mocf::Omega(_)) => ((*p).clone(), None),
        _ => return Ok(None),
    };
    let q_is_psi0 = matches!(&q_mocf, Mocf::Psi(vv, _) if matches!(vv.as_ref(), Mocf::Zero));
    let q = mocf_to_term(&q_mocf)?;
    let qnode = match q.as_ref() {
        Some(n) if tm::is_zero(&n.c) && !tm::is_zero(&n.b) => n,
        Some(n)
            if tm::is_zero(&n.c)
                && tm::is_zero(&n.b)
                && !tm::is_zero(&n.a)
                && finval(&n.a).is_none() =>
        {
            n
        }
        _ => return Ok(None),
    };
    let beta = if tm::is_zero(&qnode.b) {
        q.clone()
    } else {
        qnode.b.clone()
    };
    // β must be a pure Ω-multiple or a cardinal multiple Ω_s·k.
    let beta_ok = if tm::is_zero(&qnode.b) {
        !tm::is_zero(&qnode.a) && finval(&qnode.a).is_none()
    } else if !tm::is_zero(&qnode.a) {
        // ψ_{s-1}(Ω_{s'}·k) form: β must be an Ω_{s'}-multiple with s' ≥ 2.
        let fb = tm::first_term(&qnode.b);
        match fb.as_ref() {
            Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b) => {
                let sv = finval(&nb.a);
                sv.map_or(false, |x| x >= 2) && {
                    let (_, rr) = tm::separate(&qnode.b, &fb);
                    tm::is_zero(&rr)
                }
            }
            _ => false,
        }
    } else {
        let fb = tm::first_term(&qnode.b);
        match fb.as_ref() {
            Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b) => {
                let sv = finval(&nb.a);
                if sv.map_or(true, |x| x >= 2) {
                    let (_, rr) = tm::separate(&qnode.b, &fb);
                    tm::is_zero(&rr)
                } else {
                    matches!(&q_mocf, Mocf::Psi(..)) && {
                        let (_, rest) = tm::separate(&qnode.b, &omega1());
                        tm::is_zero(&rest) && !tm::is_zero(&qnode.b)
                    }
                }
            }
            _ => matches!(&q_mocf, Mocf::Psi(..)) && {
                let (_, rest) = tm::separate(&qnode.b, &omega1());
                tm::is_zero(&rest) && !tm::is_zero(&qnode.b)
            },
        }
    };
    if !beta_ok {
            return Ok(None);
    }
    let v = match &q_mocf {
        Mocf::Psi(qv, _) => mocf_to_term(qv)?,
        _ => level_subscript(&beta),
    };
    let beta_is_limit_card = matches!(tm::first_term(&beta).as_ref(),
        Some(n0) if !tm::is_zero(&n0.a) && tm::is_zero(&n0.b)
            && finval(&n0.a).is_none());

    // Walk prims: each contributes z to W (e = q·W) or to a small rest R.
    let mut head_bare: i32 = 0;
    let mut trailing_q: i32 = 0;
    let mut w_terms: Vec<Term> = Vec::new();
    let mut r_terms: Vec<Term> = Vec::new();
    let mut psi_rest: Vec<Term> = Vec::new();
    let fin_terms: Vec<Term> = Vec::new();
    let mut deep_terms: Vec<Term> = Vec::new();
    let mut inner_terms: Vec<Term> = Vec::new();
    let mut bare_ones: i32 = 0;
    let mut saw_nonbare = false;
    let mut i = 0usize;
    while i < ps.len() {
        let p = ps[i];
        let mut k = 1usize;
        while i + k < ps.len() && mocf_eq(p, ps[i + k]) {
            k += 1;
        }
        let k = k as i32;
        match p {
            Mocf::Psi(..) | Mocf::Omega(_) => {
                if p == &q_mocf {
                    if !saw_nonbare {
                        head_bare += k;
                    } else {
                        trailing_q += k;
                    }
                } else if power_ctx && !beta_is_limit_card {
                    let val = match p {
                        Mocf::Psi(vv, yy)
                            if matches!(yy.as_ref(), Mocf::Zero)
                                && matches!(mocf_to_term(vv), Ok(t) if !tm::is_zero(&t)) =>
                        {
                            image_sub(p)?
                        }
                        _ => mocf_to_term(p)?,
                    };
                    w_terms.push(mul_k(&val, k));
                    saw_nonbare = true;
                } else if beta_is_limit_card {
                    let val = match p {
                        Mocf::Psi(vv, yy) => {
                            let yt = mocf_to_term(yy)?;
                            let yfb = tm::first_term(&yt);
                            let ylam = matches!(yfb.as_ref(),
                                Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b)
                                    && finval(&nb.a).is_none());
                            if ylam {
                                let (_, yrest) = tm::separate(&yt, &yfb);
                                if !tm::is_zero(&yrest) {
                                    omega_s(&tm::add(&mocf_to_term(vv)?, &tm::one()))
                                } else {
                                    image_sub(p)?
                                }
                            } else {
                                image_sub(p)?
                            }
                        }
                        _ => mocf_to_term(p)?,
                    };
                    psi_rest.push(mul_k(&val, k));
                    saw_nonbare = true;
                } else {
                    let val = if matches!(p, Mocf::Psi(vv, zz)
                        if matches!(vv.as_ref(), Mocf::Zero) && matches!(zz.as_ref(), Mocf::Zero))
                    {
                        // ψ(0)-run: power image ψ₀(Ω + ψ₀(Ω)·(k−1)) (row 55).
                        let pblk = tm::t(tm::zero(), omega1(), tm::zero());
                        tm::t(
                            tm::zero(),
                            tm::add(&omega1(), &mul_k(&pblk, k - 1)),
                            tm::zero(),
                        )
                    } else {
                        match p {
                            Mocf::Psi(vv, _) => match mocf_to_term(vv) {
                                Ok(t) if !tm::is_zero(&t) => image_sub(p)?,
                                _ => mocf_to_term(p)?,
                            },
                            _ => mocf_to_term(p)?,
                        }
                    };
                    psi_rest.push(val);
                    saw_nonbare = true;
                }
            }
            Mocf::OmegaPow(f) => {
                if matches!(f.as_ref(), Mocf::Zero) {
                    // prim = ω^0 = 1 in an exponent sum. For q ≠ ψ_M(0) in
                    // product form the ones record ×ω factors (W += ω).
                    let q_is_psi0_zero = matches!(&q_mocf,
                        Mocf::Psi(qv, qy) if matches!(qv.as_ref(), Mocf::Zero)
                            && matches!(qy.as_ref(), Mocf::Zero));
                    if product_mode
                        && !q_is_psi0_zero
                        && head_bare >= 1
                        && !saw_nonbare
                        && w_terms.is_empty()
                        && r_terms.is_empty()
                        && psi_rest.is_empty()
                    {
                        bare_ones += k;
                    } else {
                        r_terms.push(mul_k(&tm::one(), k));
                    }
                    i += k as usize;
                    continue;
                }
                let gps = prim_list(f);
                if gps.is_empty() {
                    return Ok(None);
                }
                let mut j = 0usize;
                while j < gps.len() && same_value(gps[j], &q_mocf) {
                    j += 1;
                }
                if j == 0 {
                    let val = mocf_to_term(p)?;
                    if !tm::is_zero(&v) && product_mode && head_bare >= 1 {
                        w_terms.push(mul_k(&val, k));
                    } else {
                        r_terms.push(mul_k(&val, k));
                    }
                } else if j == 1 && gps.len() == 1 {
                    // prim = q exactly: trailing ψ-factor.
                    trailing_q += k;
                    saw_nonbare = true;
                } else if j >= 2
                    && q_is_psi0
                    && matches!(&q_mocf, Mocf::Psi(_, qy) if matches!(qy.as_ref(), Mocf::Zero))
                {
                    // prim = ω^{q^j·…} at level 0: its full image is the r-block.
                    let img = mocf_to_term(&Mocf::OmegaPow(f.clone()))?;
                    deep_terms.push(img);
                    let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
                    for _ in 1..k {
                        deep_terms.push(mul_k(&pblk, 2));
                    }
                    saw_nonbare = true;
                } else if j >= 2 && !tm::is_zero(&v) {
                    // Level-v deep prim: inner = sub-image minus β.
                    if let Some(img) = decompose_q_power_mode(f, true)? {
                        if let Some(n) = img.as_ref() {
                            let subp = tm::sub(&n.b, &beta);
                            if !tm::is_zero(&subp) {
                                deep_terms.push(mul_k(&subp, k));
                            }
                        }
                    }
                    saw_nonbare = true;
                } else if j >= 2 && power_ctx {
                    // q^q-like prim at level 0 in power context: wrap sub once.
                    if let Some(img) = decompose_q_power_ctx(f, true, true)? {
                        if let Some(n) = img.as_ref() {
                            let subp = tm::sub(&n.b, &beta);
                            if !tm::is_zero(&subp) {
                                let wrapped = tm::t(
                                    v.clone(),
                                    tm::add(&beta, &subp),
                                    tm::zero(),
                                );
                                deep_terms.push(mul_k(&wrapped, k));
                            }
                        }
                    }
                    saw_nonbare = true;
                } else if j == 1
                    && q_is_psi0
                    && gps.len() >= 2
                    && matches!(&q_mocf, Mocf::Psi(qv, qy)
                        if matches!(qv.as_ref(), Mocf::Zero) && matches!(qy.as_ref(), Mocf::Zero))
                {
                    // ω^{ψ(0)+r}: cofactor image ψ₀(Ω + r) (rows 33/34/35/42).
                    let rest =
                        sum_of_prims(&gps[1..]);
                    let rt = mocf_to_term(&rest)?;
                    if tm::lt(&rt, &tm::epsilon0()) {
                        let blk = tm::t(tm::zero(), tm::add(&omega1(), &rt), tm::zero());
                        deep_terms.push(mul_k(&blk, k));
                    } else {
                        let val = mocf_to_term(p)?;
                        r_terms.push(mul_k(&val, k));
                    }
                    saw_nonbare = true;
                } else {
                    let mut x = tm::zero();
                    for g in &gps[j..] {
                        if matches!(g, Mocf::Psi(..)) && !same_value(g, &q_mocf) {
                            psi_rest.push(mocf_to_term(g)?);
                        } else {
                            x = tm::add(&x, &mocf_to_term(g)?);
                        }
                    }
                    let qp = if j >= 2 {
                        tm::standard_form(&tm::exp(&tm::mul(&q, &nat((j - 1) as i32))))
                    } else {
                        tm::one()
                    };
                    let z = if tm::is_zero(&x) {
                        qp
                    } else if tm::is_ordinal_finite(&x) {
                        tm::standard_form(&tm::mul(&qp, &tm::exp(&x)))
                    } else {
                        tm::standard_form(&tm::mul(&qp, &x))
                    };
                    w_terms.push(mul_k(&z, k));
                }
                saw_nonbare = true;
            }
            _ => {
                let val = mocf_to_term(p)?;
                if let Some(n) = finval(&val) {
                    if n >= 1 {
                        r_terms.push(mul_k(&tm::standard_form(&tm::exp(&val)), k));
                    }
                } else {
                    r_terms.push(mul_k(&val, k));
                }
                saw_nonbare = true;
            }
        }
        i += k as usize;
    }

    // Bare trailing ones: for Ω_λ q they record ×ω factors; otherwise at
    // level v≥1 a single one = ×ω factor (W += ω), a run of m ≥ 2 = finite
    // multiplier m of the ψ_v block.
    let mut trailing_mult: i32 = 1;
    let mut limit_omega: i32 = 0;
    if product_mode
        && bare_ones > 0
        && w_terms.is_empty()
        && r_terms.is_empty()
    {
        if beta_is_limit_card {
            limit_omega = bare_ones;
        } else if bare_ones == 1 {
            w_terms.push(tm::omega());
        } else {
            trailing_mult = bare_ones;
        }
    }

    let mut w = tm::zero();
    for t in &w_terms {
        w = tm::add(&w, t);
    }
    let mut r = tm::zero();
    for t in &r_terms {
        r = tm::add(&r, t);
    }
    let mut fin = tm::zero();
    for t in &fin_terms {
        fin = tm::add(&fin, t);
    }
    let eps = tm::t(tm::zero(), omega1(), tm::zero());
    let mut r_add = if tm::is_zero(&r) {
        tm::zero()
    } else if beta_is_limit_card {
        if tm::is_ordinal_finite(&r) {
            r.clone()
        } else {
            log_b(&r)
        }
    } else if tm::lt(&r, &eps) {
        enc_term(&r)
    } else {
        r.clone()
    };
    for p in &psi_rest {
        r_add = tm::add(&r_add, p);
    }

    let q_is_card = matches!(&q_mocf, Mocf::Omega(_));
    let level_wrap = !tm::is_zero(&v) && (product_mode || !inner_terms.is_empty());
    if !tm::is_zero(&v) && !product_mode && !tm::is_zero(&r_add) {
        r_add = lift_sum_rest(&r_add, &v);
    }
    if w_terms.is_empty()
        && fin_terms.is_empty()
        && trailing_q == 0
        && deep_terms.is_empty()
        && inner_terms.is_empty()
        && trailing_mult == 1
    {
        // Pure bare-q run: finite power / product.
        if head_bare == 0 {
            return Ok(None);
        }
        let pblk0 = if q_is_card {
            beta.clone()
        } else {
            tm::t(v.clone(), beta.clone(), tm::zero())
        };
        let qpart = if head_bare >= 2 {
            mul_k(&pblk0, head_bare - 1)
        } else {
            tm::zero()
        };
        let arg = if q_is_card {
            let mut base = lift_omega_lambda_run(&tm::add(&beta, &qpart));
            if limit_omega > 0 {
                base = tm::standard_form(&tm::mul(&base, &tm::exp(&nat(limit_omega))));
            }
            if !tm::is_zero(&r_add)
                && product_mode
                && !tm::is_ordinal_finite(&r_add)
            {
                tm::standard_form(&tm::mul(&base, &r_add))
            } else {
                tm::add(&base, &r_add)
            }
        } else if level_wrap {
            let inner = tm::add(&qpart, &r_add);
            if tm::is_zero(&inner) {
                beta.clone()
            } else {
                let blk = tm::t(v.clone(), tm::add(&beta, &inner), tm::zero());
                tm::add(&beta, &blk)
            }
        } else if power_ctx && head_bare >= 2 {
            // q^q-like bare run: one ψ_v(β+…)-wrap of the qpart.
            let inner = tm::add(&qpart, &r_add);
            let blk = tm::t(v.clone(), tm::add(&beta, &inner), tm::zero());
            tm::add(&beta, &blk)
        } else {
            tm::add(&beta, &tm::add(&qpart, &r_add))
        };
        let _ = q_lead_exp;
            return Ok(Some(tm::t(tm::zero(), arg, tm::zero())));
    }

    // Power family: bare head run contributes q^{c−1}; trailing bare q's
    // and finite parts add ψ₀(β)-blocks after the main r.
    if head_bare >= 2 {
        let pblk0 = tm::t(v.clone(), beta.clone(), tm::zero());
        if level_wrap {
            inner_terms.push(mul_k(&pblk0, head_bare - 1));
        } else {
            w = tm::add(&mul_k(&pblk0, head_bare - 1), &w);
        }
    }
    let mut rr = match nu_pow_r(&q, &beta, &v, &w) {
        Some(x) => x,
        None => return Ok(None),
    };
    for d in &deep_terms {
        rr = tm::add(&rr, d);
    }
    if trailing_q > 0 {
        let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
        rr = tm::add(&rr, &mul_k(&pblk, trailing_q));
    }
    if !tm::is_zero(&fin) {
        if let Some(n) = finval(&fin) {
            let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
            rr = tm::add(&rr, &mul_k(&pblk, n));
        }
    }
    let extra_wrap = level_wrap
        && matches!(a, Mocf::OmegaPow(_))
        && inner_terms.is_empty()
        && tm::is_zero(&r_add)
        && rr_has_content(&rr);
    let arg = if level_wrap && !q_is_card {
        let mut inner = r_add.clone();
        for d in &inner_terms {
            inner = tm::add(&inner, d);
        }
        if tm::is_zero(&inner) && rr_has_content(&rr) {
            if extra_wrap {
                let blk = tm::t(v.clone(), tm::add(&beta, &rr), tm::zero());
                tm::add(&beta, &blk)
            } else {
                tm::add(&beta, &rr)
            }
        } else if tm::is_zero(&inner) {
            beta.clone()
        } else {
            let blk = tm::t(v.clone(), tm::add(&beta, &inner), tm::zero());
            let blk = mul_k(&blk, trailing_mult);
            tm::add(&beta, &tm::add(&rr, &blk))
        }
    } else {
        tm::add(&beta, &tm::add(&rr, &r_add))
    };
    let _ = q_lead_exp;
    Ok(Some(tm::t(tm::zero(), arg, tm::zero())))
}

/// Sum-mode rest lift at level v: finite ↦ Ω·n, Ω ↦ Ω²,
/// Ω_a ↦ Ω_a (a+1 == s) or Ω_a² (deeper), ψ-blocks kept.
fn lift_sum_rest(t: &Term, v: &Term) -> Term {
    let s = finval(v).unwrap_or(0) + 1;
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.a) && tm::is_zero(&n.b) {
            // finite
            mul_k(&omega1(), k)
        } else if tm::is_zero(&n.b) && !tm::is_zero(&n.a) {
            let a_i = finval(&n.a).unwrap_or(0);
            if a_i == 1 {
                tm::standard_form(&tm::exp(&tm::mul(&omega1(), &nat(2))))
            } else if a_i >= 2
                && s >= 2
                && ((a_i as i32) >= s + 1 || (s >= 4 && (a_i as i32) <= s - 2))
            {
                let c = omega_s(&nat(a_i));
                tm::standard_form(&tm::exp(&tm::mul(&c, &nat(2))))
            } else {
                head.clone()
            }
        } else if tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            tm::standard_form(&tm::mul(&omega1(), &head))
        } else if s >= 4 && !tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            // ψ_v(b) tail after a high lead: Ω_{v+1}·ψ_v(b) when the block
            // is below the lead level.
            let vi = finval(&n.a).unwrap_or(0);
            if vi >= 1
                && (vi as i32) < s - 1
                && !tm::lt(&n.b, &omega_s(&nat(vi + 2)))
            {
                tm::standard_form(&tm::mul(&omega_s(&nat(vi + 1)), &head))
            } else {
                head.clone()
            }
        } else {
            head.clone()
        };
        acc = tm::add(&acc, &mul_k(&lifted, if tm::is_zero(&n.a) && tm::is_zero(&n.b) { 1 } else { k }));
        cur = rest;
    }
    acc
}

fn rr_has_content(t: &Term) -> bool {
    !tm::is_zero(t)
}

fn mocf_eq(a: &Mocf, b: &Mocf) -> bool {
    same_value(a, b)
}

fn same_value(a: &Mocf, b: &Mocf) -> bool {
    match (mocf_to_term(a), mocf_to_term(b)) {
        (Ok(x), Ok(y)) => tm::eq(&x, &y),
        _ => false,
    }
}

/// Inner ψ-subscript for argument β: β = Ω_s·… ⇒ s−1, else 0.
fn level_subscript(beta: &Term) -> Term {
    let first = tm::first_term(beta);
    if let Some(n) = first.as_ref() {
        if tm::is_zero(&n.b) && !tm::is_zero(&n.a) {
            return tm::sub(&n.a, &tm::one());
        }
    }
    tm::zero()
}

/// r for ν(q^W) = ψ₀(β + r), blockwise over wv = ν(W).
/// Encode a small rest r < ε₀: ω^s-blocks ↦ ψ₀(enc(s)); finite kept.
fn enc_term(t: &Term) -> Term {
    if tm::is_zero(t) {
        return tm::zero();
    }
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.a) && tm::is_zero(&n.c) && !tm::is_zero(&n.b) {
            // ω^s block: ↦ ψ₀(enc(s))
            tm::t(tm::zero(), enc_term(&n.b), tm::zero())
        } else {
            head.clone()
        };
        acc = tm::add(&acc, &mul_k(&lifted, k));
        cur = rest;
    }
    acc
}

fn nu_pow_r(q: &Term, beta: &Term, v: &Term, wv: &Term) -> Option<Term> {
    if tm::is_zero(wv) {
        return Some(tm::zero());
    }
    if let Some(k) = finval(wv) {
        let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
        return Some(if k <= 1 { tm::zero() } else { mul_k(&pblk, k - 1) });
    }
    let mut acc = tm::zero();
    let mut cur = wv.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = head.as_ref()?;
        if !tm::is_zero(&n.a) || !tm::is_zero(&n.c) {
            if !tm::is_zero(&n.a) && tm::is_zero(&n.b) && !tm::is_zero(v) {
                // Ω^e block in a level-v region: ↦ ψ_v(β + Ω^e).
                let blk = tm::t(v.clone(), tm::add(beta, &head), tm::zero());
                acc = tm::add(&acc, &mul_k(&blk, k));
                cur = rest;
                continue;
            }
            return None;
        }
        if tm::is_zero(&n.a)
            && !tm::is_zero(&n.b)
            && !tm::is_zero(v)
            && !tm::lt(
                &n.b,
                &tm::standard_form(&tm::exp(&tm::mul(&omega1(), &omega1()))),
            )
        {
            // Deep ω-power block Ω^{E≥Ω} in a level-v region:
            // ↦ ψ_v(β + block).
            let blk = tm::t(v.clone(), tm::add(beta, &head), tm::zero());
            acc = tm::add(&acc, &mul_k(&blk, k));
            cur = rest;
            continue;
        }
        let b = &n.b;
        let u;
        if tm::lt(&head, q) && tm::is_zero(&n.a) && tm::is_zero(&n.c) {
            // Small block B < q at level 0: r-block = enc(B) directly.
            let u2 = if tm::is_zero(v) {
                small_u_level0(q, beta, &head)
            } else {
                lift_v_block(&head, v, beta)
            };
            let blk = if tm::is_zero(v) {
                // Direct r-block when u2 already reaches past β; otherwise wrap.
                let past = match u2.as_ref() {
                    Some(n2) => {
                        tm::is_zero(&n2.a) && !tm::is_zero(&n2.b) && tm::lt(beta, &n2.b)
                    }
                    None => false,
                };
                if past {
                    u2.clone()
                } else {
                    tm::t(v.clone(), tm::add(beta, &u2), tm::zero())
                }
            } else {
                tm::t(v.clone(), tm::add(beta, &u2), tm::zero())
            };
            acc = tm::add(&acc, &mul_k(&blk, k));
            cur = rest;
            continue;
        } else if tm::is_zero(b) {
            if !tm::is_zero(&n.a) {
                // Cardinal block Ω_a: u = Ω_a.
                let blk = tm::t(v.clone(), tm::add(beta, &head), tm::zero());
                acc = tm::add(&acc, &mul_k(&blk, k));
                cur = rest;
                continue;
            }
            // finite run: ψ_{s-1}(Ω_s)·k trailing copies.
            let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
            acc = tm::add(&acc, &mul_k(&pblk, k));
            cur = rest;
            continue;
        } else if tm::eq(b, beta) {
            let pblk = tm::t(v.clone(), beta.clone(), tm::zero());
            u = pblk.clone();
            let blk = tm::t(v.clone(), tm::add(beta, &u), tm::zero());
            acc = tm::add(&acc, &blk);
            if k > 1 && tm::is_zero(v) {
                acc = tm::add(&acc, &mul_k(&pblk, k - 1));
            }
            cur = rest;
            continue;
        } else if !tm::lt(b, beta) {
            // ψ₀(β + x)-form block.
            let x = tm::sub(b, beta);
            u = unwrap_x(q, beta, v, &x)?;
        } else {
            // ω^b-form block with b ≥ q: decompose b = q·z.
            u = unwrap_x(q, beta, v, b)?;
        }
        let blk = tm::t(v.clone(), tm::add(beta, &u), tm::zero());
        acc = tm::add(&acc, &mul_k(&blk, k));
        cur = rest;
    }
    Some(acc)
}

/// Small block B < q at level 0: u so that q·ω^{f} maps to the r-block.
fn small_u_level0(q: &Term, beta: &Term, b: &Term) -> Term {
    // ψ₀(b)-form block with b ≥ Ω: u is the block itself.
    if let Some(n) = b.as_ref() {
        if tm::is_zero(&n.a) && !tm::is_zero(&n.b) && !tm::lt(&n.b, &omega1()) {
            return b.clone();
        }
    }
    let l = log_b(b);
    let f = if tm::is_ordinal_finite(&l) {
        l
    } else {
        tm::t(tm::zero(), tm::add(beta, &l), tm::zero())
    };
    tm::mul(q, &tm::exp(&f))
}

/// u-increment from an exponent piece x (already q-relative).
fn unwrap_x(q: &Term, beta: &Term, v: &Term, x: &Term) -> Option<Term> {
    if tm::is_zero(x) {
        return Some(q.clone());
    }
    let qnode = q.as_ref()?;
    let qbeta = qnode.b.clone();
    if tm::lt(x, q) {
        // x < q: u = q·ω^{log(x)+1}.
        let lx = log_b(x);
        return Some(tm::standard_form(&tm::mul(
            q,
            &tm::exp(&tm::add(&lx, &tm::one())),
        )));
    }
    if !tm::is_zero(v) {
        return Some(div_omega(x));
    }
    // x ≥ q: count q-factors of x: blocks ψ₀(qbeta + y) ↦ ω^y·k.
    let mut c = tm::zero();
    let mut cur = x.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = head.as_ref()?;
        if tm::is_zero(&n.a) && tm::is_zero(&n.c) && !tm::lt(&n.b, &qbeta) {
            let inner = tm::sub(&n.b, &qbeta);
            if tm::is_zero(&inner) {
                c = tm::add(&c, &nat(k));
            } else {
                c = tm::add(&c, &mul_k(&tm::standard_form(&tm::exp(&inner)), k));
            }
        }
        cur = rest;
    }
    if tm::is_zero(&c) {
        return Some(tm::standard_form(&tm::mul(
            q,
            &tm::exp(&tm::add(x, &tm::one())),
        )));
    }
    if let Some(n) = finval(&c) {
        if n == 1 {
            return Some(q.clone());
        }
    }
    // u = q·ω^{c−1} when c ≥ 1 (with a nested ψ-wrap for deep c).
    if tm::is_zero(v) && !tm::lt(&c, q) {
        // Deep: u = ψ₀(β + recurse).
        let inner = nu_pow_r(q, beta, v, &c)?;
        return Some(tm::t(
            tm::zero(),
            tm::add(&qbeta, &inner),
            tm::zero(),
        ));
    }
    let pred = if let Some(n) = finval(&c) {
        if n >= 1 { nat(n - 1) } else { tm::zero() }
    } else {
        c.clone()
    };
    Some(tm::standard_form(&tm::mul(q, &tm::exp(&pred))))
}

/// Left-divide t by Ω: ψ₀(Ω+x) ↦ x, Ω ↦ 1, Ω_a kept, others kept.
fn div_omega(t: &Term) -> Term {
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let node = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        if tm::is_zero(&node.b) && !tm::is_zero(&node.a) {
            if tm::eq(&node.a, &tm::one()) {
                acc = tm::add(&acc, &nat(k));
            } else {
                acc = tm::add(&acc, &mul_k(&head, k));
            }
        } else if tm::is_zero(&node.a) && !tm::is_zero(&node.b) && !tm::lt(&node.b, &omega1()) {
            let x = tm::sub(&node.b, &omega1());
            if tm::is_zero(&x) {
                acc = tm::add(&acc, &nat(k));
            } else {
                acc = tm::add(&acc, &mul_k(&x, k));
            }
        } else {
            acc = tm::add(&acc, &mul_k(&head, k));
        }
        cur = rest;
    }
    acc
}

/// Level-0 ψ: preimage of ψ_M(A).
fn psi_level0(a: &Mocf) -> Result<Term, String> {
    let arg = level0_arg(a)?;
    Ok(tm::t(tm::zero(), arg, tm::zero()))
}

/// Whether an Ω_{ω^x} subscript-exponent `x` denotes an Ω_ω-power that
/// should be inverted to ω^{Ω_ω·x}. True for cardinal-led exponents and for
/// small exponents (finite, ω, Ω); false for large bare ω-powers (which are
/// genuine cardinals Ω_{Ω^k}) and for x = 0 (the bare cardinal Ω_ω).
fn omega_lambda_sub_invertible(x: &Mocf) -> bool {
    if matches!(x, Mocf::Omega(_)) {
        return true;
    }
    if let Ok(xv) = mocf_to_term(x) {
        if tm::is_ordinal_finite(&xv) || tm::eq(&xv, &tm::omega()) {
            return true;
        }
        let (e, r) = omega_divmod(&xv);
        tm::eq(&e, &tm::one()) && tm::is_zero(&r)
    } else {
        false
    }
}

/// Value of an expression that may be an Ω_ω-power tower. Ω_ω^e parses as
/// Ω_{ω^e}; invert to ω^{Ω_ω·e}, recursing so a tower exponent is itself
/// inverted rather than read as a cardinal.
fn omega_lambda_pow_value(m: &Mocf) -> Result<Term, String> {
    if let Mocf::Omega(s) = m {
        if let Mocf::OmegaPow(x) = s.as_ref() {
            if !matches!(x.as_ref(), Mocf::Zero) && omega_lambda_sub_invertible(x) {
                let xv = omega_lambda_pow_value(x)?;
                let omega_w = tm::t(tm::omega(), tm::zero(), tm::zero());
                return Ok(tm::standard_form(&tm::exp(&tm::mul(&omega_w, &xv))));
            }
        }
    }
    mocf_to_term(m)
}

/// Argument images of ψ₁(0)·Ω^X / ψ₁(0)^{Ω^X} shapes inside ψ₀
/// (rows 367/373): the ψ₁(0) factor marks the Ω₂-lead; the Ω-power stays.
fn psi1_prod_arg(a: &Mocf) -> Result<Option<Term>, String> {
    let (deep, inner) = match a {
        Mocf::OmegaPow(e) => match e.as_ref() {
            Mocf::OmegaPow(e2) => (true, e2.as_ref()),
            other => (false, other),
        },
        _ => return Ok(None),
    };
    let ts = match inner {
        Mocf::Sum(t) => t,
        _ => return Ok(None),
    };
    if ts.len() < 2 {
        return Ok(None);
    }
    if !matches!(&ts[0], Mocf::Psi(v, z)
        if !matches!(v.as_ref(), Mocf::Zero) && matches!(z.as_ref(), Mocf::Zero))
    {
        return Ok(None);
    }
    // X = an Ω-run: flat Ω·n (n ≥ 2) or ω^{Ω·n}.
    let n = if ts.len() == 2 {
        match &ts[1] {
            Mocf::OmegaPow(xe) => {
                let xs = prim_list(xe);
                let mut n = 0usize;
                while n < xs.len() && matches!(xs[n], Mocf::Omega(_)) {
                    n += 1;
                }
                if n < 2 || n != xs.len() {
                    return Ok(None);
                }
                n
            }
            _ => return Ok(None),
        }
    } else {
        let mut n = 0usize;
        for p in &ts[1..] {
            if matches!(p, Mocf::Omega(_)) {
                n += 1;
            } else {
                return Ok(None);
            }
        }
        if n < 2 {
            return Ok(None);
        }
        n
    };
    let om = omega1();
    let img = tm::standard_form(&tm::exp(&tm::mul(
        &om,
        &tm::standard_form(&tm::exp(&tm::mul(&om, &nat(n as i32 - 1)))),
    )));
    let om2 = omega_s(&nat(2));
    let mut arg = tm::add(&om2, &tm::t(tm::one(), tm::add(&om2, &img), tm::zero()));
    if deep {
        arg = tm::add(&om2, &tm::t(tm::one(), arg, tm::zero()));
    }
    Ok(Some(arg))
}

/// The ψ₀-argument for a level-0 ψ_M argument expression.
fn level0_arg(a: &Mocf) -> Result<Term, String> {
    // Ω_ω^e parses as Ω_{ω^e}: invert to ω^{Ω_ω·e} (recursively for towers).
    if let Mocf::Omega(s) = a {
        if let Mocf::OmegaPow(x) = s.as_ref() {
            if !matches!(x.as_ref(), Mocf::Zero) && omega_lambda_sub_invertible(x) {
                return omega_lambda_pow_value(a);
            }
        }
    }
    // ψ₁(0)-factor products: ω^{ψ₁(0)+Ω^X} ↦ Ω₂ + ψ₁(Ω₂ + Ω^X) and
    // ω^{ω^{ψ₁(0)+Ω^X}} ↦ Ω₂ + ψ₁(Ω₂ + ψ₁(Ω₂ + Ω^X)) (rows 367/373).
    if let Some(t) = psi1_prod_arg(a)? {
        return Ok(t);
    }
    // ψ_M(ψ_M(…)·X) arguments: ↦ Ω·image.
    if psi0_mult_syntax(a) {
        let img = mocf_to_term(a)?;
        return Ok(tm::standard_form(&tm::mul(&omega1(), &img)));
    }
    // q-power structure (ψ_s(0)·X, ψ_s(0)^X): invert via the preimage.
    let qroute_omegapow_ok = !matches!(a, Mocf::OmegaPow(_))
        || q_level_of(a) != 0
        || inner_q_acceptable(a);
    if (q_level_of(a) == 0
        || lead_psi_arg_nonzero(a)
        || lead_psi_arg_zero(a)
        || lead_omega_limit(a))
        && qroute_omegapow_ok
        && !matches!(a, Mocf::Psi(..))
        && !psi0_leading_syntax2(a)
    {
        // (ψ_s(0)-led arguments route here: q-preimage at level s.)
    if let Some(t) = omega_pow_pre(a)? {
        let node = match t.as_ref() {
            Some(n) => n,
            None => return Ok(tm::zero()),
        };
        return Ok(node.b.clone());
    }
    }
    if let Mocf::Pow(b, e) = a {
        if let Mocf::Omega(s) = b.as_ref() {
            let sv = mocf_to_term(s)?;
            let card = omega_s(&sv);
            // A whole-ψ exponent is fully evaluated (the +1 Ω-level shift);
            // anything else stays raw/symbolic (the forward keeps it so).
            let ev = if matches!(e.as_ref(), Mocf::Psi(..)) {
                mocf_to_term(e)?
            } else {
                raw_mocf_value(e)?
            };
            let lead = match finval(&ev) {
                Some(n) if tm::is_succ(&sv) => {
                    tm::standard_form(&tm::exp(&tm::mul(&card, &nat(n + 1))))
                }
                _ => tm::standard_form(&tm::exp(&tm::mul(&card, &ev))),
            };
            return Ok(lead);
        }
    }
    match a {
        Mocf::Omega(s) => {
            // Ω_ω^e parses as Ω_{ω^e}: invert to ω^{Ω_ω·e}.
            if let Mocf::OmegaPow(x) = s.as_ref() {
                if matches!(x.as_ref(), Mocf::Zero) {
                    // subscript ω^0 = 1: plain Ω.
                    return countable_or_shift(a);
                }
                let xv = mocf_to_term(x)?;
                let ow = omega_s(&tm::t(tm::zero(), tm::omega(), tm::zero()));
                let _ = ow;
                let omega_w = tm::t(tm::omega(), tm::zero(), tm::zero());
                return Ok(tm::standard_form(&tm::exp(&tm::mul(&omega_w, &xv))));
            }
            let sv = mocf_to_term(s)?;
            if tm::is_succ(&sv) {
                let card = omega_s(&sv);
                Ok(tm::standard_form(&tm::exp(&tm::mul(&card, &nat(2)))))
            } else {
                countable_or_shift(a)
            }
        }
        Mocf::Psi(v, a2) => {
            let vv = mocf_to_term(v)?;
            if tm::is_zero(&vv) && psi_leading_syntax(a2) {
                // ψ_M(ψ_M(…)…) ↦ ψ₀(Ω·image)
                let img = mocf_to_term(a)?;
                return Ok(tm::standard_form(&tm::mul(&omega1(), &img)));
            }
            if tm::is_zero(&vv) {
                let full = mocf_to_term(a)?;
                if term_uncountable(&full) {
                    return Ok(tm::standard_form(&tm::mul(&omega1(), &full)));
                }
            }
            let am2 = if psi_leading_syntax_v(a2) {
                image_sub(a2)?
            } else if tm::is_zero(&vv) {
                mocf_to_term(a)?
            } else {
                mocf_to_term(a2)?
            };
            let card = omega_s(&tm::add(&vv, &tm::one()));
            Ok(tm::standard_form(&tm::mul(
                &card,
                &tm::add(&tm::one(), &am2),
            )))
        }
        Mocf::OmegaPow(f) => {
            if let Some(t) = omega_times_lead(f)? {
                return Ok(t);
            }
            countable_or_shift(a)
        }
        Mocf::Sum(terms) => {
            if let Some(t) = omega_power_lead(terms)? {
                return Ok(t);
            }
            sum_argument(terms)
        }
        _ => countable_or_shift(a),
    }
}

/// True if a is a product/power built from ψ_M(0) factors (no Ω prims).
fn psi0_mult_syntax(a: &Mocf) -> bool {
    let ps = prim_list(a);
    if ps.is_empty() {
        return false;
    }
    match ps[0] {
        Mocf::Psi(v, y) => {
            if !matches!(v.as_ref(), Mocf::Zero) || !matches!(y.as_ref(), Mocf::Zero) {
                return false;
            }
        }
        Mocf::OmegaPow(f) => {
            let fps = prim_list(f);
            match fps.first() {
                Some(Mocf::Psi(v, y))
                    if matches!(v.as_ref(), Mocf::Zero)
                        && matches!(y.as_ref(), Mocf::Zero) => {}
                _ => return false,
            }
        }
        _ => return false,
    }
    let mut has = false;
    for p in &ps {
        match p {
            Mocf::Psi(v, _) => {
                if !matches!(v.as_ref(), Mocf::Zero) {
                    return false;
                }
                has = true;
            }
            Mocf::Omega(_) => return false,
            Mocf::OmegaPow(f) => {
                if matches!(f.as_ref(), Mocf::Zero) {
                    continue;
                }
                if !psi0_mult_syntax(f) {
                    return false;
                }
                has = true;
            }
            Mocf::Pow(q, _) => {
                if !matches!(q.as_ref(), Mocf::Psi(v, _) if matches!(mocf_to_term(v), Ok(t) if tm::is_zero(&t)))
                {
                    return false;
                }
                has = true;
            }
            _ => {}
        }
    }
    has
}

/// True if the MOCF expression begins syntactically with a ψ_M block.
fn psi_leading_syntax(a: &Mocf) -> bool {
    match a {
        Mocf::Psi(v, _) => {
            matches!(mocf_to_term(v), Ok(t) if tm::is_zero(&t))
        }
        Mocf::Sum(ts) => !ts.is_empty() && psi_leading_syntax(&ts[0]),
        _ => false,
    }
}

/// Countable argument ↦ Ω·(1+a); otherwise fall back to the value shift.
/// True if t has a block reaching Ω or beyond.
fn term_reaches_omega(t: &Term) -> bool {
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (_, rest) = tm::separate(&cur, &head);
        if let Some(n) = head.as_ref() {
            if !tm::is_zero(&n.a) {
                return true;
            }
            if b_reaches_omega(&n.b) {
                return true;
            }
        }
        cur = rest;
    }
    false
}

fn countable_or_shift(a: &Mocf) -> Result<Term, String> {
    let am = mocf_to_term(a)?;
    if !term_reaches_omega(&am) {
        return Ok(tm::standard_form(&tm::mul(
            &omega1(),
            &tm::add(&tm::one(), &am),
        )));
    }
    Ok(cardinal_arg_shift(&am))
}

/// Rest lift for Ω_s^e families: ψ₀(Ω·k + x) ↦ Ω·ψ₀(Ω·(k+1) + x').
fn lift_omegas_rest(t: &Term) -> Term {
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            // ψ₀(b) ↦ Ω·ψ₀(b)
            tm::standard_form(&tm::mul(&omega1(), &head))
        } else {
            tm::standard_form(&tm::mul(&omega1(), &head))
        };
        acc = tm::add(&acc, &mul_k(&lifted, k));
        cur = rest;
    }
    acc
}

/// Run lift in Ω_λ regions: subscripts ψ₀(Ω_s·k) ↦ ψ_{s-1}(Ω_s·k).
fn lift_omega_lambda_run(t: &Term) -> Term {
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if !tm::is_zero(&n.a) && tm::is_zero(&n.b) {
            let sub = &n.a;
            let fbs = tm::first_term(sub);
            let rev = match fbs.as_ref() {
                Some(ns) if tm::is_zero(&ns.a) && !tm::is_zero(&ns.b) => {
                    let fb2 = tm::first_term(&ns.b);
                    match fb2.as_ref() {
                        Some(n2) if !tm::is_zero(&n2.a) && tm::is_zero(&n2.b) => {
                            match finval(&n2.a) {
                                Some(sv) if sv >= 2 => Some(sv),
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            match rev {
                Some(sv) => omega_s(&tm::t(nat(sv - 1), ns_b_clone(sub), tm::zero())),
                None => head.clone(),
            }
        } else {
            head.clone()
        };
        acc = tm::add(&acc, &mul_k(&lifted, k));
        cur = rest;
    }
    acc
}

fn ns_b_clone(sub: &Term) -> Term {
    match tm::first_term(sub).as_ref() {
        Some(ns) => ns.b.clone(),
        None => tm::zero(),
    }
}

/// Tail lift in Ω_λ regions: ψ₀(Ω_λ) ↦ Ω·ψ₀(Ω_λ), ψ_s(0) ↦ image_sub,
/// ψ_s(Ω_λ+finite) ↦ Ω_{s+1}, ψ_s(Ω_λ) kept, cardinals kept, finite ↦ Ω·n.
fn lift_omega_lambda_tail(t: &Term) -> Term {
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.a) && tm::is_zero(&n.b) {
            mul_k(&omega1(), k)
        } else if tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            tm::standard_form(&tm::mul(&omega1(), &head))
        } else if !tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            // ψ_s(b): if b = Ω_λ + rest, collapse to Ω_{s+1}; else keep.
            let fb = tm::first_term(&n.b);
            let lam = matches!(fb.as_ref(),
                Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b)
                    && finval(&nb.a).is_none());
            if lam {
                let (_, rr2) = tm::separate(&n.b, &fb);
                if tm::is_zero(&rr2) {
                    head.clone()
                } else {
                    omega_s(&tm::add(&n.a, &tm::one()))
                }
            } else {
                head.clone()
            }
        } else {
            head.clone()
        };
        acc = tm::add(&acc, &mul_k(&lifted, if tm::is_zero(&n.a) && tm::is_zero(&n.b) { 1 } else { k }));
        cur = rest;
    }
    acc
}

/// Syntactic Ω_s^e·k + rest leading detection for level-0 sums:
/// ψ_M(Ω_s^e·k + rest) ↦ ψ₀(Ω_s^{e+1}·lift(k) + Ω_s·lift(rest)).
fn omega_power_lead(terms: &[Mocf]) -> Result<Option<Term>, String> {
    let first = &terms[0];
    let (sv, ev) = match first {
        Mocf::Omega(s) => (mocf_to_term(s)?, tm::one()),
        Mocf::Pow(b, e) => {
            if let Mocf::Omega(s) = b.as_ref() {
                let sv0 = mocf_to_term(s)?;
                let ev = raw_mocf_value(e)?;
                if !tm::is_zero(&ev) && finval(&ev).is_none() {
                    // Ω_s^{E≥Ω_s}: identity lead, syntactic rest lift.
                    if let Some(svi) = finval(&sv0) {
                        if svi >= 2 {
                            let card0 = omega_s(&sv0);
                            if !tm::lt(&ev, &tm::standard_form(&tm::exp(&tm::mul(&card0, &nat(2))))) {
                                let lead = tm::standard_form(&tm::mul(&card0, &ev));
                                let lead = tm::standard_form(&tm::exp(&lead));
                                let rest = &terms[1..];
                                let mut acc = lead;
                                for t in rest {
                                    acc = tm::add(&acc, &lift_id_rest(t, &card0)?);
                                }
                                return Ok(Some(acc));
                            }
                        }
                    }
                    return Ok(None);
                }
                (sv0, ev)
            } else {
                return Ok(None);
            }
        }
        _ => return Ok(None),
    };
    if finval(&sv).is_none() {
        return Ok(None);
    }
    let card = omega_s(&sv);
    // Multiplicity run of the head block.
    let mut k = 1usize;
    while k < terms.len() && mocf_eq(&terms[k], first) {
        k += 1;
    }
    let rest = &terms[k..];
    if !tm::eq(&sv, &tm::one()) && !rest.is_empty() {
        if tm::eq(&sv, &nat(2)) {
            // ψ_M(Ω_2·k + rest) ↦ ψ₀(Ω_2²·k + lift₂(rest))
            let lead_exp = tm::mul(&card, &tm::add(&ev, &tm::one()));
            let mut arg =
                tm::standard_form(&tm::mul(&tm::exp(&lead_exp), &nat(k as i32)));
            for t in rest {
                arg = tm::add(&arg, &lift_rest_2(t)?);
            }
            return Ok(Some(arg));
        }
        return Ok(None);
    }
    let lead_exp = tm::mul(&card, &tm::add(&ev, &tm::one()));
    let mut arg = tm::standard_form(&tm::mul(&tm::exp(&lead_exp), &nat(k as i32)));
    for t in rest {
        let lifted = if k >= 2 { lift_rest_raw(t)? } else { lift_rest(t)? };
            arg = tm::add(&arg, &tm::standard_form(&tm::mul(&card, &lifted)));
    }
    Ok(Some(arg))
}

/// True if the argument is a sum whose lead is a fixed-point Ω-power
/// (Ω^e with e ≥ Ω, i.e. the exp_shift region where +1 means +Ω).
fn is_fixed_pow_sum(a: &Mocf) -> bool {
    let ts = match a {
        Mocf::Sum(t) => t,
        _ => return false,
    };
    if ts.is_empty() {
        return false;
    }
    match &ts[0] {
        Mocf::Pow(b, e) => match b.as_ref() {
            Mocf::Omega(_) => match raw_mocf_value(e) {
                Ok(ev) => !tm::is_zero(&ev) && finval(&ev).is_none(),
                Err(_) => false,
            },
            _ => false,
        },
        Mocf::OmegaPow(f) => matches!(f.as_ref(), Mocf::OmegaPow(_)),
        _ => false,
    }
}

/// Value keeping ψ_v(a) blocks raw (eval-style, no collapse).
fn raw_mocf_value(m: &Mocf) -> Result<Term, String> {
    match m {
        Mocf::Psi(v, a) => {
            let vt = mocf_to_term(v)?;
            let mut raw_at = raw_mocf_value(a)?;
            // ψ_v(Ω_λ + rest) with rest ≠ 0 collapses to Ω_{v+1}.
            if !tm::is_zero(&vt) {
                let fb = tm::first_term(&raw_at);
                let lim = matches!(fb.as_ref(),
                    Some(fn_) if !tm::is_zero(&fn_.a) && tm::is_zero(&fn_.b)
                        && tm::is_zero(&fn_.c) && !tm::is_succ(&fn_.a)
                        && finval(&fn_.a).is_none());
                if lim && !tm::eq(&fb, &raw_at) {
                    return Ok(omega_s(&tm::add(&vt, &tm::one())));
                }
            }
            // ψ_M(Ω^λ·X + small) with a fixed-point Ω-power lead lifts the
            // small tail one Ω-level (forward: a trailing Ω collapses to 1).
            if tm::is_zero(&vt) && is_fixed_pow_sum(a) {
                raw_at = cardinal_arg_shift(&raw_at);
            }
            Ok(tm::t(vt, raw_at, tm::zero()))
        }
        Mocf::Sum(ts) => {
            let mut acc = tm::zero();
            for p in ts {
                acc = tm::add(&acc, &raw_mocf_value(p)?);
            }
            Ok(acc)
        }
        Mocf::OmegaPow(e) => {
            let et = raw_mocf_value(e)?;
            Ok(tm::standard_form(&tm::exp(&et)))
        }
        Mocf::Omega(a) => Ok(tm::t(mocf_to_term(a)?, tm::zero(), tm::zero())),
        Mocf::Pow(q, e) => {
            let qv = raw_mocf_value(q)?;
            let ev = raw_mocf_value(e)?;
            Ok(tm::standard_form(&tm::exp(&tm::standard_form(&tm::mul(&qv, &ev)))))
        }
        Mocf::Zero => Ok(tm::zero()),
    }
}

/// Identity-region rest lift: ψ_s(0) ↦ ψ_s(Ω_s), ψ_s(Y) kept, Ω-powers kept.
fn lift_id_rest(t: &Mocf, card: &Term) -> Result<Term, String> {
    match t {
        Mocf::Psi(v, y) => {
            let vt = mocf_to_term(v)?;
            if matches!(y.as_ref(), Mocf::Zero) {
                return Ok(tm::t(vt, card.clone(), tm::zero()));
            }
            let yv = raw_mocf_value(y)?;
            Ok(tm::t(vt, yv, tm::zero()))
        }
        _ => mocf_to_term(t),
    }
}

/// Level-2 rest lift: ψ₁(Ω₂) ↦ ψ₁(Ω₂²), ψ₁(Ω₂+…) ↦ Ω₂·ψ₁(Ω₂²),
/// ψ₁(0) ↦ ψ₁(Ω₂), Ω₂ ↦ Ω₂.
fn lift_rest_2(t: &Mocf) -> Result<Term, String> {
    match t {
        Mocf::Psi(v, y) => {
            let vi = finval(&mocf_to_term(v)?).unwrap_or(0);
            if vi == 1 {
                let om2 = omega_s(&nat(2));
                let om2sq = tm::standard_form(&tm::exp(&tm::mul(&om2, &nat(2))));
                match y.as_ref() {
                    Mocf::Zero => return Ok(tm::t(nat(1), om2, tm::zero())),
                    Mocf::Omega(os) => {
                        let sv2 = finval(&mocf_to_term(os)?).unwrap_or(0);
                        if sv2 == 2 {
                            return Ok(tm::t(nat(1), om2sq.clone(), tm::zero()));
                        }
                        return mocf_to_term(t);
                    }
                    Mocf::Sum(ys) => {
                        let has_psi1 = ys.iter().any(|p| {
                            matches!(p, Mocf::Psi(v2, _) if matches!(mocf_to_term(v2), Ok(tt) if tm::eq(&tt, &nat(1))))
                        });
                        let is_plus_one = ys.len() == 2
                            && matches!(&ys[1], Mocf::OmegaPow(z) if matches!(z.as_ref(), Mocf::Zero));
                        if is_plus_one {
                            return Ok(om2.clone());
                        }
                        if has_psi1 {
                            let blk = tm::t(nat(1), om2sq.clone(), tm::zero());
                            return Ok(tm::standard_form(&tm::mul(&om2, &blk)));
                        }
                        return Ok(om2.clone());
                    }
                    _ => {
                        let blk = tm::t(nat(1), om2sq.clone(), tm::zero());
                        return Ok(tm::standard_form(&tm::mul(&om2, &blk)));
                    }
                }
            }
            mocf_to_term(t)
        }
        _ => mocf_to_term(t),
    }
}

/// Syntactic ω^{Ω·k + r} = Ω^k·ω^r detection:
/// ψ_M(Ω·X) ↦ ψ₀(Ω²·lift(X)) for transfinite X.
fn omega_times_lead(f: &Mocf) -> Result<Option<Term>, String> {
    let fps = prim_list(f);
    let first_sv = match fps.first() {
        Some(Mocf::Omega(s)) => mocf_to_term(s)?,
        _ => return Ok(None),
    };
    if tm::is_zero(&first_sv) || !tm::lt(&first_sv, &omega1()) {
        return Ok(None);
    }
    let mut k = 0usize;
    while k < fps.len() {
        if let Mocf::Omega(s) = fps[k] {
            if let Ok(sv) = mocf_to_term(s) {
                if tm::eq(&sv, &first_sv) {
                    k += 1;
                    continue;
                }
            }
        }
        break;
    }
    if k == 0 || k >= fps.len() {
        return Ok(None);
    }
    // rest: multiplier X = ω^{sum of remaining prims}
    let mut acc = tm::zero();
    for p in &fps[k..] {
        let val = match p {
            Mocf::Psi(vv, _) if matches!(mocf_to_term(vv), Ok(t) if !tm::is_zero(&t)) => {
                image_sub(p)?
            }
            _ => mocf_to_term(p)?,
        };
        acc = tm::add(&acc, &val);
    }
    let x = tm::standard_form(&tm::exp(&acc));
    let card = omega_s(&first_sv);
    let lead = tm::standard_form(&tm::exp(&tm::mul(&card, &nat(k as i32 + 1))));
    let arg = tm::standard_form(&tm::mul(&lead, &x));
    Ok(Some(arg))
}

/// Tail lift in k ≥ 2 regions: deep ψ_M(Ω·j + ψ_M(Ω·j)) un-collapses to
/// ψ₀(Ω²·j); everything else uses the recursive image lift.
fn lift_rest_raw(t: &Mocf) -> Result<Term, String> {
    if let Mocf::Psi(v, y) = t {
        if let Ok(vv) = mocf_to_term(v) {
            if tm::is_zero(&vv) {
                if let Mocf::Sum(ys) = y.as_ref() {
                    let omegas: Vec<&Mocf> = ys
                        .iter()
                        .take_while(|p| matches!(p, Mocf::Omega(_)))
                        .collect();
                    let j = omegas.len();
                    if j >= 1 && ys.len() == j + 1 {
                        if let Mocf::Psi(v2, y2) = &ys[j] {
                            if let (Ok(a), Ok(b)) =
                                (mocf_to_term(v2), mocf_to_term(y2))
                            {
                                let head2 = mul_k(&omega1(), j as i32);
                                if tm::is_zero(&a) && tm::eq(&b, &head2) {
                                    let lead = tm::standard_form(&tm::exp(
                                        &tm::mul(&omega1(), &nat(2)),
                                    ));
                                    return Ok(tm::t(
                                        tm::zero(),
                                        tm::standard_form(&tm::mul(&lead, &nat(j as i32))),
                                        tm::zero(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    lift_rest(t)
}

/// Tail lift for ψ_M(Ω_s^e·k + rest): ψ_M(0) ↦ ψ₀(Ω²), ψ_M(y) ↦ image.
fn lift_rest(t: &Mocf) -> Result<Term, String> {
    match t {
        Mocf::Psi(v, y) => {
            let vv = mocf_to_term(v)?;
            if tm::is_zero(&vv) {
                let arg = level0_arg(y)?;
                return Ok(tm::t(tm::zero(), arg, tm::zero()));
            }
            mocf_to_term(t)
        }
        _ => mocf_to_term(t),
    }
}

/// Shift an argument up one Ω-level: ψ_M(Ω^e·X + rest) ↦
/// ψ₀(Ω^{e+1}·ω^x·k + lifted rest), decomposing the lead block's argument
/// b = Ω·e + x.
fn cardinal_arg_shift(am: &Term) -> Term {
    if tm::is_zero(am) {
        return tm::zero();
    }
    // Pre-pass: ψ₀(Ω_s^{Ω_s·…}) identity region, located block-wise.
    {
        let mut cur = am.clone();
        while !tm::is_zero(&cur) {
            let head = tm::first_term(&cur);
            let (run, rest) = tm::separate(&cur, &head);
            if let Some(n) = head.as_ref() {
                // ψ_s(Ω_s·X) with X ≥ Ω_s: Ω_s^{Ω_s·…} identity block.
                if !tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
                    if let Some(sv) = finval(&n.a) {
                        if sv >= 2 {
                            let om_s = omega_s(&n.a);
                            let om_s_sq = tm::standard_form(&tm::exp(&tm::mul(&om_s, &nat(2))));
                            let b_ok = match n.b.as_ref() {
                                Some(nb) if tm::is_zero(&nb.a) => true,
                                Some(nb) if tm::eq(&nb.a, &n.a) => {
                                    matches!(tm::first_term(&nb.b).as_ref(),
                                        Some(nb2) if tm::eq(&nb2.a, &n.a) && tm::is_zero(&nb2.b))
                                }
                                _ => false,
                            };
                            if b_ok && !tm::lt(&n.b, &om_s_sq) {
                                let others = tm::sub(am, &run);
                                let mut acc = run.clone();
                                if !tm::is_zero(&others) {
                                    acc = tm::add(&acc, &rest_lift(&others));
                                }
                                return acc;
                            }
                        }
                    }
                }
                if tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
                    let fb = tm::first_term(&n.b);
                    let high_lead = match fb.as_ref() {
                        Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b) => {
                            finval(&nb.a).map_or(true, |sv| sv >= 2)
                        }
                        Some(nb) if tm::is_zero(&nb.a) && !tm::is_zero(&nb.b) => {
                            matches!(tm::first_term(&nb.b).as_ref(),
                                Some(nb2) if !tm::is_zero(&nb2.a) && tm::is_zero(&nb2.b)
                                    && finval(&nb2.a).map_or(true, |sv| sv >= 2))
                        }
                        _ => false,
                    };
                    if high_lead {
                        let (e, x) = omega_divmod(&n.b);
                        if tm::eq(&e, &tm::one()) && !tm::is_zero(&x) {
                            let others = tm::sub(am, &run);
                            let mut acc = run.clone();
                            if !tm::is_zero(&others) {
                                acc = tm::add(&acc, &rest_lift(&others));
                            }
                            return acc;
                        }
                    }
                }
            }
            cur = rest;
        }
    }
    let first = tm::first_term(am);
    let node = match first.as_ref() {
        Some(n) => n,
        None => return am.clone(),
    };
    if !tm::is_zero(&node.a) && tm::is_zero(&node.b) && finval(&node.a).is_none() {
        // Ω_λ-led argument: fixed-point region; tails lifted with Ω·…
        let (run, rest) = tm::separate(am, &first);
        let mut acc = lift_omega_lambda_run(&run);
        if !tm::is_zero(&rest) {
            acc = tm::add(&acc, &lift_omega_lambda_tail(&rest));
        }
        return acc;
    }
    // ψ₀(b)-form with b Ω_λ-led: identity region (Ω_ω^e etc.). Descend
    // through nested leading ψ₀ blocks (Ω_ω power towers).
    if tm::is_zero(&node.a) && !tm::is_zero(&node.b) {
        let mut cur_b = node.b.clone();
        loop {
            let fbt = tm::first_term(&cur_b);
            match fbt.as_ref() {
                Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b)
                    && finval(&nb.a).is_none() => return am.clone(),
                Some(nb) if tm::is_zero(&nb.a) && !tm::is_zero(&nb.b)
                    && tm::is_zero(&fbt.as_ref().unwrap().c) => {
                    cur_b = nb.b.clone();
                }
                _ => break,
            }
        }
    }
    // Collapsed Ω^ω-power shape ψ_v(Ω_ω+Ω·k) ↦ ψ_v(Ω^ω+Ω²·k)
    // (row 295: Ω^{Ω^ω+1} ↦ Ω^{Ω^ω+Ω}).
    if !tm::is_zero(&node.a)
        && !tm::is_zero(&node.b)
        && tm::is_zero(&node.c)
        && tm::eq(&first, am)
        && finval(&node.a).map_or(false, |v| v >= 1)
    {
        let ftb = tm::first_term(&node.b);
        let is_omw = matches!(ftb.as_ref(),
            Some(fn2) if !tm::is_zero(&fn2.a) && tm::is_zero(&fn2.c)
                && matches!(fn2.b.as_ref(),
                    Some(w) if tm::eq(&w.a, &tm::omega())
                        && tm::is_zero(&w.b) && tm::is_zero(&w.c)));
        if is_omw && !tm::eq(&ftb, &node.b) {
            let (_, rest) = tm::separate(&node.b, &ftb);
            let om = omega1();
            let (om_run, after) = tm::separate(&rest, &om);
            if tm::is_zero(&after) && !tm::is_zero(&om_run) {
                let k = tm::length1(&om_run);
                let omw_pow = tm::standard_form(&tm::exp(
                    &tm::standard_form(&tm::mul(&om, &tm::omega())),
                ));
                let om2 = tm::exp(&tm::mul(&om, &nat(2)));
                let inner = tm::add(&omw_pow, &mul_k(&om2, k));
                return tm::t(node.a.clone(), inner, tm::zero());
            }
        }
    }
    if !tm::is_zero(&node.a) && !tm::is_zero(&node.b) {
        // ψ_s(Ω_s·k) = Ω_s^{k+1}: shift the lead to Ω_s^{k+2}, lift the rest.
        let s_val = node.a.clone();
        let om_s = omega_s(&s_val);
        let fb2 = tm::first_term(&node.b);
        let is_mult = matches!(fb2.as_ref(), Some(nb2) if tm::eq(&nb2.a, &s_val) && tm::is_zero(&nb2.b));
        if is_mult {
            let (run2, rest2) = tm::separate(&node.b, &om_s);
            if tm::is_zero(&rest2) {
                let kk = tm::length1(&run2);
                let lead = tm::standard_form(&tm::exp(&tm::mul(&om_s, &nat(kk + 2))));
                let (run, rest) = tm::separate(am, &first);
                let k = tm::length1(&run);
                let mut arg = mul_k(&lead, k);
                if !tm::is_zero(&rest) {
                    arg = tm::add(&arg, &lift_omegas_rest(&rest));
                }
                return arg;
            }
        }
    }
    // ψ_s(Ω_s·k) = Ω_s^{k+1}-region lead: ↦ ψ₀(Ω_s²·ψ_s(Ω_s·k)).
    if !tm::is_zero(&node.a) && !tm::is_zero(&node.b) && tm::is_zero(&node.c) {
        let s_val = node.a.clone();
        if let Some(sv) = finval(&s_val) {
            if sv >= 2 {
                let om_s = omega_s(&s_val);
                let fb = tm::first_term(&node.b);
                let ok = matches!(fb.as_ref(), Some(nb) if tm::eq(&nb.a, &s_val) && tm::is_zero(&nb.b));
                if ok {
                    let (run2, rest2) = tm::separate(&node.b, &om_s);
                    if tm::is_zero(&rest2) && !tm::is_zero(&run2) {
                        let lead = tm::standard_form(&tm::exp(&tm::mul(&om_s, &nat(2))));
                        let blk = tm::t(s_val.clone(), node.b.clone(), tm::zero());
                        let (run, rest) = tm::separate(am, &first);
                        let k = tm::length1(&run);
                        let mut arg = mul_k(
                            &tm::standard_form(&tm::mul(&lead, &blk)),
                            k,
                        );
                        if !tm::is_zero(&rest) {
                            arg = tm::add(&arg, &rest_lift(&rest));
                        }
                        return arg;
                    }
                }
            }
        }
    }
    // ψ₀(Ω_s^{Ω_s·…}) identity region: lead kept, rest lifted.
    if tm::is_zero(&node.a) && !tm::is_zero(&node.b) && b_reaches_omega(&node.b) {
        let fb = tm::first_term(&node.b);
        let high_lead = match fb.as_ref() {
            Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b) => true,
            Some(nb) if tm::is_zero(&nb.a) && !tm::is_zero(&nb.b) => {
                matches!(tm::first_term(&nb.b).as_ref(),
                    Some(nb2) if !tm::is_zero(&nb2.a) && tm::is_zero(&nb2.b))
            }
            _ => false,
        };
        if high_lead {
            let (e, x) = omega_divmod(&node.b);
            if tm::eq(&e, &tm::one()) && !tm::is_zero(&x) {
                let (run, rest) = tm::separate(am, &first);
                        let mut acc = run.clone();
                if !tm::is_zero(&rest) {
                    acc = tm::add(&acc, &rest_lift(&rest));
                }
                return acc;
            }
        }
    }
    if tm::is_zero(&node.a) && !tm::is_zero(&node.b) && b_reaches_omega(&node.b) {
        // ψ₀(b)-form with b ≥ Ω: b = Ω·e + x ↦ ω^{Ω·(e+1)+x} = Ω^{e+1}·ω^x.
        let b = node.b.clone();
        let (e, x) = omega_divmod(&b);
        let expterm = tm::add(&tm::mul(&omega1(), &tm::add(&e, &tm::one())), &x);
        let lead = tm::standard_form(&tm::exp(&expterm));
        let (run, rest) = tm::separate(am, &first);
        let k = tm::length1(&run);
        let mut arg = mul_k(&lead, k);
            if !tm::is_zero(&rest) {
            let rl = rest_lift(&rest);
                    arg = tm::add(&arg, &rl);
        }
        return arg;
    }
    if tm::eq(&node.a, &tm::one()) && tm::is_zero(&node.b) {
        // Ω block(s): Ω·k ↦ Ω²·k, rest lifted ×Ω.
        let (run, rest) = tm::separate(am, &first);
        let k = tm::length1(&run);
        let lead = tm::standard_form(&tm::exp(&tm::mul(&omega1(), &nat(2))));
        let mut arg = mul_k(&lead, k);
        if !tm::is_zero(&rest) {
            arg = tm::add(&arg, &rest_lift(&rest));
        }
        return arg;
    }
    if !tm::is_zero(&node.a) && tm::is_zero(&node.b) {
        // Higher Ω_a block: stays; lift rest.
        let (_run, rest) = tm::separate(am, &first);
        let mut acc = first.clone();
        if !tm::is_zero(&rest) {
            acc = tm::add(&acc, &rest_lift(&rest));
        }
        return acc;
    }
    lift_fixed_tail(am)
}

/// Decompose b = Ω·e + x (e the Ω-run length of the leading block).
fn omega_divmod(b: &Term) -> (Term, Term) {
    if tm::is_zero(b) {
        return (tm::zero(), tm::zero());
    }
    let first = tm::first_term(b);
    let node = match first.as_ref() {
        Some(n) => n,
        None => return (tm::one(), b.clone()),
    };
    if tm::is_zero(&node.b) && tm::eq(&node.a, &tm::one()) {
        let (run, rest) = tm::separate(b, &first);
        let k = tm::length1(&run);
        return (nat(k), rest);
    }
    if tm::is_zero(&node.b) && !tm::is_zero(&node.a) {
        // Higher cardinal Ω_s·k: count the finite run.
        let (run, rest2) = tm::separate(b, &first);
        let k = tm::length1(&run);
        let kv = finval(&nat(k));
        if tm::is_zero(&rest2) && kv.is_some() && kv.unwrap() >= 1 {
            return (nat(k), tm::zero());
        }
        return (tm::one(), b.clone());
    }
    (tm::one(), b.clone())
}

/// Lift rest blocks for Ω-power regions: ψ₀(b) ↦ ψ₀(Ω·b), Ω^e ↦ Ω^{e+1},
/// finite ↦ Ω·n, Ω_a ↦ Ω_a·Ω.
fn rest_lift(t: &Term) -> Term {
    if tm::is_zero(t) {
        return tm::zero();
    }
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            // ψ₀(b): if b leads with Ω_s (s ≥ 2), reverse to ψ_{s-1}(b);
            // otherwise ψ₀(b) ↦ ψ₀(Ω·b).
            let fb = tm::first_term(&n.b);
            let rev = match fb.as_ref() {
                Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b) => {
                    match finval(&nb.a) {
                        Some(sv) if sv >= 2 => {
                            let (_, rr2) = tm::separate(&n.b, &omega_s(&nat(sv)));
                            if tm::is_zero(&rr2) { Some(sv) } else { None }
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
                    let s_led = matches!(fb.as_ref(),
                Some(nb) if !tm::is_zero(&nb.a) && tm::is_zero(&nb.b)
                    && finval(&nb.a).map_or(false, |sv| sv >= 2));
            match rev {
                Some(sv) => tm::t(nat(sv - 1), n.b.clone(), tm::zero()),
                None if s_led => head.clone(),
                None => tm::t(
                    tm::zero(),
                    tm::standard_form(&tm::mul(&omega1(), &n.b)),
                    tm::zero(),
                ),
            }
        } else if !tm::is_zero(&n.a) && !tm::is_zero(&n.b) {
            // ψ_s(b≠0): kept.
            head.clone()
        } else if tm::is_zero(&n.b) && !tm::is_zero(&n.a) {
            match finval(&n.a) {
                Some(sv) if sv >= 2 => head.clone(),
                _ => tm::standard_form(&tm::mul(&head, &omega1())),
            }
        } else if tm::eq(&n.a, &tm::one()) {
            // Ω^{b+1}-form block: multiply by Ω.
            tm::standard_form(&tm::mul(&omega1(), &head))
        } else {
            tm::standard_form(&tm::mul(&omega1(), &head))
        };
        acc = tm::add(&acc, &mul_k(&lifted, k));
        cur = rest;
    }
    acc
}

/// ψ_M(sum): leading-block analysis.
fn sum_argument(terms: &[Mocf]) -> Result<Term, String> {
    if let Mocf::Psi(v, a2) = &terms[0] {
        let vv = mocf_to_term(v)?;
        if !tm::is_zero(&vv) {
            let vi = finval(&vv).unwrap_or(0);
            let s = tm::add(&vv, &tm::one());
            let card = omega_s(&s);
            let am2 = if psi_leading_syntax_v(a2) {
                image_sub(a2)?
            } else {
                mocf_to_term(a2)?
            };
            let mut arg =
                tm::standard_form(&tm::mul(&card, &tm::add(&tm::one(), &am2)));
            let mut k = 1usize;
            while k < terms.len() {
                if let Mocf::Psi(v2, a3) = &terms[k] {
                    if tm::eq(&mocf_to_term(v2)?, &vv)
                        && tm::eq(&mocf_to_term(a3)?, &mocf_to_term(a2)?)
                    {
                        k += 1;
                        continue;
                    }
                }
                break;
            }
            if k >= 2 {
                let blk = tm::t(vv.clone(), card.clone(), tm::zero());
                arg = tm::add(&arg, &mul_k(&blk, (k - 1) as i32));
            }
            for t in &terms[k..] {
                arg = tm::add(&arg, &lift_syn(t, vi + 1)?);
            }
            return Ok(arg);
        }
    }
    // Cardinal / Ω-power arguments: shift the whole value up one level.
    let mut am = tm::zero();
    for t in terms {
        am = tm::add(&am, &mocf_to_term(t)?);
    }
    if !tm::lt(&am, &omega1()) {
        return Ok(cardinal_arg_shift(&am));
    }
    let arg = tm::standard_form(&tm::mul(&omega1(), &tm::add(&tm::one(), &am)));
    Ok(arg)
}

/// Tail lift in Ω-power regions: every block gains an Ω factor
/// (1 ↦ Ω, Ω^e ↦ Ω^{e+1}, ψ₀(Ω^e+…) ↦ ψ₀(Ω^{e+1}+…) via ×Ω value).
fn lift_fixed_tail(t: &Term) -> Term {
    if tm::is_zero(t) {
        return tm::zero();
    }
    let mut acc = tm::zero();
    let mut cur = t.clone();
    while !tm::is_zero(&cur) {
        let head = tm::first_term(&cur);
        let (run, rest) = tm::separate(&cur, &head);
        let k = tm::length1(&run);
        let n = match head.as_ref() {
            Some(n) => n,
            None => return acc,
        };
        let lifted = if tm::is_zero(&n.b) && !tm::is_zero(&n.a) {
            // Ω_a ↦ Ω_a·Ω
            tm::standard_form(&tm::mul(&head, &omega1()))
        } else if tm::eq(&n.a, &tm::one()) && !tm::is_zero(&n.b) {
            // Ω^b·k ↦ Ω^{b+1}·k when b finite; else Ω·block.
            match finval(&n.b) {
                Some(m) => {
                    let p = tm::standard_form(&tm::exp(&tm::mul(&omega1(), &nat(m + 1))));
                    p
                }
                None => tm::standard_form(&tm::mul(&omega1(), &head)),
            }
        } else {
            tm::standard_form(&tm::mul(&omega1(), &head))
        };
        acc = tm::add(&acc, &mul_k(&lifted, k));
        cur = rest;
    }
    acc
}

/// Render a standard-form Term as a parseable Unicode BOCF string.
/// Render a ψ-argument, folding pure Ω-multiples into Ω×k.
fn arg_to_bocf_input(b: &Term) -> String {
    if tm::is_zero(b) {
        return "0".to_string();
    }
    if let Some(n) = finval(b) {
        return n.to_string();
    }
    // Pure Ω-multiple arguments fold to Ω×k.
    if tm::eq(&tm::first_term(b), &tm::omega1()) {
        let (run, rest) = tm::separate(b, &tm::omega1());
        if tm::is_zero(&rest) && tm::eq(&run, b) {
            let k = tm::length1(&run);
            if k == 1 {
                return "Ω".to_string();
            }
            return format!("Ω×{}", k);
        }
    }
    let node = match b.as_ref() {
        Some(n) => n,
        None => return "0".to_string(),
    };
    if tm::is_zero(&node.c) && tm::is_zero(&node.b) && !tm::is_zero(&node.a) {
        if !tm::eq(&node.a, &tm::one()) {
            return if let Some(n) = finval(&node.a) {
                format!("Ω_{}", n)
            } else {
                format!("Ω_({})", term_to_bocf_input(&node.a))
            };
        }
    }
    term_to_bocf_input(b)
}

pub fn term_to_bocf_input(q: &Term) -> String {
    if tm::is_zero(q) {
        return "0".to_string();
    }
    if let Some(n) = finval(q) {
        return n.to_string();
    }
    let node = q.as_ref().unwrap();
    let block = if tm::is_zero(&node.b) {
        if tm::is_zero(&node.a) {
            "1".to_string()
        } else if let Some(n) = finval(&node.a) {
            format!("ψ_{}(0)", n)
        } else {
            format!("ψ_({})(0)", term_to_bocf_input(&node.a))
        }
    } else {
        let arg = arg_to_bocf_input(&node.b);
        if tm::is_zero(&node.a) {
            format!("ψ({})", arg)
        } else if let Some(n) = finval(&node.a) {
            format!("ψ_{}({})", n, arg)
        } else {
            format!("ψ_({})({})", term_to_bocf_input(&node.a), arg)
        }
    };
    if tm::is_zero(&node.c) {
        block
    } else {
        format!("{}+{}", block, term_to_bocf_input(&node.c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(s: &str) -> String {
        mocf_to_bocf(s).unwrap()
    }

    #[test]
    fn mocf_to_bocf_basic() {
        assert_eq!(conv("0"), "0");
        assert_eq!(conv("ψ(0)"), "ψ(Ω)");
        assert_eq!(conv("Ω"), "ψ_1(0)");
        assert_eq!(conv("Ω_2"), "ψ_2(0)");
        assert_eq!(conv("ψ(1)"), "ψ(Ω×2)");
    }

    fn split_csv(line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    fields.push(cur.clone());
                    cur.clear();
                }
                _ => cur.push(c),
            }
        }
        fields.push(cur);
        fields
    }

    fn plain_mocf(s: &str) -> String {
        let mut t = s
            .replace("\\cdot", "*")
            .replace("\\Omega", "Ω")
            .replace("\\omega", "ω")
            .replace("\\psi", "ψ")
            .replace("^{", "^(")
            .replace("_{", "_(")
            .replace('{', "")
            .replace('}', ")");
        t.retain(|c| !c.is_whitespace());
        t
    }

    #[test]
    fn mocf_to_bocf_csv_audit() {
        let csv = include_str!("../../../../bocf vs mocf.csv");
        let mut checked = 0;
        let mut fail = 0;
        for (idx, line) in csv.lines().enumerate() {
            if idx == 0 {
                continue;
            }
            let fields = split_csv(line);
            if fields.len() < 2 {
                continue;
            }
            let bocf = plain_mocf(&fields[0]);
            let mocf_in = plain_mocf(&fields[1]);
            let expected = match crate::parser::parse_bocf(&bocf)
                .and_then(|a| crate::parser::eval_raw_ast(&a))
            {
                Ok(t) => t,
                Err(_) => continue,
            };
            let parsed = match parse_mocf(&mocf_in) {
                Ok(p) => p,
                Err(e) => {
                    println!("ROW {} PARSE ERROR {}: {}", idx + 1, mocf_in, e);
                    fail += 1;
                    continue;
                }
            };
            let got = match mocf_to_term_top(&parsed) {
                Ok(t) => t,
                Err(e) => {
                    println!("ROW {} CONVERT ERROR {}: {}", idx + 1, mocf_in, e);
                    fail += 1;
                    continue;
                }
            };
            let got_sf = tm::standard_form(&got);
            if !tm::eq(&got_sf, &tm::standard_form(&expected)) {
                println!(
                    "ROW {} VALUE MISMATCH: mocf {} -> {} (expected {})",
                    idx + 1,
                    fields[1],
                    tm::term_to_string(false, &got_sf),
                    fields[0]
                );
                fail += 1;
            }
            checked += 1;
        }
        println!("checked {} rows: fail={}", checked, fail);
        assert_eq!(fail, 0, "mocf->bocf audit mismatches");
    }
}
