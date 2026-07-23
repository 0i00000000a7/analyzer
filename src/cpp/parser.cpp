#include "parser.h"
#include <sstream>

extern "C" void reportBMSProgress(const char *s);

// Error string for parse/eval errors
std::string g_errorMsg;

// ============================================================
// UTF-8 helper
// ============================================================

static bool startsWith(const std::string &s, size_t pos, const std::string &pat) {
  return s.compare(pos, pat.size(), pat) == 0;
}

// UTF-8 byte sequences for Unicode characters
static const std::string UTF8_PSI = "\xCF\x88";     // ψ U+03C8
static const std::string UTF8_OMEGA = "\xCE\xA9";   // Ω U+03A9
static const std::string UTF8_OMEGA_L = "\xCF\x89"; // ω U+03C9
static const std::string UTF8_TIMES = "\xC3\x97";   // × U+00D7

// ============================================================
// Token
// ============================================================

enum class TokenKind { Num, Psi, Omega, OmegaLower, LParen, RParen, LBrace, RBrace, Plus, Mul, Pow, Subscript, Eof };

struct Token {
  TokenKind kind;
  int numValue = 0;
};

// ============================================================
// Lexer
// ============================================================

class Lexer {
  const std::string &src;
  size_t pos = 0;

public:
  std::string error;

  Lexer(const std::string &s) : src(s) {}

  Token next() {
    for (;;) {
      if (pos >= src.size())
        return {TokenKind::Eof};

      char ch = src[pos];

      // whitespace
      if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r') {
        pos++;
        continue;
      }

      // ψ (Unicode) or \psi (LaTeX) or psi (text)
      if (startsWith(src, pos, UTF8_PSI)) {
        pos += 2;
        return {TokenKind::Psi};
      }
      if (startsWith(src, pos, "\\psi")) {
        pos += 4;
        return {TokenKind::Psi};
      }
      if (startsWith(src, pos, "psi")) {
        pos += 3;
        return {TokenKind::Psi};
      }

      // Ω (Unicode) or \Omega (LaTeX) or Omega (text)
      if (startsWith(src, pos, UTF8_OMEGA)) {
        pos += 2;
        return {TokenKind::Omega};
      }
      if (startsWith(src, pos, "\\Omega")) {
        pos += 6;
        return {TokenKind::Omega};
      }
      if (startsWith(src, pos, "Omega")) {
        pos += 5;
        return {TokenKind::Omega};
      }

      // ω (Unicode) or \omega (LaTeX) or omega (text)
      if (startsWith(src, pos, UTF8_OMEGA_L)) {
        pos += 2;
        return {TokenKind::OmegaLower};
      }
      if (startsWith(src, pos, "\\omega")) {
        pos += 6;
        return {TokenKind::OmegaLower};
      }
      if (startsWith(src, pos, "omega")) {
        pos += 5;
        return {TokenKind::OmegaLower};
      }

      // Single-character shortcuts: p → ψ, w → ω, W → Ω
      if (ch == 'p') {
        pos++;
        return {TokenKind::Psi};
      }
      if (ch == 'w') {
        pos++;
        return {TokenKind::OmegaLower};
      }
      if (ch == 'W') {
        pos++;
        return {TokenKind::Omega};
      }

      if (ch == '_') {
        pos++;
        return {TokenKind::Subscript};
      }
      if (ch == '(') {
        pos++;
        return {TokenKind::LParen};
      }
      if (ch == ')') {
        pos++;
        return {TokenKind::RParen};
      }
      if (ch == '{') {
        pos++;
        return {TokenKind::LBrace};
      }
      if (ch == '}') {
        pos++;
        return {TokenKind::RBrace};
      }
      if (ch == '+') {
        pos++;
        return {TokenKind::Plus};
      }
      if (startsWith(src, pos, "**")) {
        pos += 2;
        return {TokenKind::Pow};
      }
      if (startsWith(src, pos, UTF8_TIMES) || ch == '*') {
        if (ch == '*')
          pos++;
        else
          pos += 2;
        return {TokenKind::Mul};
      }
      if (ch == '^') {
        pos++;
        return {TokenKind::Pow};
      }

      if (ch >= '0' && ch <= '9') {
        int v = 0;
        while (pos < src.size() && src[pos] >= '0' && src[pos] <= '9') {
          v = v * 10 + (src[pos] - '0');
          pos++;
        }
        return {TokenKind::Num, v};
      }

      g_errorMsg = std::string("Unexpected character '") + ch + "' at position " + std::to_string(pos);
      return {TokenKind::Eof};
    }
  }
};

