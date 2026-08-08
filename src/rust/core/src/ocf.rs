//! Nothing OCF (NOCF) and Madore's OCF (MOCF) implementations.
//!
//! NOCF: ψ_v(a) — Nothing OCF up to EBO.
//! MOCF: ω^, Ω, ψ — Madore's OCF up to EBO.

use std::fmt;

// ════════════════════════════════════════════════════════════════
// Nothing OCF (NOCF)
// ════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nocf {
    Zero,
    Psi(Box<Nocf>, Box<Nocf>),
}

impl Nocf {
    pub fn zero() -> Nocf { Nocf::Zero }
    pub fn psi(v: Nocf, a: Nocf) -> Nocf { Nocf::Psi(Box::new(v), Box::new(a)) }

    /// Build a natural number n as ψ_0(ψ_0(...(0)...)) (n nested ψ_0's).
    /// For n=0 returns Zero; for n=1 returns ψ_0(0); etc.
    pub fn from_nat(n: i32) -> Nocf {
        let mut r = Nocf::zero();
        for _ in 0..n { r = Nocf::psi(Nocf::zero(), r); }
        r
    }
    fn is_zero(&self) -> bool { matches!(self, Nocf::Zero) }
    pub fn to_nat(&self) -> i32 {
        match self {
            Nocf::Zero => 0,
            Nocf::Psi(v, a) => {
                if !v.is_zero() { return -1; }
                let n = a.to_nat();
                if n < 0 { -1 } else { n + 1 }
            }
        }
    }

    fn compare(&self, other: &Nocf) -> i32 {
        if self.is_zero() && other.is_zero() { return 0; }
        if self.is_zero() { return -1; }
        if other.is_zero() { return 1; }
        match (self, other) {
            (Nocf::Psi(v1, a1), Nocf::Psi(v2, a2)) => {
                let c = v1.compare(v2);
                if c != 0 { return c; }
                a1.compare(a2)
            }
            _ => unreachable!(),
        }
    }

    fn cofinality(&self) -> Option<Nocf> {
        match self {
            Nocf::Zero => None,
            Nocf::Psi(v, a) => {
                if a.is_zero() {
                    if v.is_zero() { return None; }
                    v.cofinality().or_else(|| Some(v.as_ref().clone()))
                } else {
                    a.cofinality().and_then(|cf_a| {
                        if cf_a.compare(v) <= 0 { Some(cf_a) } else { Some(Nocf::zero()) }
                    })
                }
            }
        }
    }

