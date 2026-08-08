//! Mahlo BOCF: AST definition and a lenient parser.
//!
//! Mahlo BOCF is a *display* notation for IHSS Hydra. This module defines the
//! abstract syntax tree and a parser that accepts both Unicode (ψ, Ω, ω, M,
//! ×) and LaTeX (\psi, \Omega, \omega, \times) spellings, plus underscore
//! subscript shorthand (M_2, Ω_2, ψ_{...}).

/// Abstract syntax tree for a Mahlo BOCF term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MboTerm {
    /// The successor-addend `1` and natural numbers (`0`, `2`, ...).
    Nat(u64),
    /// The limit `ω` (epsilon base).
    OmegaTerm,
    /// The regular `Ω`, or `Ω_n` when `Some(n)`.
    Omega(Option<u64>),
    /// The Mahlo `M`, or `M_n` when `Some(n)`.
    Mahlo(Option<u64>),
    /// `ψ_α(β)`.
    Psi { sub: Box<MboTerm>, arg: Box<MboTerm> },
    /// `α × n` for a natural number `n`.
    Mul(Box<MboTerm>, u64),
    /// `α + β + …`, order-preserving (ordinal addition is not commutative).
    Add(Vec<MboTerm>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Num(u64),
    Psi,
    Omega,
    OmegaTerm,
    Mahlo,
    Times,
    Plus,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Sub,
}

fn tokenize(input: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '0'..='9' => {
                let mut j = i;
                let mut n: u64 = 0;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    n = n.saturating_mul(10).saturating_add(chars[j].to_digit(10).unwrap() as u64);
                    j += 1;
                }
                toks.push(Tok::Num(n));
                i = j;
            }
            '\\' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                let name: String = chars[i + 1..j].iter().collect();
                let tok = match name.as_str() {
                    "psi" => Some(Tok::Psi),
                    "Omega" => Some(Tok::Omega),
                    "omega" => Some(Tok::OmegaTerm),
                    "times" => Some(Tok::Times),
                    _ => None,
                };
                match tok {
                    Some(t) => toks.push(t),
                    None => return Err(format!("unknown LaTeX command \\{}", name)),
                }
                i = j;
            }
            'ψ' => {
                toks.push(Tok::Psi);
                i += 1;
            }
            'Ω' => {
                toks.push(Tok::Omega);
                i += 1;
            }
            'ω' => {
                toks.push(Tok::OmegaTerm);
                i += 1;
            }
            'M' => {
                toks.push(Tok::Mahlo);
                i += 1;
            }
            '+' => {
                toks.push(Tok::Plus);
                i += 1;
            }
            '×' | '*' => {
                toks.push(Tok::Times);
                i += 1;
            }
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            '{' => {
                toks.push(Tok::LBrace);
                i += 1;
            }
            '}' => {
                toks.push(Tok::RBrace);
                i += 1;
            }
            '_' => {
                toks.push(Tok::Sub);
                i += 1;
            }
            _ => {
                return Err(format!("unexpected character `{}`", c));
            }
        }
    }
    Ok(toks)
}