// ============================================================
// Parser
// ============================================================

static ASTPtr parseExpr(Lexer &lexer, Token &tok);

static bool expect(Lexer &lexer, Token &tok, TokenKind k) {
  if (tok.kind != k) {
    g_errorMsg = "Expected " + std::to_string((int)k) + " but got " + std::to_string((int)tok.kind);
    return false;
  }
  tok = lexer.next();
  return true;
}

static bool expectClose(Lexer &lexer, Token &tok, TokenKind openKind) {
  if (tok.kind != TokenKind::RParen && tok.kind != TokenKind::RBrace) {
    g_errorMsg = "Expected closing bracket";
    return false;
  }
  tok = lexer.next();
  return true;
}

// primary → NUM | ω | Ω ('_' primary)? | ψ ('_' primary)? '('|'{' expr ')'|'}'
// | '('|'{' expr ')'|'}'
static ASTPtr parsePrimary(Lexer &lexer, Token &tok) {
  if (tok.kind == TokenKind::Num) {
    int n = tok.numValue;
    tok = lexer.next();
    return std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Num, .numValue = n});
  }

  if (tok.kind == TokenKind::OmegaLower) {
    tok = lexer.next();
    return std::make_shared<ASTNode>(ASTNode{.type = ASTNode::W});
  }

  if (tok.kind == TokenKind::Omega) {
    tok = lexer.next();
    ASTPtr sub = nullptr;
    if (tok.kind == TokenKind::Subscript) {
      tok = lexer.next();
      sub = parsePrimary(lexer, tok);
    }
    return std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Omega, .sub = sub});
  }

  if (tok.kind == TokenKind::Psi) {
    tok = lexer.next();
    ASTPtr sub = nullptr;
    if (tok.kind == TokenKind::Subscript) {
      tok = lexer.next();
      sub = parsePrimary(lexer, tok);
    }
    TokenKind openKind = tok.kind;
    if (openKind == TokenKind::LBrace)
      tok = lexer.next();
    else if (!expect(lexer, tok, TokenKind::LParen))
      return nullptr;
    ASTPtr arg = parseExpr(lexer, tok);
    if (openKind == TokenKind::LBrace) {
      if (!expect(lexer, tok, TokenKind::RBrace))
        return nullptr;
    } else if (!expect(lexer, tok, TokenKind::RParen))
      return nullptr;
    return std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Psi, .sub = sub, .arg = arg});
  }

  if (tok.kind == TokenKind::LParen || tok.kind == TokenKind::LBrace) {
    TokenKind openKind = tok.kind;
    tok = lexer.next();
    ASTPtr inner = parseExpr(lexer, tok);
    if (!expectClose(lexer, tok, openKind))
      return nullptr;
    return inner;
  }

  g_errorMsg = "Unexpected token in primary expression";
  return nullptr;
}

// power → primary ( '^' power )?
static ASTPtr parsePower(Lexer &lexer, Token &tok) {
  ASTPtr base = parsePrimary(lexer, tok);
  if (tok.kind == TokenKind::Pow) {
    tok = lexer.next();
    ASTPtr exp = parsePower(lexer, tok);
    return std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Pow, .left = base, .right = exp});
  }
  return base;
}