    /// Fundamental sequence with arbitrary expression index.
    fn fs(&self, index: &Nocf) -> Nocf {
        match self {
            Nocf::Zero => Nocf::zero(),
            Nocf::Psi(v, a) => {
                if a.is_zero() {
                    if v.is_zero() { return Nocf::zero(); }
                    match v.cofinality() {
                        None => index.clone(),
                        Some(_) => Nocf::psi(v.fs(index), Nocf::zero()),
                    }
                } else {
                    match a.cofinality() {
                        None => Nocf::psi(v.as_ref().clone(), a.fs(&Nocf::zero())),
                        Some(cf_a) => {
                            if cf_a.compare(v) <= 0 {
                                Nocf::psi(v.as_ref().clone(), a.fs(index))
                            } else {
                                let cf_a_pred = cf_a.fs(&Nocf::zero());
                                let i_nat = if index.is_zero() { 0 } else { index.to_nat() };
                                let mut result = Nocf::zero();
                                for _ in 0..i_nat.max(0) {
                                    result = a.fs(&Nocf::psi(cf_a_pred.clone(), result));
                                }
                                Nocf::psi(v.as_ref().clone(), result)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Expand by natural number index.
    pub fn expand(&self, index: i32) -> Nocf {
        self.fs(&Nocf::from_nat(index))
    }
}

impl fmt::Display for Nocf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nocf::Zero => write!(f, "0"),
            Nocf::Psi(v, a) => {
                if v.is_zero() { write!(f, "\\psi({})", a) }
                else {
                    // Display natural number subscripts as plain numbers
                    let n = v.to_nat();
                    if n >= 0 { write!(f, "\\psi_{{{}}}({})", n, a) }
                    else { write!(f, "\\psi_{{{}}}({})", v, a) }
                }
            }
        }
    }
}

/// Sugar display for NOCF:
/// - natural numbers: ψ_0^n(0) → n
/// - ψ_0(Ω) → ω
/// - ψ_0(α) → ψ(α)
/// - ψ_α(0) → Ω_α  (α = 1 → Ω)
/// - ψ_α(β) → Ω_α + β
pub fn nocf_to_sugar_string(n: &Nocf) -> String {
    let nat = n.to_nat();
    if nat >= 0 { return nat.to_string(); }
    match n {
        Nocf::Zero => "0".to_string(),
        Nocf::Psi(v, a) => {
            match (v.is_zero(), a.is_zero()) {
                // ψ_0(0) = 1
                (true, true) => "1".to_string(),
                // ψ_0(Ω) = ω
                (true, false) if is_nocf_omega(a) => "\\omega".to_string(),
                // ψ_0(α) = ψ(α)
                (true, false) => format!("\\psi({})", nocf_to_sugar_string(a)),
                // ψ_α(0) = Ω_α
                (false, true) => {
                    let sub = nocf_to_sugar_string(v);
                    if sub == "1" { "\\Omega".to_string() } else { format!("\\Omega_{{{}}}", sub) }
                }
                // ψ_α(β) = Ω_α + β
                (false, false) => {
                    let sub = nocf_to_sugar_string(v);
                    let omega = if sub == "1" { "\\Omega".to_string() } else { format!("\\Omega_{{{}}}", sub) };
                    format!("{}+{}", omega, nocf_to_sugar_string(a))
                }
            }
        }
    }
}

/// True if `n` is Ω = ψ_1(0).
fn is_nocf_omega(n: &Nocf) -> bool {
    matches!(n, Nocf::Psi(v, a) if v.to_nat() == 1 && a.is_zero())
}

pub fn parse_nocf(input: &str) -> Result<Nocf, String> {
    let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s == "0" { return Ok(Nocf::zero()); }
    if s.chars().all(|c| c.is_ascii_digit()) {
        let n: i32 = s.parse().map_err(|_| "invalid number".to_string())?;
        return Ok(Nocf::from_nat(n));
    }
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    let result = parse_nocf_expr(&chars, &mut pos)?;
    if pos != chars.len() { return Err(format!("trailing: {}", chars[pos..].iter().collect::<String>())); }
    Ok(result)
}

fn parse_nocf_expr(chars: &[char], pos: &mut usize) -> Result<Nocf, String> {
    if *pos >= chars.len() { return Err("unexpected end".to_string()); }
    let c = chars[*pos];
    if c == '0' { *pos += 1; return Ok(Nocf::zero()); }
    if c.is_ascii_digit() && c != '0' {
        let mut n = 0i32;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            n = n * 10 + (chars[*pos] as i32 - '0' as i32);
            *pos += 1;
        }
        return Ok(Nocf::from_nat(n));
    }
    if c == 'ψ' {
        *pos += 1;
        let mut v = Nocf::zero();
        if *pos < chars.len() && chars[*pos] == '_' {
            *pos += 1;
            if *pos < chars.len() && chars[*pos] == '(' {
                *pos += 1; v = parse_nocf_expr(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
                *pos += 1;
            } else { v = parse_nocf_expr(chars, pos)?; }
        }
        if *pos >= chars.len() || chars[*pos] != '(' { return Err("expected '('".to_string()); }
        *pos += 1; let a = parse_nocf_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
        *pos += 1;
        return Ok(Nocf::psi(v, a));
    }
    if c == '(' { *pos += 1; let e = parse_nocf_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
        *pos += 1; return Ok(e); }
    // Ω = ψ_1(0), with optional subscript
    if c == 'Ω' || c == 'W' {
        *pos += 1;
        if *pos < chars.len() && chars[*pos] == '_' {
            *pos += 1;
            if *pos < chars.len() && chars[*pos] == '(' {
                *pos += 1; let sub = parse_nocf_expr(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
                *pos += 1;
                return Ok(Nocf::psi(sub, Nocf::zero()));
            } else { let sub = parse_nocf_expr(chars, pos)?; return Ok(Nocf::psi(sub, Nocf::zero())); }
        }
        return Ok(Nocf::psi(Nocf::from_nat(1), Nocf::zero()));
    }
    Err(format!("unexpected '{}'", c))
}

pub fn analyze_nocf(input: &str) -> Result<String, String> {
    Ok(parse_nocf(input)?.to_string())
}

/// Analyze a NOCF expression with sugar: ψ_0^n(0) → n.
pub fn analyze_nocf_sugar(input: &str) -> Result<String, String> {
    let expr = parse_nocf(input)?;
    Ok(nocf_to_sugar_string(&expr))
}

pub fn expand_nocf(input: &str, fs: i32) -> Result<String, String> {
    let expr = parse_nocf(input)?;
    Ok(expr.expand(fs).to_string())
}

// ════════════════════════════════════════════════════════════════
// Madore's OCF (MOCF)
// ════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mocf {
    Zero,
    Sum(Vec<Mocf>),
    OmegaPow(Box<Mocf>),
    Omega(Box<Mocf>),
    Psi(Box<Mocf>, Box<Mocf>),
    /// Normalized power q^e of an ω^-fixed point q (display form only).
    Pow(Box<Mocf>, Box<Mocf>),
}

impl Mocf {
    fn zero() -> Mocf { Mocf::Zero }
    fn one() -> Mocf { Mocf::omega_pow(Mocf::zero()) }
    fn omega_pow(a: Mocf) -> Mocf { Mocf::OmegaPow(Box::new(a)) }
    fn omega(a: Mocf) -> Mocf { Mocf::Omega(Box::new(a)) }
    fn psi(v: Mocf, a: Mocf) -> Mocf { Mocf::Psi(Box::new(v), Box::new(a)) }
    fn is_zero(&self) -> bool { matches!(self, Mocf::Zero) }

    fn prim_list(&self) -> Vec<&Mocf> {
        if self.is_zero() { return vec![]; }
        match self { Mocf::Sum(terms) => terms.iter().collect(), _ => vec![self] }
    }
    fn from_prim_list(ps: Vec<Mocf>) -> Mocf {
        let mut flat: Vec<Mocf> = Vec::new();
        for p in ps {
            match p {
                Mocf::Sum(terms) => flat.extend(terms),
                other => flat.push(other),
            }
        }
        if flat.is_empty() { return Mocf::zero(); }
        if flat.len() == 1 { return flat.into_iter().next().unwrap(); }
        Mocf::Sum(flat)
    }
    fn add(&self, other: &Mocf) -> Mocf {
        let mut ps: Vec<Mocf> = Vec::new();
        for p in self.prim_list() { ps.push(p.clone()); }
        let qs = other.prim_list();
        // Left absorption: blocks are additively principal, so a smaller
        // prefix vanishes before a larger block (α + ω^β = ω^β for α < ω^β).
        if let (Some(last), Some(first)) = (ps.last(), qs.first()) {
            if last.compare(first) < 0 { ps.clear(); }
        }
        for p in qs { ps.push(p.clone()); }
        Mocf::from_prim_list(ps)
    }
    fn mul_nat(&self, n: i32) -> Mocf {
        let mut ps: Vec<Mocf> = Vec::new();
        for _ in 0..n {
            for p in self.prim_list() { ps.push(p.clone()); }
        }
        Mocf::from_prim_list(ps)
    }
    fn from_nat(n: i32) -> Mocf { Mocf::one().mul_nat(n) }
    fn to_nat(&self) -> i32 {
        let ps = self.prim_list();
        if ps.is_empty() { return 0; }
        if ps[0].compare(&Mocf::one()) != 0 { return -1; }
        ps.len() as i32
    }
    fn to_nat_pos(&self) -> Option<i32> {
        let n = self.to_nat();
        if n >= 1 { Some(n) } else { None }
    }

    fn compare(&self, other: &Mocf) -> i32 {
        fn cmp(a: &Mocf, b: &Mocf) -> i32 {
            if a.is_zero() || b.is_zero() || matches!(a, Mocf::Sum(_)) || matches!(b, Mocf::Sum(_)) {
                let pa = a.prim_list(); let pb = b.prim_list();
                let n = pa.len().min(pb.len());
                for i in 0..n { let c = cmp(pa[i], pb[i]); if c != 0 { return c; } }
                return (pa.len() as i32 - pb.len() as i32).signum();
            }
            match (a, b) {
                (Mocf::OmegaPow(a1), Mocf::OmegaPow(b1)) => cmp(a1, b1),
                (Mocf::OmegaPow(a1), _) => cmp(a1, b),
                (_, Mocf::OmegaPow(b1)) => cmp(a, b1),
                (Mocf::Omega(a1), Mocf::Omega(b1)) => cmp(a1, b1),
                (Mocf::Omega(a1), Mocf::Psi(v, a2)) => {
                    let c = cmp(a1, v);
                    if c != 0 { return c; }
                    // Ω_v = ψ_v(0); Ω_v < ψ_v(x) for x > 0.
                    if a2.is_zero() { 0 } else { -1 }
                }
                (Mocf::Psi(v, a1), Mocf::Omega(b1)) => {
                    let c = cmp(v, b1);
                    if c != 0 { return c; }
                    if a1.is_zero() { 0 } else { 1 }
                }
                (Mocf::Psi(v1, a1), Mocf::Psi(v2, a2)) => {
                    let c = cmp(v1, v2); if c != 0 { return c; } cmp(a1, a2)
                }
                (Mocf::Pow(b1, e1), Mocf::Pow(b2, e2)) => cmp(
                    &Mocf::omega_pow(omega_mul(b1, e1)),
                    &Mocf::omega_pow(omega_mul(b2, e2)),
                ),
                (Mocf::Pow(bx, ex), ob) => cmp(&Mocf::omega_pow(omega_mul(bx, ex)), ob),
                (ob, Mocf::Pow(bx, ex)) => cmp(ob, &Mocf::omega_pow(omega_mul(bx, ex))),
                _ => 0,
            }
        }
        cmp(self, other)
    }

    fn cofinality(&self) -> Option<Mocf> {
        match self {
            Mocf::Zero => None,
            Mocf::Sum(terms) => terms.last().and_then(|t| t.cofinality()),
            Mocf::OmegaPow(a) => { if a.is_zero() { None } else { Some(a.cofinality().unwrap_or_else(|| Mocf::zero())) } }
            Mocf::Omega(a) => a.cofinality().or_else(|| Some(a.as_ref().clone())),
            Mocf::Psi(v, a) => {
                if a.is_zero() { return Some(Mocf::zero()); }
                a.cofinality().map(|cf_a| if cf_a.compare(v) <= 0 { cf_a } else { Mocf::zero() })
                    .or(Some(Mocf::zero()))
            }
            Mocf::Pow(_, e) => e.cofinality(),
        }
    }

    /// Fundamental sequence with arbitrary expression index.
    fn fs(&self, index: &Mocf) -> Mocf {
        match self {
            Mocf::Zero => Mocf::zero(),
            Mocf::Sum(terms) => {
                if terms.is_empty() { return Mocf::zero(); }
                let tail = terms.last().unwrap().fs(index);
                let mut rest: Vec<Mocf> = terms[..terms.len() - 1].to_vec();
                for p in tail.prim_list() { rest.push(p.clone()); }
                Mocf::from_prim_list(rest)
            }
            Mocf::OmegaPow(a) => {
                if a.is_zero() { return Mocf::zero(); }
                match a.cofinality() {
                    None => { let wp = Mocf::omega_pow(a.fs(&Mocf::zero())); wp.mul_nat(index.to_nat().max(0)) }
                    Some(_) => Mocf::omega_pow(a.fs(index)),
                }
            }
            Mocf::Pow(b, e) => {
                Mocf::omega_pow(fixed_mul(b, e)).fs(index)
            }
            Mocf::Omega(a) => {
                match a.cofinality() {
                    None => Mocf::from_nat(index.to_nat().max(0)),
                    Some(_) => { let e = a.fs(index); if e.is_zero() { Mocf::zero() } else { Mocf::omega(e) } }
                }
            }
            Mocf::Psi(v, a) => {
                match a.cofinality() {
                    None => {
                        let i_nat = index.to_nat().max(0);
                        let base = if a.is_zero() {
                            if v.is_zero() { Mocf::zero() } else { Mocf::omega(v.as_ref().clone()) }
                        } else { Mocf::psi(v.as_ref().clone(), a.fs(&Mocf::zero())) };
                        if i_nat == 0 { return base; }
                        let mut result = base.add(&Mocf::one());
                        for _ in 0..i_nat - 1 { result = Mocf::omega_pow(result); }
                        result
                    }
                    Some(cf_a) => {
                        if cf_a.compare(v) <= 0 { Mocf::psi(v.as_ref().clone(), a.fs(index)) }
                        else {
                            let cf_a_pred = cf_a.fs(&Mocf::zero());
                            let i_nat = index.to_nat().max(0);
                            let mut result = Mocf::zero();
                            for _ in 0..i_nat {
                                result = a.fs(&Mocf::psi(cf_a_pred.clone(), result));
                            }
                            Mocf::psi(v.as_ref().clone(), result)
                        }
                    }
                }
            }
        }
    }

    pub fn expand(&self, index: i32) -> Mocf {
        self.fs(&Mocf::from_nat(index))
    }
}

/// True if the block is a fixed point of ω^: any ψ-value (ε-numbers, and
/// ψ_v(0) = Ω_v), Ω_v itself, or ω^ of such a block (ψ = ω^ψ).
fn is_psi_value(p: &Mocf) -> bool {
    match p {
        Mocf::Psi(..) => true,
        Mocf::Omega(_) => true,
        Mocf::OmegaPow(e) => is_psi_value(e),
        Mocf::Pow(q, _) => is_psi_value(q),
        _ => false,
    }
}

/// Render a power tower base^{e1^{e2^{...}}} with nested braces.
fn tower_format(q: &Mocf, exp: &Mocf) -> String {
    match exp {
        Mocf::Pow(q2, y) => format!("{}^{{{}}}", q, tower_format(q2, y)),
        _ => format!("{}^{{{}}}", q, exp),
    }
}

/// Decompose `exp` as q·Y when every prim of `exp` is ω^{q·k+r} sharing
/// a common ω^-fixed-point block q: the prim contributes the Y-part
/// q^{k-1}·ω^r. Returns (q, Y).
fn extract_qy(exp: &Mocf) -> Option<(Mocf, Mocf)> {
    let prims = exp.prim_list();
    if prims.is_empty() { return None; }
    let mut common: Option<Mocf> = None;
    let mut y_parts: Vec<Mocf> = Vec::new();
    for p in &prims {
        match p {
            Mocf::OmegaPow(e) => {
                let gs = display_groups(e);
                if gs.is_empty() || !is_psi_value(&gs[0].0) { return None; }
                let pb = gs[0].0.clone();
                match &common {
                    None => common = Some(pb.clone()),
                    Some(c0) => {
                        if c0.compare(&pb) != 0 { return None; }
                    }
                }
                let mut rest: Vec<Mocf> = Vec::new();
                for _ in 0..gs[0].1 - 1 { rest.push(pb.clone()); }
                for (b, k) in &gs[1..] {
                    for _ in 0..*k { rest.push(b.clone()); }
                }
                let y = if rest.is_empty() {
                    Mocf::one()
                } else {
                    Mocf::omega_pow(Mocf::from_prim_list(rest))
                };
                y_parts.push(y);
            }
            Mocf::Omega(_) | Mocf::Psi(..) => {
                // A bare fixed point is ω^q: contributes one q-factor.
                let pb = (*p).clone();
                match &common {
                    None => common = Some(pb.clone()),
                    Some(c0) => {
                        if c0.compare(&pb) != 0 { return None; }
                    }
                }
                y_parts.push(Mocf::one());
            }
            _ => return None,
        }
    }
    Some((common?, Mocf::from_prim_list(y_parts)))
}

/// Display grouping of a sum of prim blocks: left-absorption (A + B = B
/// when A < B, since blocks are additively principal) and like-term
/// merging into (block, count) groups.
fn display_groups(x: &Mocf) -> Vec<(Mocf, i32)> {
    let mut groups: Vec<(Mocf, i32)> = Vec::new();
    for p in x.prim_list() {
        while let Some((last, _)) = groups.last() {
            if last.compare(p) < 0 { groups.pop(); } else { break; }
        }
        if let Some((last, k)) = groups.last_mut() {
            if last.compare(p) == 0 { *k += 1; continue; }
        }
        groups.push((p.clone(), 1));
    }
    groups
}

/// q·x for an ω^-fixed point q (ψ = ω^ψ, Ω_a = ω^{Ω_a}): distribute over
/// the prim blocks of x, ω^q · ω^e = ω^{q+e}; Ω and ψ blocks are
/// ω^-fixed points too, so ω^q · r = ω^{q+r}.
fn fixed_mul(q: &Mocf, x: &Mocf) -> Mocf {
    let mut out: Vec<Mocf> = Vec::new();
    for p in x.prim_list() {
        let e = match p {
            Mocf::Zero => continue,
            Mocf::OmegaPow(e) => q.add(e),
            other => q.add(other),
        };
        out.push(Mocf::omega_pow(e));
    }
    Mocf::from_prim_list(out)
}

/// base^exp for ω^-fixed points, processed at parse time as
/// ω^(base×exp): ψ_a(b)^c = ω^{ψ_a(b)·c}.
/// Ω_a^c keeps the Ω-power tower in display form (Pow), unless the
/// exponent is a ≥Ω ψ-value that absorbs the base entirely
/// (Ω^ψ_1(1) = ψ_1(1)).
fn mocf_pow(base: Mocf, exp: Mocf) -> Result<Mocf, String> {
    match &base {
        Mocf::Psi(..) => Ok(Mocf::omega_pow(mul_mocf(&base, &exp)?)),
        Mocf::Omega(_) => {
            if is_omega_pow_exp(&exp) || exp.compare(&Mocf::omega(Mocf::one())) < 0 {
                Ok(Mocf::Pow(Box::new(base), Box::new(exp)))
            } else {
                Ok(Mocf::omega_pow(mul_mocf(&base, &exp)?))
            }
        }
        Mocf::OmegaPow(x) => Ok(Mocf::omega_pow(mul_mocf(x, &exp)?)),
        _ => Err("^ is only supported for \\psi and \\Omega bases".to_string()),
    }
}

/// True if e keeps the Ω^e power-tower display form (bare Ω or Ω^…).
fn is_omega_pow_exp(e: &Mocf) -> bool {
    match e {
        Mocf::Omega(_) => true,
        Mocf::Pow(b, _) => matches!(b.as_ref(), Mocf::Omega(_)),
        _ => false,
    }
}

/// The ω^-exponent of a whole block: ω^e ↦ e; Ω_a and ψ_a(b) are fixed
/// points of ω^ (in Madore's OCF ψ(0) = ε₀), so their exponent is
/// themselves.
fn block_exponent(p: &Mocf) -> Result<Mocf, String> {
    match p {
        Mocf::Zero => Err("zero factor".to_string()),
        Mocf::OmegaPow(e) => Ok(e.as_ref().clone()),
        Mocf::Omega(_) => Ok(p.clone()),
        Mocf::Psi(..) => Ok(p.clone()),
        Mocf::Pow(q, y) => match q.as_ref() {
            Mocf::Psi(..) | Mocf::Omega(_) => Ok(omega_mul(q, y)),
            _ => Err("unsupported power factor in multiplication".to_string()),
        },
        Mocf::Sum(_) => Err("sum is not a whole block".to_string()),
    }
}

/// q·y for a fixed point q (q = ψ or Ω): ψ·y = Σ ω^{ψ + block(y)};
/// Ω·y = the ω^-exponent of Ω^y, i.e. Ω·y itself as a value.
fn omega_mul(q: &Mocf, y: &Mocf) -> Mocf {
    if matches!(q, Mocf::Omega(_)) {
        if let Mocf::Pow(b, e) = y {
            if matches!(b.as_ref(), Mocf::Omega(_)) {
                // Ω·Ω^e = Ω^{e+1}.
                if let Some(k) = e.to_nat_pos() {
                    return Mocf::from_prim_list(
                        std::iter::repeat(q.clone()).take(k as usize + 1).collect(),
                    );
                }
                // Ω·Ω^e = Ω^e for infinite e (1+e = e).
                return Mocf::omega_pow(omega_mul(q, e));
            }
        }
        if let Some(k) = y.to_nat_pos() {
            return Mocf::from_prim_list(
                std::iter::repeat(q.clone()).take(k as usize).collect(),
            );
        }
    }
    fixed_mul(q, y)
}

/// Ordinal multiplication of whole blocks (ω^e, Ω_a, ψ_a(b)) and their
/// sums. Uses ω^α·ω^β = ω^{α+β}: a·b depends only on the ω^-degree of
/// a and the block exponents of b (left-distributive over the right
/// operand); a trailing natural coefficient is iterated addition.
fn mul_mocf(a: &Mocf, b: &Mocf) -> Result<Mocf, String> {
    if a.is_zero() || b.is_zero() { return Ok(Mocf::zero()); }
    if let Some(n) = b.to_nat_pos() { return Ok(a.mul_nat(n)); }
    let deg = block_exponent(a.prim_list()[0])?;
    let mut result = Mocf::zero();
    let mut nat_tail = 0;
    for p in b.prim_list() {
        if p.compare(&Mocf::one()) == 0 {
            nat_tail += 1;
            continue;
        }
        let e = block_exponent(p)?;
        // ψ_v(0)·X keeps the ψ_v(0) factor even when ordinal addition would
        // absorb it (ψ_v(0) + X = X for X > ψ_v(0)); the product form is
        // meaningful in MOCF (rows 367/373).
        let combined = if deg.compare(&e) < 0
            && matches!(&deg, Mocf::Psi(v, a)
                if !matches!(v.as_ref(), Mocf::Zero) && matches!(a.as_ref(), Mocf::Zero))
        {
            Mocf::from_prim_list(vec![deg.clone(), e.clone()])
        } else {
            deg.add(&e)
        };
        result = result.add(&Mocf::omega_pow(combined));
    }
    if nat_tail > 0 {
        result = result.add(&a.mul_nat(nat_tail));
    }
    Ok(result)
}

/// SSS-specific NOCF display: ψ_0(0) = 1, ψ_α(0) = Ω_α, ψ_α(β) = Ω_α + β.
/// A bare natural number (represented as ψ_0^n(0)) is displayed as the number.
pub fn nocf_to_sss_string(n: &Nocf) -> String {
    // Check if the whole expression is a natural number
    let nat = n.to_nat();
    if nat >= 0 {
        return nat.to_string();
    }
    match n {
        Nocf::Zero => "0".to_string(),
        Nocf::Psi(v, a) => {
            match (v.is_zero(), a.is_zero()) {
                // ψ_0(0) = 1
                (true, true) => "1".to_string(),
                // ψ_0(α) = ψ(α)
                (true, false) => format!("\\psi\\left({}\\right)", nocf_to_sss_string(a)),
                // ψ_α(0) = Ω_α
                (false, true) => {
                    let sub = nocf_to_sss_string(v);
                    if sub == "1" {
                        "\\Omega".to_string()
                    } else {
                        format!("\\Omega_{{{}}}", sub)
                    }
                }
                // ψ_α(β) = Ω_α + β
                (false, false) => {
                    let sub = nocf_to_sss_string(v);
                    let omega = if sub == "1" {
                        "\\Omega".to_string()
                    } else {
                        format!("\\Omega_{{{}}}", sub)
                    };
                    format!("{}+{}", omega, nocf_to_sss_string(a))
                }
            }
        }
    }
}

impl fmt::Display for Mocf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mocf::Zero => write!(f, "0"),
            Mocf::Sum(_) => {
                let groups = display_groups(self);
                if groups.is_empty() { return write!(f, "0"); }
                let mut first = true;
                for (b, k) in &groups {
                    if !first { write!(f, "+")?; }
                    first = false;
                    if b.compare(&Mocf::one()) == 0 {
                        if *k > 1 { write!(f, "{}", k)?; } else { write!(f, "1")?; }
                    } else if *k > 1 {
                        write!(f, "{}{}", b, k)?;
                    } else {
                        write!(f, "{}", b)?;
                    }
                }
                Ok(())
            }
            Mocf::OmegaPow(a) => {
                if a.is_zero() { return write!(f, "1"); }
                if matches!(a.as_ref(), Mocf::OmegaPow(a2) if a2.is_zero()) {
                    return write!(f, "\\omega");
                }
                // ω^{q·Y} = q^Y for ω^-fixed points q (Ω_α, ψ-values).
                if let Some((q, y)) = extract_qy(a) {
                    return if y.compare(&Mocf::one()) == 0 {
                        write!(f, "{}", q)
                    } else {
                        write!(f, "{}^{{{}}}", q, y)
                    };
                }
                // ω^{ω^{q·Y}} = q^{q^Y}, e.g. ω^{Ω^Ω} = Ω^{Ω^Ω}.
                if let Mocf::OmegaPow(i) = a.as_ref() {
                    if let Some((q, y)) = extract_qy(i) {
                        return if y.compare(&Mocf::one()) == 0 {
                            write!(f, "{}", q)
                        } else {
                            write!(
                                f,
                                "{}^{{{}}}",
                                q,
                                Mocf::Pow(Box::new(q.clone()), Box::new(y))
                            )
                        };
                    }
                }
                // Remaining ψ-factors of the exponent.
                let has_psi = a.prim_list().iter().any(|p| is_psi_value(p));
                if !has_psi {
                    return write!(f, "\\omega^{{{}}}", a);
                }
                let groups = display_groups(a);
                for (b, k) in &groups {
                    if is_psi_value(b) {
                        if *k > 1 { write!(f, "{}^{{{}}}", b, k)?; }
                        else { write!(f, "{}", b)?; }
                    } else if b.compare(&Mocf::one()) == 0 {
                        // ω^{1·k} = ω^k
                        if *k == 1 { write!(f, "\\omega")?; }
                        else { write!(f, "\\omega^{{{}}}", k)?; }
                    } else if *k > 1 {
                        write!(f, "\\omega^{{{}{}}}", b, k)?;
                    } else {
                        write!(f, "\\omega^{{{}}}", b)?;
                    }
                }
                Ok(())
            }
            Mocf::Omega(a) => {
                if matches!(a.as_ref(), Mocf::OmegaPow(a2) if a2.is_zero()) { write!(f, "\\Omega") }
                else { write!(f, "\\Omega_{{{}}}", a) }
            }
            Mocf::Psi(v, a) => {
                if v.is_zero() { write!(f, "\\psi({})", a) }
                else { write!(f, "\\psi_{{{}}}({})", v, a) }
            }
            Mocf::Pow(b, e) => {
                if e.is_zero() {
                    write!(f, "1")
                } else if matches!(e.as_ref(), Mocf::OmegaPow(x) if x.is_zero()) {
                    write!(f, "{}", b)
                } else {
                    write!(f, "{}", tower_format(b, e))
                }
            }
        }
    }
}

/// Value equality of two parsed MOCF expressions (canonical comparison).
pub fn mocf_value_eq(a: &Mocf, b: &Mocf) -> bool {
    a.compare(b) == 0
}

pub fn parse_mocf(input: &str) -> Result<Mocf, String> {
    let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s == "0" { return Ok(Mocf::zero()); }
    if s.chars().all(|c| c.is_ascii_digit()) {
        let n: i32 = s.parse().map_err(|_| "invalid number".to_string())?;
        return Ok(Mocf::from_nat(n));
    }
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    let result = parse_mocf_expr(&chars, &mut pos)?;
    if pos != chars.len() { return Err(format!("trailing: {}", chars[pos..].iter().collect::<String>())); }
    Ok(result)
}

fn parse_mocf_expr(chars: &[char], pos: &mut usize) -> Result<Mocf, String> {
    let mut terms = vec![parse_mocf_term(chars, pos)?];
    while *pos < chars.len() && chars[*pos] == '+' {
        *pos += 1;
        terms.push(parse_mocf_term(chars, pos)?);
    }
    Ok(Mocf::from_prim_list(terms))
}

fn parse_mocf_term(chars: &[char], pos: &mut usize) -> Result<Mocf, String> {
    let mut base = parse_mocf_pow_factor(chars, pos)?;
    while *pos < chars.len()
        && (chars[*pos..].starts_with(&['\\', 'c', 'd', 'o', 't'])
            || matches!(chars[*pos], '·' | '×' | '*'))
    {
        if chars[*pos] == '\\' { *pos += 5; } else { *pos += 1; }
        let rhs = parse_mocf_pow_factor(chars, pos)?;
        base = mul_mocf(&base, &rhs)?;
    }
    Ok(base)
}

/// A factor with its ^ chain and an optional juxtaposed natural
/// coefficient (Ω2 = Ω·2).
fn parse_mocf_pow_factor(chars: &[char], pos: &mut usize) -> Result<Mocf, String> {
    let base = parse_mocf_factor(chars, pos)?;
    let base = parse_mocf_pow_rhs(chars, pos, base)?;
    if *pos < chars.len() && chars[*pos].is_ascii_digit() && chars[*pos] != '0' {
        let mut n = 0i32;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            n = n * 10 + (chars[*pos] as i32 - '0' as i32);
            *pos += 1;
        }
        if n == 0 { return Err("coeff 0".to_string()); }
        return Ok(base.mul_nat(n));
    }
    Ok(base)
}