pub fn parse_mbocf(input: &str) -> Result<MboTerm, String> {
    let toks = tokenize(input)?;
    let mut p = Parser { toks, pos: 0 };
    if p.toks.is_empty() {
        return Err("empty input".to_string());
    }
    let term = p.parse_expr()?;
    if p.pos != p.toks.len() {
        return Err(format!("unexpected trailing input at position {}", p.pos));
    }
    Ok(term)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    // expr := mul { '+' mul }
    fn parse_expr(&mut self) -> Result<MboTerm, String> {
        let mut parts = vec![self.parse_mul()?];
        while self.peek() == Some(&Tok::Plus) {
            self.bump();
            parts.push(self.parse_mul()?);
        }
        if parts.len() == 1 {
            Ok(parts.pop().unwrap())
        } else {
            Ok(MboTerm::Add(parts))
        }
    }

    // mul := atom [ '×' num ]
    fn parse_mul(&mut self) -> Result<MboTerm, String> {
        let base = self.parse_atom()?;
        if self.peek() == Some(&Tok::Times) {
            self.bump();
            match self.bump() {
                Some(Tok::Num(n)) => Ok(MboTerm::Mul(Box::new(base), n)),
                _ => Err("expected a natural number after ×".to_string()),
            }
        } else {
            Ok(base)
        }
    }

    fn parse_atom(&mut self) -> Result<MboTerm, String> {
        match self.bump() {
            Some(Tok::Num(0)) => Ok(MboTerm::Nat(0)),
            Some(Tok::Num(n)) => Ok(MboTerm::Nat(n)),
            Some(Tok::OmegaTerm) => Ok(MboTerm::OmegaTerm),
            Some(Tok::Omega) => {
                let n = self.parse_optional_number_subscript()?;
                Ok(MboTerm::Omega(n))
            }
            Some(Tok::Mahlo) => {
                let n = self.parse_optional_number_subscript()?;
                Ok(MboTerm::Mahlo(n))
            }
            Some(Tok::LParen) => {
                let inner = self.parse_expr()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(inner),
                    _ => Err("expected `)`".to_string()),
                }
            }
            Some(Tok::Psi) => self.parse_psi(),
            _ => Err("expected a term".to_string()),
        }
    }

    fn parse_optional_number_subscript(&mut self) -> Result<Option<u64>, String> {
        if self.peek() == Some(&Tok::Sub) {
            self.bump();
            if self.peek() == Some(&Tok::LBrace) {
                self.bump();
                let n = self.expect_num()?;
                match self.bump() {
                    Some(Tok::RBrace) => Ok(Some(n)),
                    _ => Err("expected `}` in subscript".to_string()),
                }
            } else {
                Ok(Some(self.expect_num()?))
            }
        } else {
            Ok(None)
        }
    }

    fn expect_num(&mut self) -> Result<u64, String> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(n),
            _ => Err("expected a natural number in subscript".to_string()),
        }
    }

    fn parse_psi(&mut self) -> Result<MboTerm, String> {
        let sub = if self.peek() == Some(&Tok::Sub) {
            self.bump();
            if self.peek() == Some(&Tok::LBrace) {
                self.bump();
                let s = self.parse_expr()?;
                match self.bump() {
                    Some(Tok::RBrace) => s,
                    _ => return Err("expected `}` in ψ subscript".to_string()),
                }
            } else {
                self.parse_atom()?
            }
        } else {
            // A bare ψ defaults to ψ_Ω.
            MboTerm::Omega(None)
        };

        match self.bump() {
            Some(Tok::LParen) => {}
            _ => return Err("expected `(` after ψ subscript".to_string()),
        }
        let arg = self.parse_expr()?;
        match self.bump() {
            Some(Tok::RParen) => Ok(MboTerm::Psi {
                sub: Box::new(sub),
                arg: Box::new(arg),
            }),
            _ => Err("expected `)` in ψ argument".to_string()),
        }
    }
}

impl MboTerm {
    /// Canonical form: undo sugar (Ω→ψ_M(M), Ω_n→ψ_M(M×n), ω→ψ_Ω(1)) and
    /// split merges (α×n → n consecutive α, n>1 → n consecutive 1s), with
    /// nested `Add` flattened. Order is preserved (ordinal addition is not
    /// commutative); distinct terms are never merged.
    pub fn canonicalize(&self) -> MboTerm {
        match self {
            MboTerm::Omega(None) => MboTerm::Psi {
                sub: Box::new(MboTerm::Mahlo(None)),
                arg: Box::new(MboTerm::Mahlo(None)),
            },
            MboTerm::Omega(Some(n)) => MboTerm::Psi {
                sub: Box::new(MboTerm::Mahlo(None)),
                arg: Box::new(MboTerm::Mul(Box::new(MboTerm::Mahlo(None)), *n).canonicalize()),
            },
            MboTerm::OmegaTerm => MboTerm::Psi {
                sub: Box::new(MboTerm::Omega(None).canonicalize()),
                arg: Box::new(MboTerm::Nat(1)),
            },
            MboTerm::Mul(t, n) => {
                let ct = t.canonicalize();
                MboTerm::Add(vec![ct; *n as usize])
            }
            MboTerm::Nat(n) if *n > 1 => MboTerm::Add(vec![MboTerm::Nat(1); *n as usize]),
            MboTerm::Nat(n) => MboTerm::Nat(*n),
            MboTerm::Mahlo(k) => MboTerm::Mahlo(*k),
            MboTerm::Add(terms) => {
                let mut flat = Vec::new();
                for t in terms {
                    match t.canonicalize() {
                        MboTerm::Add(inner) => flat.extend(inner),
                        other => flat.push(other),
                    }
                }
                MboTerm::Add(flat)
            }
            MboTerm::Psi { sub, arg } => MboTerm::Psi {
                sub: Box::new(sub.canonicalize()),
                arg: Box::new(arg.canonicalize()),
            },
        }
    }