// term → power ( '×' power )*
static ASTPtr parseTerm(Lexer &lexer, Token &tok) {
  ASTPtr left = parsePower(lexer, tok);
  while (tok.kind == TokenKind::Mul) {
    tok = lexer.next();
    ASTPtr right = parsePower(lexer, tok);
    left = std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Mul, .left = left, .right = right});
  }
  return left;
}

// expr → term ( '+' term )*
static ASTPtr parseExpr(Lexer &lexer, Token &tok) {
  ASTPtr left = parseTerm(lexer, tok);
  while (tok.kind == TokenKind::Plus) {
    tok = lexer.next();
    ASTPtr right = parseTerm(lexer, tok);
    left = std::make_shared<ASTNode>(ASTNode{.type = ASTNode::Add, .left = left, .right = right});
  }
  return left;
}

ASTPtr parseBOCF(const std::string &input) {
  Lexer lexer(input);
  Token tok = lexer.next();
  ASTPtr result = parseExpr(lexer, tok);
  if (tok.kind != TokenKind::Eof) {
    g_errorMsg = "Unexpected trailing tokens";
    return nullptr;
  }
  return result;
}

// ============================================================
// printAST
// ============================================================

std::string printAST(const ASTPtr &node, const std::string &indent) {
  if (!node)
    return indent + "null";

  auto join = [&](const std::string &label, std::initializer_list<std::pair<const char *, ASTPtr>> fields) -> std::string {
    std::string s = indent + label;
    for (auto &[name, child] : fields) {
      if (child) {
        s += "\n" + printAST(child, indent + "  ") + "  \u2190 " + name;
      }
    }
    return s;
  };

  switch (node->type) {
  case ASTNode::Num:
    return indent + "num " + std::to_string(node->numValue);
  case ASTNode::W:
    return indent + "\u03C9";
  case ASTNode::Omega:
    if (node->sub)
      return join("\u03A9", {{"sub", node->sub}});
    else
      return indent + "\u03A9";
  case ASTNode::Psi:
    return join("\u03C8", {{"sub", node->sub}, {"arg", node->arg}});
  case ASTNode::Add:
    return join("+", {{"left", node->left}, {"right", node->right}});
  case ASTNode::Mul:
    return join("\u00D7", {{"left", node->left}, {"right", node->right}});
  case ASTNode::Pow:
    return join("^", {{"base", node->left}, {"exp", node->right}});
  }
  return indent + "?";
}

// ============================================================
// evalAST — convert AST to TermPtr
// ============================================================

// Forward declarations for recursive eval
static TermPtr evalNode(const ASTPtr &node);
static TermPtr evalMulTerm(TermPtr a, TermPtr b);
static TermPtr evalPowTerm(TermPtr base, TermPtr exp);

static TermPtr evalNode(const ASTPtr &node) {
  if (!node)
    return ZERO();

  switch (node->type) {
  case ASTNode::Num: {
    // n = ψ₀(0) + ψ₀(0) + ... (n times)
    int n = node->numValue;
    if (n <= 0)
      return ZERO();
    TermPtr r = ZERO();
    for (int i = 0; i < n; i++)
      r = add(r, ONE());
    return r;
  }
  case ASTNode::W: {
    // ω = ψ₀(1)
    return OMEGA();
  }
  case ASTNode::Omega: {
    // Ω with optional subscript: Ω_n = ψ_{n}(0), Ω = Ω₁ = ψ₁(0)
    if (!node->sub)
      return OMEGA1();
    TermPtr sub = evalNode(node->sub);
    return T(sub, ZERO());
  }
  case ASTNode::Psi: {
    // ψ_n(α) = ψ_{n}(α) = T(sub, arg)
    TermPtr sub = node->sub ? evalNode(node->sub) : ZERO();
    TermPtr arg = evalNode(node->arg);
    return T(sub, arg);
  }
  case ASTNode::Add: {
    // α + β
    return add(evalNode(node->left), evalNode(node->right));
  }
  case ASTNode::Mul: {
    // α × β
    return evalMulTerm(evalNode(node->left), evalNode(node->right));
  }
  case ASTNode::Pow: {
    // α ^ β
    return evalPowTerm(evalNode(node->left), evalNode(node->right));
  }
  }
  return ZERO();
}