/// Right-associative power chain: a^b^c parses as a^(b^c).
fn parse_mocf_pow_rhs(chars: &[char], pos: &mut usize, base: Mocf) -> Result<Mocf, String> {
    if *pos >= chars.len() || chars[*pos] != '^' { return Ok(base); }
    *pos += 1;
    let exp = if *pos < chars.len() && chars[*pos] == '{' {
        *pos += 1;
        let e = parse_mocf_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != '}' { return Err("expected '}'".to_string()); }
        *pos += 1;
        e
    } else {
        let f = parse_mocf_factor(chars, pos)?;
        parse_mocf_pow_rhs(chars, pos, f)?
    };
    let base = mocf_pow(base, exp)?;
    parse_mocf_pow_rhs(chars, pos, base)
}

fn parse_mocf_factor(chars: &[char], pos: &mut usize) -> Result<Mocf, String> {
    if *pos >= chars.len() { return Err("unexpected end".to_string()); }
    let c = chars[*pos];
    if c == '0' { *pos += 1; return Ok(Mocf::zero()); }
    if c.is_ascii_digit() && c != '0' {
        let mut n = 0i32;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            n = n * 10 + (chars[*pos] as i32 - '0' as i32);
            *pos += 1;
        }
        return Ok(Mocf::from_nat(n));
    }
    if c == '(' { *pos += 1; let e = parse_mocf_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
        *pos += 1; return Ok(e); }
    if c == 'ω' || c == 'w' {
        *pos += 1;
        return Ok(Mocf::omega_pow(Mocf::one()));
    }
    if c == 'Ω' || c == 'W' {
        *pos += 1;
        let mut sub = Mocf::one();
        if *pos < chars.len() && chars[*pos] == '_' {
            *pos += 1;
            if *pos < chars.len() && chars[*pos] == '(' {
                *pos += 1; sub = parse_mocf_expr(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
                *pos += 1;
            } else { sub = parse_mocf_factor(chars, pos)?; }
        }
        return Ok(Mocf::omega(sub));
    }
    if c == 'ψ' || c == 'p' {
        *pos += 1;
        let mut v = Mocf::zero();
        if *pos < chars.len() && chars[*pos] == '_' {
            *pos += 1;
            if *pos < chars.len() && chars[*pos] == '(' {
                *pos += 1; v = parse_mocf_expr(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
                *pos += 1;
            } else { v = parse_mocf_term(chars, pos)?; }
        }
        if *pos >= chars.len() || chars[*pos] != '(' { return Err("expected '('".to_string()); }
        *pos += 1; let a = parse_mocf_expr(chars, pos)?;
        if *pos >= chars.len() || chars[*pos] != ')' { return Err("expected ')'".to_string()); }
        *pos += 1;
        return Ok(Mocf::psi(v, a));
    }
    Err(format!("unexpected '{}'", c))
}

pub fn analyze_mocf(input: &str) -> Result<String, String> {
    Ok(parse_mocf(input)?.to_string())
}

pub fn expand_mocf(input: &str, fs: i32) -> Result<String, String> {
    let expr = parse_mocf(input)?;
    Ok(expr.expand(fs).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nocf(s: &str) -> Nocf { parse_nocf(s).unwrap() }
    fn mocf(s: &str) -> Mocf { parse_mocf(s).unwrap() }

    #[test]
    fn nocf_parse_display() {
        assert_eq!(nocf("0").to_string(), "0");
        assert_eq!(nocf("ψ(0)").to_string(), "\\psi(0)");
        assert_eq!(nocf("ψ_1(0)").to_string(), "\\psi_{1}(0)");
        assert_eq!(nocf("1").to_string(), "\\psi(0)");
        assert_eq!(nocf("2").to_string(), "\\psi(\\psi(0))");
    }

    #[test]
    fn nocf_sugar_display() {
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ(0)").unwrap()), "1");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ(ψ(0))").unwrap()), "2");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("3").unwrap()), "3");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ_1(0)").unwrap()), "\\Omega");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ_2(0)").unwrap()), "\\Omega_{2}");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ(Ω)").unwrap()), "\\omega");
        assert_eq!(nocf_to_sugar_string(&parse_nocf("ψ_Ω(0)").unwrap()), "\\Omega_{\\Omega}");
    }

    #[test]
    fn nocf_expand() {
        // ψ(0) = 1, FS(1, 0) = 0
        assert_eq!(parse_nocf("ψ(0)").unwrap().expand(0).to_string(), "0");
        // ψ(ψ(0)) = 2, FS(2, 0) = ψ(0) = 1
        assert_eq!(parse_nocf("ψ(ψ(0))").unwrap().expand(0).to_string(), "\\psi(0)");
        // ψ(ψ(0)) = 2, FS(2, 1) = ψ(ψ(0)) = 2? No, ψ(ψ(0)) is a successor, FS(2, 1) = ψ(0) = 1
        // Actually FS(ψ(ψ(0)), 1) = ψ(0) for successor, same as FS(..., 0) because
        // the FS of a successor only depends on the cofinality of the argument
        assert_eq!(parse_nocf("ψ(ψ(0))").unwrap().expand(1).to_string(), "\\psi(0)");
    }

    #[test]
    fn nocf_compare() {
        assert_eq!(nocf("0").compare(&nocf("0")), 0);
        assert!(nocf("ψ(0)").compare(&nocf("0")) > 0);
        assert!(nocf("0").compare(&nocf("ψ(0)")) < 0);
        assert!(nocf("ψ(ψ(0))").compare(&nocf("ψ(0)")) > 0);
    }

    #[test]
    fn mocf_parse_display() {
        assert_eq!(mocf("0").to_string(), "0");
        assert_eq!(mocf("1").to_string(), "1");
        assert_eq!(mocf("ω").to_string(), "\\omega");
        assert_eq!(mocf("Ω").to_string(), "\\Omega");
        assert_eq!(mocf("ψ(0)").to_string(), "\\psi(0)");
        assert_eq!(mocf("ψ_Ω(0)").to_string(), "\\psi_{\\Omega}(0)");
        assert_eq!(mocf("ω^ω").to_string(), "\\omega^{\\omega}");
        assert_eq!(mocf("Ω_Ω").to_string(), "\\Omega_{\\Omega}");
    }

    #[test]
    fn mocf_display_normalize() {
        // Natural numbers merge: 1+1 = 2 (not "1+1").
        assert_eq!(mocf("2").to_string(), "2");
        assert_eq!(mocf("1+1").to_string(), "2");
        assert_eq!(mocf("12").to_string(), "12");
        // Like terms merge into coefficients.
        assert_eq!(mocf("ω+ω").to_string(), "\\omega2");
        assert_eq!(mocf("ω2").to_string(), "\\omega2");
        assert_eq!(mocf("ω2").to_string(), "\\omega2");
        assert_eq!(mocf("ψ(0)+ψ(0)").to_string(), "\\psi(0)2");
        // Left absorption: A + B = B when A < B.
        assert_eq!(mocf("ω+ψ(0)").to_string(), "\\psi(0)");
        assert_eq!(mocf("ω+1+ψ(0)").to_string(), "\\psi(0)");
        assert_eq!(mocf("ψ(0)+ψ(1)").to_string(), "\\psi(1)");
        assert_eq!(mocf("ω^ψ(0)+ψ(1)").to_string(), "\\psi(1)");
        // Descending sums stay.
        assert_eq!(mocf("ω^2+ω").to_string(), "\\omega^{2}+\\omega");
        assert_eq!(mocf("ω+2").to_string(), "\\omega+2");
        // Mixed coefficient + rest.
        assert_eq!(mocf("ω^2+ω+ω+1").to_string(), "\\omega^{2}+\\omega2+1");
        // Subscripts and ψ-arguments normalize too.
        assert_eq!(mocf("Ω_2").to_string(), "\\Omega_{2}");
        assert_eq!(mocf("ψ_2(0)").to_string(), "\\psi_{2}(0)");
        assert_eq!(mocf("ψ(1+1)").to_string(), "\\psi(2)");
        // Ω = ψ_1(0): equal values merge, no mis-absorption.
        assert_eq!(mocf("Ω+ψ_1(0)").to_string(), "\\Omega2");
        assert_eq!(mocf("ψ_1(0)+ψ_1(0)").to_string(), "\\psi_{1}(0)2");
        // ε_1 + Ω = Ω, but Ω + ε_1 stays.
        assert_eq!(mocf("ψ(1)+Ω").to_string(), "\\Omega");
        assert_eq!(mocf("Ω+ψ(1)").to_string(), "\\Omega+\\psi(1)");
    }

    #[test]
    fn mocf_pow_operator() {
        // ψ_a(b)^c = ω^{ψ_a(b)·c} (ordinary ordinal powers of ε-numbers).
        assert_eq!(mocf("ψ(0)^0").to_string(), "1");
        assert_eq!(mocf("ψ(0)^1").to_string(), "\\psi(0)");
        assert_eq!(mocf("ψ(0)^2").to_string(), "\\psi(0)^{2}");
        assert_eq!(mocf("ψ_1(0)^2").to_string(), "\\psi_{1}(0)^{2}");
        // Ω_α^c = ω^{Ω_α·c}.
        assert_eq!(mocf("Ω^2").to_string(), "\\Omega^{2}");
        assert_eq!(mocf("Ω_2^ω").to_string(), "\\Omega_{2}^{\\omega}");
        assert_eq!(mocf("Ω^Ω").to_string(), "\\Omega^{\\Omega}");
        assert_eq!(mocf("Ω^ω").to_string(), "\\Omega^{\\omega}");
        // Ω·ψ_1(1) = ψ_1(1), so Ω^ψ_1(1) = ψ_1(1).
        assert_eq!(mocf("Ω^ψ_1(1)").to_string(), "\\psi_{1}(1)");
        // Tower powers display right-associated: Ω^(Ω^Ω) = Ω^Ω^Ω.
        assert_eq!(mocf("Ω^(Ω^Ω)").to_string(), "\\Omega^{\\Omega^{\\Omega}}");
        assert_eq!(mocf("Ω^Ω^Ω").to_string(), "\\Omega^{\\Omega^{\\Omega}}");
        assert_eq!(mocf("ψ(0)^(ψ(0)^ψ(0))").to_string(), "\\psi(0)^{\\psi(0)^{\\psi(0)}}");
        // ω^(q×n) = q^n for any ω^-fixed point q.
        assert_eq!(mocf("ω^(Ω3)").to_string(), "\\Omega^{3}");
        assert_eq!(mocf("ω^(ψ(0)2)").to_string(), "\\psi(0)^{2}");
        // ω^(q^n) = q^q^(n-1) for n < ω; ω^(q^α) = q^(q^α) otherwise.
        assert_eq!(mocf("ω^(Ω^3)").to_string(), "\\Omega^{\\Omega^{2}}");
        assert_eq!(mocf("ω^(Ω^ω)").to_string(), "\\Omega^{\\Omega^{\\omega}}");
        assert_eq!(mocf("ω^Ω^Ω").to_string(), "\\Omega^{\\Omega^{\\Omega}}");
        // ω^{Ω·Y} displays as Ω^Y.
        assert_eq!(mocf("ω^(Ω^2)").to_string(), "\\Omega^{\\Omega}");
        assert_eq!(mocf("ω^(Ω2)").to_string(), "\\Omega^{2}");
        assert_eq!(mocf("ω^(Ω^Ω)").to_string(), "\\Omega^{\\Omega^{\\Omega}}");
        // Powers with ψ-exponents.
        assert_eq!(mocf("ψ(0)^ψ(0)").to_string(), "\\psi(0)^{\\psi(0)}");
        // Infinite exponents.
        assert_eq!(mocf("ψ(0)^ω").to_string(), "\\psi(0)^{\\omega}");
        assert_eq!(mocf("ψ(0)^{ω}").to_string(), "\\psi(0)^{\\omega}");
        assert_eq!(mocf("ψ(0)^{ω2}").to_string(), "\\psi(0)^{\\omega2}");
        // Coefficient applies to the power.
        assert_eq!(mocf("ψ(0)^1\\cdot 2").to_string(), "\\psi(0)2");
        // ω^{ψ·k} displays as ψ^k.
        assert_eq!(mocf("ω^ψ(0)").to_string(), "\\psi(0)");
        assert_eq!(mocf("ω^(ψ(0)+ψ(0))").to_string(), "\\psi(0)^{2}");
        assert_eq!(mocf("ω^(ψ(0)+ω)").to_string(), "\\psi(0)\\omega^{\\omega}");
        // Non-fixed-point bases are rejected.
        assert!(parse_mocf("2^3").is_err());
    }

    #[test]
    fn mocf_mul() {
        // ω^α × ω^β = ω^(α+β); Ω and ψ-values are ω^-fixed points.
        assert_eq!(mocf("Ω×ω").to_string(), "\\Omega\\omega");
        assert_eq!(mocf("Ω×ω^ω").to_string(), "\\Omega\\omega^{\\omega}");
        assert_eq!(mocf("ω×Ω").to_string(), "\\Omega");
        assert_eq!(mocf("Ω×Ω").to_string(), "\\Omega^{2}");
        assert_eq!(mocf("ω^2×ω^3").to_string(), "\\omega^{5}");
        assert_eq!(mocf("ψ(0)×ω").to_string(), "\\psi(0)\\omega");
        assert_eq!(mocf("ψ(1)×ω").to_string(), "\\psi(1)\\omega");
        assert_eq!(mocf("ψ_1(1)×ω").to_string(), "\\psi_{1}(1)\\omega");
        assert_eq!(mocf("Ω×ψ(0)").to_string(), "\\Omega\\psi(0)");
        assert_eq!(mocf("Ω×ψ_1(1)").to_string(), "\\psi_{1}(1)");
        // Left-distributive over the right operand.
        assert_eq!(mocf("ω×(ω+1)").to_string(), "\\omega^{2}+\\omega");
        assert_eq!(mocf("(ω^2+ω)×ω").to_string(), "\\omega^{3}");
        // Natural factors.
        assert_eq!(mocf("(ω+1)×2").to_string(), "\\omega2+1");
        assert_eq!(mocf("2×ω").to_string(), "\\omega");
        assert_eq!(mocf("Ω\\cdot 2").to_string(), "\\Omega2");
    }

    #[test]
    fn mocf_expand() {
        // ω[1] = 1
        assert_eq!(mocf("ω").expand(1).to_string(), "1");
        // ψ(0)[0] = 0
        assert_eq!(mocf("ψ(0)").expand(0).to_string(), "0");
        // ψ(0)[1] = 1 (Madore's OCF uses shifted indexing)
        assert_eq!(mocf("ψ(0)").expand(1).to_string(), "1");
        // ψ(0)[2] = ω
        assert_eq!(mocf("ψ(0)").expand(2).to_string(), "\\omega");
    }

    #[test]
    fn mocf_compare() {
        // ψ(0) < Ω (ε₀ < first uncountable)
        assert!(mocf("ψ(0)").compare(&mocf("Ω")) < 0);
        // ω > 1
        assert!(mocf("ω").compare(&mocf("1")) > 0);
        // Ω > ω
        assert!(mocf("Ω").compare(&mocf("ω")) > 0);
        // Ω = Ω
        assert_eq!(mocf("Ω").compare(&mocf("Ω")), 0);
        // 0 = 0
        assert_eq!(mocf("0").compare(&mocf("0")), 0);
    }
}