    /// Reverse-construct the IHSS Hydra from a Mahlo BOCF term.
    ///
    /// The target is the **value matrix**: `value[c][r] = (parent==-1) ? 0 :
    /// value[parent][r] + 1`. We build the value matrix directly (each column
    /// `(depth, level)`), then derive the parent matrix via `IHSS::from_value`.
    pub fn to_ihss(&self) -> Result<crate::ihss::IHSS, String> {
        let canon = self.canonicalize();
        let value = Self::build_value(&canon, 0)?;
        Ok(crate::ihss::IHSS::from_value(&value))
    }

    /// Build the value matrix for a canonical term. `depth` is the row-0 value
    /// of the current node (children get `depth+1`); `level` is the row-1 value.
    fn build_value(term: &MboTerm, depth: i32) -> Result<Vec<Vec<i32>>, String> {
        match term {
            MboTerm::Nat(_) => Ok(vec![vec![depth, 0]]),
            MboTerm::Mahlo(k) => Ok(vec![vec![depth, k.map(|x| x as i32).unwrap_or(1)]]),
            MboTerm::Add(terms) => {
                // Multiple roots: each addend is an independent tree at depth 0.
                let mut out = Vec::new();
                for t in terms {
                    out.extend(Self::build_value(t, 0)?);
                }
                Ok(out)
            }
            MboTerm::Psi { sub, arg } => {
                let (k, s_children, t_children) = Self::extract_children(sub, arg)?;
                let mut out = vec![vec![depth, k]];
                for s in s_children {
                    out.extend(Self::build_value(&s, depth + 1)?);
                }
                for t in t_children {
                    out.extend(Self::build_value(&t, depth + 1)?);
                }
                Ok(out)
            }
            _ => Err("internal: unexpected term after canonicalize".to_string()),
        }
    }

    /// Invert the renderer's s/t split. Returns (node level k, s-children, t-children).
    fn extract_children(sub: &MboTerm, arg: &MboTerm) -> Result<(i32, Vec<MboTerm>, Vec<MboTerm>), String> {
        match sub {
            // Case A: bare M_{k+1}, t empty → arg carries the s-children.
            MboTerm::Mahlo(k) => {
                let n = k.map(|x| x as i32).unwrap_or(1);
                Ok((n - 1, Self::split_terms(arg), Vec::new()))
            }
            // Case B: ψ_{M_{k+1}}(…), t from arg; s from inner minus trailing phantom M_n.
            MboTerm::Psi { sub: sub2, arg: inner } => {
                let n = match sub2.as_ref() {
                    MboTerm::Mahlo(k) => k.map(|x| x as i32).unwrap_or(1),
                    _ => return Err("ψ subscript must be a Mahlo".to_string()),
                };
                let k = n - 1;
                let mut s = Self::split_terms(inner);
                if let Some(last) = s.last() {
                    if Self::is_mahlo_n(last, n) {
                        s.pop();
                    }
                }
                let t = Self::split_terms(arg);
                Ok((k, s, t))
            }
            _ => Err("invalid ψ subscript".to_string()),
        }
    }

    fn split_terms(t: &MboTerm) -> Vec<MboTerm> {
        match t {
            MboTerm::Add(ts) => ts.clone(),
            other => vec![other.clone()],
        }
    }