// Ordinal multiplication
static TermPtr evalMulTerm(TermPtr a, TermPtr b) { return mul(a, b); }

// Ω_a^b = exp(ψ_a(0) × b)
static TermPtr powOmega(TermPtr a, TermPtr b) {
  if (isZero(b))
    return ONE();
  if (isZero(a))
    return exp(b);                          // Ω₀ = ψ₀(0) = 1, 1^b = ω^b
  return exp(mul(T(a, ZERO(), ZERO()), b)); // Ω_a^b = exp(Ω_a × b)
}

// Ordinal exponentiation — supports ω^α and Ω_a^β
static TermPtr evalPowTerm(TermPtr base, TermPtr exponent) {
  if (isZero(exponent))
    return ONE();
  if (eq(exponent, ONE()))
    return base;

  // ω^α: use exp function
  if (eq(base, OMEGA())) {
    return exp(exponent);
  }

  // Ω_a^b: for base of the form ψ_a(0)
  if (!isZero(base) && isZero(base->b) && isZero(base->c)) {
    return powOmega(base->a, exponent);
  }

  g_errorMsg = "Exponentiation only supported for ω and Ω_a bases";
  return ZERO();
}

TermPtr evalAST(const ASTPtr &node) { return evalNode(node); }

// ============================================================
// BOCF → BMS conversion
// ============================================================

/// Build a starting BMS matrix from subscript depth.
/// Depth 3 → ψ(Ω_Ω) = (0,0,0)(1,1,1)(2,1,1)(3,1,0)
/// Each extra depth appends (1,1,1)(2,1,1)(3,1,0).
static Matrix buildBMSForDepth(int depth) {
  if (depth < 3)
    depth = 3;
  Matrix M = {{0, 0, 0}, {1, 1, 1}, {2, 1, 1}, {3, 1, 0}};
  for (int i = 3; i < depth; i++) {
    M.push_back({1, 1, 1});
    M.push_back({2, 1, 1});
    M.push_back({3, 1, 0});
  }
  return M;
}

/// Format a matrix as a BMS display string.
static std::string matrixToBMSStr(const Matrix &M) {
  std::string s;
  for (const auto &col : M) {
    s += "(";
    for (size_t i = 0; i < col.size(); i++) {
      if (i > 0)
        s += ",";
      s += std::to_string(col[i]);
    }
    s += ")";
  }
  return s;
}

/// Try to expand M with the given fs. If the expansion doesn't reduce
/// the ordinal (i.e. successor with no non-zero limit row), return an
/// empty matrix as a sentinel.
static Matrix tryExpand(const Matrix &M, int fs) {
  int l = (int)M.size();
  if (l <= 1)
    return {}; // can't expand

  // Check if M has a limit row in the last column (same logic as expandBMS)
  int rows = (int)M.back().size();
  int x = -1;
  while (x + 1 < rows && M[l - 1][x + 1] > 0)
    x++;

  // No limit row → successor, all fs give the same predecessor
  if (x < 0) {
    Matrix pred;
    for (int i = 0; i < l - 1; i++)
      pred.push_back(M[i]);
    return pred;
  }

  return expandBMS(M, fs);
}

