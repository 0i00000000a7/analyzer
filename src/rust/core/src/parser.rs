#![allow(unused_assignments)]
//! BOCF expression parser and BOCF → BMS conversion.

use crate::bms::BmsContext;
use crate::expand::expand_bms;
use crate::term::*;
use crate::Matrix;

// ============================================================
// AST
// ============================================================

#[derive(Debug, Clone)]
pub enum Ast {
    Num(i32),
    W,
    Omega(Option<Box<Ast>>),
    Psi(Option<Box<Ast>>, Box<Ast>),
    Add(Box<Ast>, Box<Ast>),
    Mul(Box<Ast>, Box<Ast>),
    Pow(Box<Ast>, Box<Ast>),
}

// ============================================================
// Lexer
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenKind {
    Num,
    Psi,
    Omega,
    OmegaLower,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Plus,
    Mul,
    Pow,
    Subscript,
    Eof,
}

#[derive(Debug, Clone, Copy)]
struct Token {
    kind: TokenKind,
    num_value: i32,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { src, pos: 0 }
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.src[self.pos..].starts_with(pat)
    }

    fn next(&mut self) -> Result<Token, String> {
        loop {
            if self.pos >= self.src.len() {
                return Ok(Token { kind: TokenKind::Eof, num_value: 0 });
            }
            let ch = self.src[self.pos..].chars().next().unwrap();

            // whitespace
            if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                self.pos += ch.len_utf8();
                continue;
            }

            // ψ (Unicode) or \psi (LaTeX) or psi (text)
            if self.starts_with("ψ") {
                self.pos += "ψ".len();
                return Ok(Token { kind: TokenKind::Psi, num_value: 0 });
            }
            if self.starts_with("\\psi") {
                self.pos += "\\psi".len();
                return Ok(Token { kind: TokenKind::Psi, num_value: 0 });
            }
            if self.starts_with("psi") {
                self.pos += "psi".len();
                return Ok(Token { kind: TokenKind::Psi, num_value: 0 });
            }

            // Ω (Unicode) or \Omega (LaTeX) or Omega (text)
            if self.starts_with("Ω") {
                self.pos += "Ω".len();
                return Ok(Token { kind: TokenKind::Omega, num_value: 0 });
            }
            if self.starts_with("\\Omega") {
                self.pos += "\\Omega".len();
                return Ok(Token { kind: TokenKind::Omega, num_value: 0 });
            }
            if self.starts_with("Omega") {
                self.pos += "Omega".len();
                return Ok(Token { kind: TokenKind::Omega, num_value: 0 });
            }

            // ω (Unicode) or \omega (LaTeX) or omega (text)
            if self.starts_with("ω") {
                self.pos += "ω".len();
                return Ok(Token { kind: TokenKind::OmegaLower, num_value: 0 });
            }
            if self.starts_with("\\omega") {
                self.pos += "\\omega".len();
                return Ok(Token { kind: TokenKind::OmegaLower, num_value: 0 });
            }
            if self.starts_with("omega") {
                self.pos += "omega".len();
                return Ok(Token { kind: TokenKind::OmegaLower, num_value: 0 });
            }

            // Single-character shortcuts: p → ψ, w → ω, W → Ω
            if ch == 'p' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::Psi, num_value: 0 });
            }
            if ch == 'w' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::OmegaLower, num_value: 0 });
            }
            if ch == 'W' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::Omega, num_value: 0 });
            }

            if ch == '_' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::Subscript, num_value: 0 });
            }
            if ch == '(' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::LParen, num_value: 0 });
            }
            if ch == ')' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::RParen, num_value: 0 });
            }
            if ch == '{' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::LBrace, num_value: 0 });
            }
            if ch == '}' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::RBrace, num_value: 0 });
            }
            if ch == '+' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::Plus, num_value: 0 });
            }
            if self.starts_with("**") {
                self.pos += 2;
                return Ok(Token { kind: TokenKind::Pow, num_value: 0 });
            }
            if self.starts_with("×") || ch == '*' {
                if ch == '*' {
                    self.pos += 1;
                } else {
                    self.pos += "×".len();
                }
                return Ok(Token { kind: TokenKind::Mul, num_value: 0 });
            }
            if ch == '^' {
                self.pos += 1;
                return Ok(Token { kind: TokenKind::Pow, num_value: 0 });
            }

            if ch.is_ascii_digit() {
                let mut v = 0;
                while self.pos < self.src.len() {
                    let c = self.src[self.pos..].chars().next().unwrap();
                    if !c.is_ascii_digit() {
                        break;
                    }
                    v = v * 10 + (c as i32 - '0' as i32);
                    self.pos += 1;
                }
                return Ok(Token { kind: TokenKind::Num, num_value: v });
            }

            return Err(format!("Unexpected character '{}' at position {}", ch, self.pos));
        }
    }
}