    fn is_mahlo_n(t: &MboTerm, n: i32) -> bool {
        match t {
            MboTerm::Mahlo(k) => k.map(|x| x as i32).unwrap_or(1) == n,
            _ => false,
        }
    }

    /// Indented tree lines for display as an AST.
    pub fn to_ast_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        self.collect_lines(0, &mut lines);
        lines
    }

    fn collect_lines(&self, indent: usize, out: &mut Vec<String>) {
        let pad = "  ".repeat(indent);
        match self {
            MboTerm::Nat(n) => out.push(format!("{}Nat({})", pad, n)),
            MboTerm::OmegaTerm => out.push(format!("{}omega", pad)),
            MboTerm::Omega(None) => out.push(format!("{}Omega", pad)),
            MboTerm::Omega(Some(n)) => out.push(format!("{}Omega_{}", pad, n)),
            MboTerm::Mahlo(None) => out.push(format!("{}Mahlo", pad)),
            MboTerm::Mahlo(Some(n)) => out.push(format!("{}Mahlo_{}", pad, n)),
            MboTerm::Psi { sub, arg } => {
                out.push(format!("{}Psi", pad));
                out.push(format!("{}  sub:", pad));
                sub.collect_lines(indent + 1, out);
                out.push(format!("{}  arg:", pad));
                arg.collect_lines(indent + 1, out);
            }
            MboTerm::Mul(t, n) => {
                out.push(format!("{}Mul({})", pad, n));
                t.collect_lines(indent + 1, out);
            }
            MboTerm::Add(terms) => {
                out.push(format!("{}Add", pad));
                for t in terms {
                    t.collect_lines(indent + 1, out);
                }
            }
        }
    }

    /// Regenerate Mahlo BOCF LaTeX from the AST (round-trips the parser).
    pub fn to_latex(&self) -> String {
        match self {
            MboTerm::Nat(n) => n.to_string(),
            MboTerm::OmegaTerm => "\\omega".to_string(),
            MboTerm::Omega(None) => "\\Omega".to_string(),
            MboTerm::Omega(Some(n)) => format!("\\Omega_{{{}}}", n),
            MboTerm::Mahlo(None) => "M".to_string(),
            MboTerm::Mahlo(Some(n)) => format!("M_{{{}}}", n),
            MboTerm::Psi { sub, arg } => {
                format!("\\psi_{{{}}}({})", sub.to_latex(), arg.to_latex())
            }
            MboTerm::Mul(t, n) => format!("{} \\times {}", t.to_latex(), n),
            MboTerm::Add(terms) => {
                let parts: Vec<String> = terms.iter().map(|t| t.to_latex()).collect();
                parts.join(" + ")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(input: &str) -> String {
        parse_mbocf(input).unwrap().to_latex()
    }

    #[test]
    fn parse_psi_m() {
        assert_eq!(roundtrip("\\psi_{M}(M)"), "\\psi_{M}(M)");
        assert_eq!(roundtrip("ψ_M(M)"), "\\psi_{M}(M)");
    }

    #[test]
    fn parse_bare_psi_defaults_to_omega() {
        assert_eq!(roundtrip("\\psi(1)"), "\\psi_{\\Omega}(1)");
        assert_eq!(roundtrip("ψ(1)"), "\\psi_{\\Omega}(1)");
    }

    #[test]
    fn parse_nested_subscript() {
        assert_eq!(
            roundtrip("\\psi_{\\psi_M(M)}(\\psi_M(M))"),
            "\\psi_{\\psi_{M}(M)}(\\psi_{M}(M))"
        );
    }

    #[test]
    fn parse_add_and_mul() {
        assert_eq!(roundtrip("\\psi_M(M \\times 2) + 1"), "\\psi_{M}(M \\times 2) + 1");
    }

    #[test]
    fn parse_omega() {
        assert_eq!(roundtrip("\\Omega_2"), "\\Omega_{2}");
        assert_eq!(roundtrip("\\omega"), "\\omega");
    }

    #[test]
    fn ast_lines() {
        let t = parse_mbocf("\\psi_{M}(M)").unwrap();
        let lines = t.to_ast_lines();
        assert!(lines.iter().any(|l| l.trim() == "Psi"));
        assert!(lines.iter().any(|l| l.trim() == "Mahlo"));
    }

    /// Round-trip: build IHSS, render back to LaTeX, re-parse, canonicalize.
    /// Must equal canonicalizing the original input (canonicalize normalizes the
    /// merging of like terms, so `M×2` on either side compares equal to `M+M`).
    fn rt(input: &str) -> String {
        let term = parse_mbocf(input).unwrap();
        let ihss = term.to_ihss().unwrap();
        let back = parse_mbocf(&ihss.to_latex()).unwrap();
        back.canonicalize().to_latex()
    }

    #[test]
    fn roundtrip_psi_psi_m() {
        let t = parse_mbocf("\\psi_{\\psi_M(M)}(\\psi_M(M))").unwrap();
        assert_eq!(rt("\\psi_{\\psi_M(M)}(\\psi_M(M))"), t.canonicalize().to_latex());
    }

    #[test]
    fn roundtrip_m2_case() {
        assert_eq!(
            rt("\\psi_M(\\psi_{\\psi_{M_2}(M_2)}(M))"),
            parse_mbocf("\\psi_M(\\psi_{\\psi_{M_2}(M_2)}(M))").unwrap().canonicalize().to_latex()
        );
    }

    #[test]
    fn roundtrip_omega_and_sugar() {
        // Ω = ψ_M(M)
        assert_eq!(rt("\\Omega"), parse_mbocf("\\Omega").unwrap().canonicalize().to_latex());
        // ω = ψ_Ω(1)
        assert_eq!(rt("\\omega"), parse_mbocf("\\omega").unwrap().canonicalize().to_latex());
        // Ω_2 = ψ_M(M×2)
        assert_eq!(rt("\\Omega_2"), parse_mbocf("\\Omega_2").unwrap().canonicalize().to_latex());
    }

    #[test]
    fn roundtrip_add() {
        assert_eq!(
            rt("\\psi_{\\psi_M(M)}(1) + 1"),
            parse_mbocf("\\psi_{\\psi_M(M)}(1) + 1").unwrap().canonicalize().to_latex()
        );
    }

    #[test]
    fn value_self_consistency() {
        // to_value(from_value(value)) must equal value for every built matrix.
        for input in [
            "\\psi_{\\psi_M(M)}(\\psi_M(M))",
            "\\psi_M(\\psi_{\\psi_{M_2}(M_2)}(M))",
            "\\psi_M(M)",
            "\\psi_{\\psi_M(M)}(1) + 1",
            "\\psi_M(M \\times 2)",
        ] {
            let term = parse_mbocf(input).unwrap();
            let ihss = term.to_ihss().unwrap();
            let value = ihss.to_value();
            let rebuilt = crate::ihss::IHSS::from_value(&value);
            assert_eq!(rebuilt.to_value(), value, "value not self-consistent for {input}");
        }
    }

    #[test]
    fn known_value_matrices() {
        // Reverse of the forward renderer examples: reconstruct from LaTeX and
        // confirm the rendered Mahlo BOCF matches the source matrix's rendering.
        let cases = [
            ("\\psi_{\\psi_M(M)}(\\psi_M(M))", "(0,0)(1,0)(2,1)"),
            ("\\psi_M(\\psi_{\\psi_{M_2}(M_2)}(M))", "(0,0)(1,1)(2,1)"),
            ("\\psi_M(M)", "(0)(1,1)"),
            ("\\psi_{\\psi_M(M)}(1) + 1", "(0)(1)(0)"),
        ];
        for (input, src) in cases {
            let term = parse_mbocf(input).unwrap();
            let ihss = term.to_ihss().unwrap();
            let src_ihss = crate::ihss::IHSS::from_string(src).unwrap();
            assert_eq!(ihss.to_latex(), src_ihss.to_latex(), "for {input}");
        }
    }
}