/// Convert a BOCF expression string to its BMS matrix representation.
std::string bocfToBMS(const std::string &input) {
  g_errorMsg.clear();
  ASTPtr ast = parseBOCF(input);
  if (!g_errorMsg.empty())
    return "!" + g_errorMsg;
  TermPtr target = evalAST(ast);
  if (!g_errorMsg.empty())
    return "!" + g_errorMsg;

  // Zero ordinal → empty matrix
  if (isZero(target)) {
    return "(empty)";
  }

  // Normalize to standard/canonical form for correct ordinal comparison
  target = standardForm(target);

  // BOCF ordinals >= Ω cannot be represented as a finite BMS matrix
  if (!lt(target, OMEGA1())) {
    return "!Ordinal is too large for BMS conversion: \"" + input + "\"";
  }

  int d = subscriptDepth(target);
  int startDepth = std::max(3, d + 2);
  Matrix M = buildBMSForDepth(startDepth);

  int iter = 0;

  while (true) {
    iter++;
    TermPtr curOrd = BMSToBocf(M);

    // Report progress every iteration
    reportBMSProgress(std::to_string(iter).c_str());

    if (eq(curOrd, target)) {
      return matrixToBMSStr(M);
    }

    // curOrd must be > target; if not, try a deeper starting matrix
    if (!lt(target, curOrd)) {
      startDepth += 2;
      M = buildBMSForDepth(startDepth);
      continue;
    }

    // Try fs=0 first (predecessor / start of FS)
    Matrix M0 = tryExpand(M, 0);
    if (M0.empty()) {
      return "!Cannot expand for \"" + input + "\"";
    }

    iter++;
    TermPtr m0Ord = BMSToBocf(M0);
    reportBMSProgress(std::to_string(iter).c_str());

    if (eq(m0Ord, target)) {
      return matrixToBMSStr(M0);
    }

    if (!lt(m0Ord, target)) {
      // M0 >= target  (strictly > since == already checked)
      M = M0;
      continue;
    }

    // M0 < target < M  →  find smallest fs where Mfs >= target
    bool progressed = false;
    Matrix M_prev = M0; // largest known M[fs] < target
    Matrix M_upper;     // smallest known M[fs] >= target (once found)
    int fsLo = 1, fsHi = 1;

    // Linear search for small fs (up to 5), then exponential
    while (fsHi <= 5) {
      M_upper = tryExpand(M, fsHi);
      iter++;
      TermPtr fsOrd = BMSToBocf(M_upper);
      reportBMSProgress(std::to_string(iter).c_str());

      if (eq(fsOrd, target))
        return matrixToBMSStr(M_upper);
      if (!lt(fsOrd, target))
        goto colSearch;

      M_prev = M_upper;
      fsLo = ++fsHi;
    }

    // Exponential search: double fsHi until an upper bound is found
    while (true) {
      M_upper = tryExpand(M, fsHi);
      iter++;
      TermPtr fsOrd = BMSToBocf(M_upper);
      reportBMSProgress(std::to_string(iter).c_str());

      if (eq(fsOrd, target))
        return matrixToBMSStr(M_upper);
      if (!lt(fsOrd, target))
        goto colSearch;

      M_prev = M_upper;
      fsLo = ++fsHi;
      fsHi *= 2;
    }

    if (M_upper.empty()) {
      if (!progressed) {
        return "!Cannot find BMS representation for \"" + input + "\"";
      }
      continue;
    }

    {
    colSearch:
      // Column binary search between M_prev and M_upper
      int n0 = (int)M_prev.size();
      int n = (int)M_upper.size();
      if (n - n0 >= 4) {
        int lo = n0, hi = n;
        while (lo < hi) {
          int mid = (lo + hi) / 2;
          Matrix Mmid(M_upper.begin(), M_upper.begin() + mid);
          iter++;
          TermPtr midOrd = BMSToBocf(Mmid);
          reportBMSProgress(std::to_string(iter).c_str());
          if (eq(midOrd, target)) {
            return matrixToBMSStr(Mmid);
          }
          if (!lt(midOrd, target)) {
            hi = mid;
          } else {
            lo = mid + 1;
          }
        }
        if (lo < n) {
          M = Matrix(M_upper.begin(), M_upper.begin() + lo);
          progressed = true;
        }
      }
      if (!progressed) {
        M = M_upper;
        progressed = true;
      }
    }

    if (!progressed) {
      return "!Cannot find BMS representation for \"" + input + "\"";
    }
  }
}