// ============================================================
// Parser (recursive descent)
// ============================================================

struct Parser<'a> {
    lexer: Lexer<'a>,
    tok: Token,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self, String> {
        let mut lexer = Lexer::new(input);
        let tok = lexer.next()?;
        Ok(Parser { lexer, tok })
    }

    fn expect(&mut self, k: TokenKind) -> Result<(), String> {
        if self.tok.kind != k {
            return Err(format!(
                "Expected {} but got {}",
                k as i32, self.tok.kind as i32
            ));
        }
        self.tok = self.lexer.next()?;
        Ok(())
    }

    fn expect_close(&mut self, _open_kind: TokenKind) -> Result<(), String> {
        if self.tok.kind != TokenKind::RParen && self.tok.kind != TokenKind::RBrace {
            return Err("Expected closing bracket".to_string());
        }
        self.tok = self.lexer.next()?;
        Ok(())
    }

    // primary → NUM | ω | Ω ('_' primary)? | ψ ('_' primary)? '('|'{' expr ')'|'}'
    // | '('|'{' expr ')'|'}'
    fn parse_primary(&mut self) -> Result<Ast, String> {
        match self.tok.kind {
            TokenKind::Num => {
                let n = self.tok.num_value;
                self.tok = self.lexer.next()?;
                Ok(Ast::Num(n))
            }
            TokenKind::OmegaLower => {
                self.tok = self.lexer.next()?;
                Ok(Ast::W)
            }
            TokenKind::Omega => {
                self.tok = self.lexer.next()?;
                let mut sub = None;
                if self.tok.kind == TokenKind::Subscript {
                    self.tok = self.lexer.next()?;
                    sub = Some(Box::new(self.parse_primary()?));
                }
                Ok(Ast::Omega(sub))
            }
            TokenKind::Psi => {
                self.tok = self.lexer.next()?;
                let mut sub = None;
                if self.tok.kind == TokenKind::Subscript {
                    self.tok = self.lexer.next()?;
                    sub = Some(Box::new(self.parse_primary()?));
                }
                let open_kind = self.tok.kind;
                if open_kind == TokenKind::LBrace {
                    self.tok = self.lexer.next()?;
                } else {
                    self.expect(TokenKind::LParen)?;
                }
                let arg = self.parse_expr()?;
                if open_kind == TokenKind::LBrace {
                    self.expect(TokenKind::RBrace)?;
                } else {
                    self.expect(TokenKind::RParen)?;
                }
                Ok(Ast::Psi(sub, Box::new(arg)))
            }
            TokenKind::LParen | TokenKind::LBrace => {
                let open_kind = self.tok.kind;
                self.tok = self.lexer.next()?;
                let inner = self.parse_expr()?;
                self.expect_close(open_kind)?;
                Ok(inner)
            }
            _ => Err("Unexpected token in primary expression".to_string()),
        }
    }

    // power → primary ( '^' power )?
    fn parse_power(&mut self) -> Result<Ast, String> {
        let base = self.parse_primary()?;
        if self.tok.kind == TokenKind::Pow {
            self.tok = self.lexer.next()?;
            let exp = self.parse_power()?;
            Ok(Ast::Pow(Box::new(base), Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    // term → power ( '×' power )*
    fn parse_term(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_power()?;
        while self.tok.kind == TokenKind::Mul {
            self.tok = self.lexer.next()?;
            let right = self.parse_power()?;
            left = Ast::Mul(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    // expr → term ( '+' term )*
    fn parse_expr(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_term()?;
        while self.tok.kind == TokenKind::Plus {
            self.tok = self.lexer.next()?;
            let right = self.parse_term()?;
            left = Ast::Add(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_full(mut self) -> Result<Ast, String> {
        let result = self.parse_expr()?;
        if self.tok.kind != TokenKind::Eof {
            return Err("Unexpected trailing tokens".to_string());
        }
        Ok(result)
    }
}

pub fn parse_bocf(input: &str) -> Result<Ast, String> {
    let parser = Parser::new(input)?;
    parser.parse_full()
}

// ============================================================
// printAST
// ============================================================

pub fn print_ast(node: &Ast, indent: &str) -> String {
    let join = |label: &str, fields: Vec<(&str, Option<&Ast>)>| -> String {
        let mut s = format!("{}{}", indent, label);
        for (name, child) in fields {
            if let Some(c) = child {
                s += &format!("\n{}{}  ← {}", print_ast(c, &format!("{}  ", indent)), "", name);
            }
        }
        s
    };

    match node {
        Ast::Num(n) => format!("{}num {}", indent, n),
        Ast::W => format!("{}ω", indent),
        Ast::Omega(sub) => match sub {
            Some(s) => join("Ω", vec![("sub", Some(s))]),
            None => format!("{}Ω", indent),
        },
        Ast::Psi(sub, arg) => join("ψ", vec![("sub", sub.as_deref()), ("arg", Some(arg))]),
        Ast::Add(l, r) => join("+", vec![("left", Some(l)), ("right", Some(r))]),
        Ast::Mul(l, r) => join("×", vec![("left", Some(l)), ("right", Some(r))]),
        Ast::Pow(b, e) => join("^", vec![("base", Some(b)), ("exp", Some(e))]),
    }
}

// ============================================================
// evalAST — convert AST to Term
// ============================================================

fn eval_node(node: &Ast) -> Result<Term, String> {
    match node {
        Ast::Num(n) => {
            // n = ψ₀(0) + ψ₀(0) + ... (n times)
            let mut r = zero();
            for _ in 0..*n {
                r = add(&r, &one());
            }
            Ok(r)
        }
        Ast::W => Ok(omega()),
        Ast::Omega(sub) => match sub {
            None => Ok(omega1()),
            Some(s) => {
                let sub = eval_node(s)?;
                Ok(t(sub, zero(), zero()))
            }
        },
        Ast::Psi(sub, arg) => {
            let sub_term = match sub {
                Some(s) => eval_node(s)?,
                None => zero(),
            };
            let arg_term = eval_node(arg)?;
            Ok(t(sub_term, arg_term, zero()))
        }
        Ast::Add(l, r) => {
            let a = eval_node(l)?;
            let b = eval_node(r)?;
            Ok(add(&a, &b))
        }
        Ast::Mul(l, r) => {
            let a = eval_node(l)?;
            let b = eval_node(r)?;
            Ok(mul(&a, &b))
        }
        Ast::Pow(b, e) => {
            let base = eval_node(b)?;
            let exp = eval_node(e)?;
            eval_pow_term(&base, &exp)
        }
    }
}

fn eval_pow_term(base: &Term, exponent: &Term) -> Result<Term, String> {
    if is_zero(exponent) {
        return Ok(one());
    }
    if eq(exponent, &one()) {
        return Ok(base.clone());
    }

    // ω^α: use exp function
    if eq(base, &omega()) {
        return Ok(exp(exponent));
    }

    // (ω^β)^α: every Ω_a is a power of ω
    if !is_zero(base) {
        let bn = base.as_ref().unwrap();
        if is_zero(&bn.c) {
            return Ok(exp(&mul(&log(base), exponent)));
        }
    }

    Err("Exponentiation only supported for exponent of ω".to_string())
}

pub fn eval_ast(ast: &Ast) -> Result<Term, String> {
    eval_node(ast)
}

// ============================================================
// BOCF → BMS conversion
// ============================================================

/// Build a starting BMS matrix from subscript depth.
fn build_bms_for_depth(depth: i32) -> Matrix {
    let depth = depth.max(3);
    let mut m: Matrix = vec![
        vec![0, 0, 0],
        vec![1, 1, 1],
        vec![2, 1, 1],
        vec![3, 1, 0],
    ];
    for _ in 3..depth {
        m.push(vec![1, 1, 1]);
        m.push(vec![2, 1, 1]);
        m.push(vec![3, 1, 0]);
    }
    m
}

/// Format a matrix as a BMS display string.
pub fn matrix_to_bms_str(m: &Matrix) -> String {
    let mut s = String::new();
    for col in m {
        s += "(";
        for (i, v) in col.iter().enumerate() {
            if i > 0 {
                s += ",";
            }
            s += &v.to_string();
        }
        s += ")";
    }
    s
}

/// Try to expand M with the given fs. If the expansion doesn't reduce
/// the ordinal (successor with no non-zero limit row), return an
/// empty matrix as a sentinel.
fn try_expand(m: &Matrix, fs: i32) -> Matrix {
    let l = m.len();
    if l <= 1 {
        return Vec::new(); // can't expand
    }

    // Check if M has a limit row in the last column
    let rows = m[l - 1].len();
    let mut x: i32 = -1;
    while ((x + 1) as usize) < rows && m[l - 1][(x + 1) as usize] > 0 {
        x += 1;
    }

    // No limit row → successor, all fs give the same predecessor
    if x < 0 {
        let mut pred: Matrix = Vec::with_capacity(l - 1);
        for i in 0..l - 1 {
            pred.push(m[i].clone());
        }
        return pred;
    }

    expand_bms(m, fs)
}

/// Convert a BOCF expression string to its BMS matrix representation.
pub fn bocf_to_bms(input: &str, progress: &mut dyn FnMut(&str)) -> Result<String, String> {
    let ast = parse_bocf(input)?;
    let target = eval_ast(&ast)?;

    // Zero ordinal → empty matrix
    if is_zero(&target) {
        return Ok("(empty)".to_string());
    }

    // Normalize to standard/canonical form
    let target = standard_form(&target);

    // BOCF ordinals >= Ω cannot be represented as a finite BMS matrix
    if !lt(&target, &omega1()) {
        return Err(format!("Ordinal is too large for BMS conversion: \"{}\"", input));
    }

    let d = subscript_depth(&target);
    let mut start_depth = (d + 2).max(3);
    let mut m = build_bms_for_depth(start_depth);

    let mut iter = 0i32;

    loop {
        iter += 1;
        let mut ctx = BmsContext::new();
        if iter >= 62 {
        }
        let cur_ord = ctx.bms_to_bocf(&m);
        progress(&iter.to_string());

        if eq(&cur_ord, &target) {
            return Ok(matrix_to_bms_str(&m));
        }
        // curOrd must be > target; if not, try a deeper starting matrix
        if !lt(&target, &cur_ord) {
            start_depth += 2;
            m = build_bms_for_depth(start_depth);
            continue;
        }

        // Try fs=0 first (predecessor / start of FS)
        let m0 = try_expand(&m, 0);
        if iter >= 62 {
        }
        if m0.is_empty() {
            return Err(format!("Cannot expand for \"{}\"", input));
        }

        iter += 1;
        let mut ctx = BmsContext::new();
        let m0_ord = ctx.bms_to_bocf(&m0);
        progress(&iter.to_string());

        if eq(&m0_ord, &target) {
            return Ok(matrix_to_bms_str(&m0));
        }
        if !lt(&m0_ord, &target) {
            // M0 >= target (strictly > since == already checked)
            m = m0;
            continue;
        }

        // M0 < target < M → find smallest fs where Mfs >= target
        let mut m_prev = m0;
        let mut m_upper: Matrix = Vec::new();
        let mut fs_hi: i32 = 1;
        let mut upper_found = false;

        // Linear search for small fs (up to 5), then exponential
        while fs_hi <= 5 {
            m_upper = try_expand(&m, fs_hi);
            iter += 1;
            let mut ctx = BmsContext::new();
            let fs_ord = ctx.bms_to_bocf(&m_upper);
            progress(&iter.to_string());

            if eq(&fs_ord, &target) {
                return Ok(matrix_to_bms_str(&m_upper));
            }
            if !lt(&fs_ord, &target) {
                upper_found = true;
                break;
            }
            m_prev = m_upper.clone();
            fs_hi += 1;
        }

        // Exponential search: double fsHi until an upper bound is found
        if !upper_found {
            loop {
                m_upper = try_expand(&m, fs_hi);
                iter += 1;
                let mut ctx = BmsContext::new();
                let fs_ord = ctx.bms_to_bocf(&m_upper);
                progress(&iter.to_string());

                if eq(&fs_ord, &target) {
                    return Ok(matrix_to_bms_str(&m_upper));
                }
                if !lt(&fs_ord, &target) {
                    upper_found = true;
                    break;
                }
                m_prev = m_upper.clone();
                fs_hi += 1;
                fs_hi *= 2;
            }
        }

        // Column binary search between M_prev and M_upper
        let mut progressed = false;
        let n0 = m_prev.len() as i32;
        let n = m_upper.len() as i32;
        if n - n0 >= 4 {
            let mut lo = n0;
            let mut hi = n;
            while lo < hi {
                let mid = (lo + hi) / 2;
                let mmid: Matrix = m_upper[..mid as usize].to_vec();
                iter += 1;
                let mut ctx = BmsContext::new();
                let mid_ord = ctx.bms_to_bocf(&mmid);
                progress(&iter.to_string());
                if eq(&mid_ord, &target) {
                    return Ok(matrix_to_bms_str(&mmid));
                }
                if !lt(&mid_ord, &target) {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            if lo < n {
                m = m_upper[..lo as usize].to_vec();
                progressed = true;
            }
        }
        if !progressed {
            m = m_upper.clone();
            progressed = true;
        }
    }
}
