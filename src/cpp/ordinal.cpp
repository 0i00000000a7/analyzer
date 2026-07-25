#include "ordinal.h"
#include <algorithm>
#include <cstdio>
#include <map>
#include <sstream>
#include <vector>

// ── Detailed profiler ──
// (removed)

// ============================================================
// Structural helpers
// ============================================================

bool isOrdinalFinite(TermPtr a) { return isZero(a) || (isZero(a->a) && isZero(a->b)); }

int length1(TermPtr a) { return isZero(a) ? 0 : 1 + length1(a->c); }

bool eq(TermPtr a, TermPtr b) {
  if (isZero(a) || isZero(b)) {
    return isZero(a) == isZero(b);
  }
  return eq(a->a, b->a) && eq(a->b, b->b) && eq(a->c, b->c);
}

bool lt(TermPtr a, TermPtr b) {
  if (isZero(b))
    return false;
  if (isZero(a))
    return true;
  if (!eq(a->a, b->a))
    return lt(a->a, b->a);
  if (!eq(a->b, b->b))
    return lt(a->b, b->b);
  return lt(a->c, b->c);
}

TermPtr firstTerm(TermPtr a) {
  if (isZero(a))
    return ZERO();
  return T(a->a, a->b, ZERO());
}

TermPtr lastTerm(TermPtr a) {
  if (isZero(a))
    return ZERO();
  if (isZero(a->c))
    return a;
  return lastTerm(a->c);
}

// ============================================================
// Arithmetic
// ============================================================

/// Natural number n as an ordinal
static TermPtr fromInt(int n) {
  TermPtr r = ZERO();
  for (int i = 0; i < n; i++)
    r = add(r, ONE());
  return r;
}

bool isSucc(TermPtr a) {
  if (isZero(a))
    return false;
  TermPtr last = lastTerm(a);
  // last term is ψ₀(0) = 1 (i.e., α = X + 1)
  return isZero(last->a) && isZero(last->b);
}

TermPtr pred(TermPtr a) {
  if (!isSucc(a))
    return a;
  // Rebuild sum without the last term (= ψ₀(0))
  TermPtr r = ZERO();
  TermPtr cur = a;
  while (!isZero(cur)) {
    if (isZero(cur->c))
      break; // last term — skip
    r = add(r, T(cur->a, cur->b, ZERO()));
    cur = cur->c;
  }
  return r;
}

TermPtr add(TermPtr a, TermPtr b) {
  if (isZero(a))
    return b;
  if (isZero(b))
    return a;
  if (lt(firstTerm(a), firstTerm(b))) {
    return b;
  }
  return T(a->a, a->b, add(a->c, b));
}

TermPtr sub(TermPtr a, TermPtr b) {
  if (isZero(a))
    return ZERO();
  if (isZero(b))
    return a;
  if (gt(firstTerm(a), firstTerm(b))) {
    return a;
  }
  return sub(a->c, b->c);
}

std::pair<TermPtr, TermPtr> separate(TermPtr a, TermPtr b) {
  if (isZero(a))
    return {ZERO(), ZERO()};
  if (lt(firstTerm(a), b))
    return {ZERO(), a};
  auto [s0, s1] = separate(a->c, b);
  return {T(a->a, a->b, s0), s1};
}

TermPtr truncate(TermPtr a, TermPtr b) {
  if (isZero(a))
    return ZERO();
  if (isZero(truncate(a->c, b)) && lt(firstTerm(a), T(b, ZERO(), ZERO()))) {
    return ZERO();
  }
  return T(a->a, a->b, truncate(a->c, b));
}

TermPtr exp(TermPtr a) {
  if (lt(a, EPSILON0())) {
    return T(ZERO(), a, ZERO());
  }
  auto [p, _rest] = separate(a->b, T(succ(a->a), ZERO(), ZERO()));
  (void)_rest;
  return T(a->a, add(p, sub(a, T(a->a, p, ZERO()))), ZERO());
}

TermPtr log(TermPtr a) {
  if (isZero(a))
    return ZERO();
  auto [p, q] = separate(a->b, T(succ(a->a), ZERO(), ZERO()));
  if (isZero(a->a) && isZero(p)) {
    if (!lt(a->b, EPSILON0())) {
      if (eq(log(q), q) && isZero(q->c) && lt(a->b, OMEGA1())) {
        return firstTerm(a);
      }
    }
    return q;
  }
  TermPtr m = T(a->a, p, q); // m = ψ_a(p) + q
  if (!lt(a->b, T(a->a, T(succ(a->a), ZERO(), ZERO()), ZERO()))) {
    if (eq(log(a->b), a->b) && isZero(a->c) && lt(a->b, T(succ(a->a), ZERO(), ZERO()))) {
      return firstTerm(a);
    }
  }
  return m;
}

// ============================================================
// Subscript depth
// ============================================================

int subscriptDepth(TermPtr t) {
  if (isZero(t))
    return 0;
  int da = subscriptDepth(t->a) + 1;
  int db = subscriptDepth(t->b);
  int dc = subscriptDepth(t->c);
  return std::max({da, db, dc});
}

// ============================================================
// Multiplication
// ============================================================

std::vector<TermPtr> everyTerms(TermPtr a) {
  std::vector<TermPtr> terms;
  while (!isZero(a)) {
    terms.push_back(firstTerm(a));
    a = a->c;
  }
  return terms;
}

/// Multiply a by a finite ordinal b via iterated addition
TermPtr mulFinite(TermPtr a, TermPtr b) {
  if (isZero(b))
    return ZERO();
  return add(a, mulFinite(a, b->c));
}

/// General ordinal multiplication
TermPtr mul(TermPtr a, TermPtr b) {
  if (isZero(a) || isZero(b))
    return ZERO();
  if (lt(firstTerm(log(a)), firstTerm(log(b))))
    return b;

  auto [c, d] = separate(b, OMEGA());
  TermPtr logA = log(a);
  auto terms = everyTerms(c);

  TermPtr result = mulFinite(a, d);
  for (auto it = terms.rbegin(); it != terms.rend(); ++it) {
    TermPtr term = exp(add(logA, log(*it)));
    result = T(term->a, term->b, result);
  }

  return result;
}

// ============================================================
// Standard form
// ============================================================

TermPtr mergePsiAddends(TermPtr a, TermPtr b, TermPtr c) {
  if (isZero(c)) {
    return T(a, b, ZERO());
  }
  // c = ψ_d(t+h) + f, where all terms of t are >= ψ_{d+1}(0)
  if (lt(b, c->b) && gt(c, T(a, ZERO(), ZERO()))) {
    TermPtr t = truncate(c->b, succ(c->a));
    return mergePsiAddends(a, add(t, sub(firstTerm(c), T(c->a, t, ZERO()))), c->c);
  }
  return mergePsiAddends(a, add(b, firstTerm(c)), c->c);
}

TermPtr standardForm(TermPtr a) {
  if (isZero(a))
    return ZERO();
  return add(mergePsiAddends(standardForm(a->a), ZERO(), standardForm(a->b)), standardForm(a->c));
}

// ============================================================
static bool isFiniteNat(TermPtr t) {
  if (isZero(t))
    return true;
  if (!isZero(t->a) || !isZero(t->b))
    return false;
  return isFiniteNat(t->c);
}

// Fundamental sequences for Extended Buchholz's ψ
// ============================================================

/// Cofinality (returns 0, 1, ω (=ψ₀(1)), or a regular cardinal Ωα)
TermPtr cofinality(TermPtr a) {
  if (isZero(a))
    return ZERO();
  TermPtr last = lastTerm(a);
  if (!eq(last, a))
    return cofinality(last);

  TermPtr beta = a->a, gamma = a->b;
  if (isZero(gamma)) {
    if (isZero(beta))
      return ONE(); // ψ₀(0) → 1
    TermPtr cfBeta = cofinality(beta);
    if (eq(cfBeta, ONE()))
      return a;    // succ subscript → regular
    return cfBeta; // limit subscript → cof(subscript)
  }
  TermPtr cfGamma = cofinality(gamma);
  if (eq(cfGamma, ONE()))
    return OMEGA(); // succ argument → ω
  // Compare Cof(γ) with the subscript β (JS: compare(cf_a, v) ≤ 0)
  if (!lt(beta, cfGamma))
    return cfGamma; // β ≥ Cof(γ) → Cof(γ) ≤ β
  return OMEGA();   // β < Cof(γ) → ω
}

static TermPtr sumWithoutLast(TermPtr a) {
  if (isZero(a))
    return ZERO();
  TermPtr r = ZERO(), cur = a;
  while (true) {
    if (isZero(cur))
      break;
    if (isZero(cur->c))
      break;
    r = add(r, T(cur->a, cur->b, ZERO()));
    cur = cur->c;
  }
  return r;
}

/// Fundamental sequence α[n] for integer index n (JS BOCF_EBO.ts logic)
TermPtr fundamentalSequence(TermPtr a, int n) {
  if (isZero(a))
    return ZERO();

  // Sum: α[n] = first + last[n]
  TermPtr last = lastTerm(a);
  if (!eq(last, a))
    return add(sumWithoutLast(a), fundamentalSequence(last, n));

  TermPtr beta = a->a, gamma = a->b;

  if (isZero(gamma)) {
    // ψβ(0)
    if (isZero(beta))
      return ZERO(); // ψ₀(0)[n] = 0 (rule 3)
    TermPtr cfBeta = cofinality(beta);
    if (eq(cfBeta, ONE()))
      return fromInt(n);                                    // succ β → n (rule 4)
    return T(fundamentalSequence(beta, n), ZERO(), ZERO()); // limit β → ψβ[n](0) (rule 5)
  }

  TermPtr cfGamma = cofinality(gamma);

  if (eq(cfGamma, ONE())) {
    // Cof(γ) = 1 → succ argument (rule 6): ψβ(γ[0]) repeated n times
    TermPtr gamma0 = fundamentalSequence(gamma, 0);
    return mulFinite(T(beta, gamma0, ZERO()), fromInt(n));
  }

  // Compare Cof(γ) with β (JS: compare(cf_a, v) ≤ 0 → rule 07)
  if (!lt(beta, cfGamma)) {
    // β ≥ Cof(γ) → Cof(γ) ≤ β (rule 7): ψβ(γ[n])
    return T(beta, fundamentalSequence(gamma, n), ZERO());
  }

  // β < Cof(γ) (rule 8): iterate Re with JS-style n-iteration loop.
  // For successor cardinals Ω_{δ+1}, extract δ and use ψ_δ in the loop.
  if (!isZero(cfGamma) && !isZero(cfGamma->a) && isSucc(cfGamma->a)) {
    TermPtr delta = sub(cfGamma->a, ONE()); // δ = pred(cofinality subscript)
    TermPtr re = ZERO();
    for (int i = 0; i < n; i++)
      re = fundamentalSequence(gamma, T(delta, re, ZERO()));
    return T(beta, re, ZERO());
  }
  // Non-successor-cardinal cofinalities: fallback to ψβ(γ[n])
  return T(beta, fundamentalSequence(gamma, n), ZERO());
}

/// Fundamental sequence α[β] for ordinal index β (used internally by rule 8)
TermPtr fundamentalSequence(TermPtr a, TermPtr index) {
  TermPtr last = lastTerm(a);
  if (!eq(last, a))
    return add(sumWithoutLast(a), fundamentalSequence(last, index));

  if (isFiniteNat(index))
    return fundamentalSequence(a, length1(index));

  TermPtr beta = a->a, gamma = a->b;

  if (isZero(gamma)) {
    if (isZero(beta))
      return ZERO();
    TermPtr cfBeta = cofinality(beta);
    if (eq(cfBeta, ONE())) {
      if (lt(index, a))
        return index;
      return ZERO();
    }
    return T(fundamentalSequence(beta, index), ZERO(), ZERO());
  }

  TermPtr cfGamma = cofinality(gamma);
  if (eq(cfGamma, ONE()))
    return fundamentalSequence(a, length1(index));

  if (!lt(beta, cfGamma))
    return T(beta, fundamentalSequence(gamma, index), ZERO());

  return fundamentalSequence(a, length1(index));
}

// ============================================================
// EBO check
// ============================================================

bool isEqEBO(const Matrix &M) { return matrixLexOrder(M, EBO) == 0; }

bool isGtEBO(const Matrix &M) { return matrixLexOrder(M, EBO) > 0; }

// ============================================================
// String conversion (LaTeX)
// ============================================================

static std::string renderTerm(TermPtr q);

static std::string omegaStr(TermPtr a) {
  if (isZero(a))
    return "\\omega";
  if (eq(a, ONE()))
    return "\\Omega";
  return "\\Omega_{" + renderTerm(a) + "}";
}

// Decompose ψ_a(b) into Ω_a^{first} * second (JS decomposePower() function)
static std::pair<TermPtr, TermPtr> decomposePower(TermPtr a) {
  if (isZero(a))
    return {ZERO(), ZERO()};
  if (isZero(a->a))
    return {log(a), ZERO()};
  auto [p, s] = separate(a->b, T(succ(a->a), ZERO(), ZERO()));
  auto [q, r] = separate(s, T(a->a, ZERO(), ZERO()));
  TermPtr second = exp(r);
  TermPtr first = add(ONE(), p);
  TermPtr ptr = q;
  while (!isZero(ptr)) {
    first = add(first, exp(sub(log(ptr), T(a->a, ZERO(), ZERO()))));
    ptr = ptr->c;
  }
  return {first, second};
}

// ── renderTerm cache (deep structural equality via recursive comparison) ──
struct TermPtrLess {
  bool operator()(TermPtr a, TermPtr b) const {
    if (a == b)
      return false;
    if (!a)
      return true;
    if (!b)
      return false;
    if (operator()(a->a, b->a))
      return true;
    if (operator()(b->a, a->a))
      return false;
    if (operator()(a->b, b->b))
      return true;
    if (operator()(b->b, a->b))
      return false;
    return operator()(a->c, b->c);
  }
};
static std::map<TermPtr, std::string, TermPtrLess> g_renderCache;

static std::string renderTerm(TermPtr q) {
  if (isZero(q))
    return "0";
  if (isOrdinalFinite(q))
    return std::to_string(length1(q));

  // Memoize by term pointer identity (terms are immutable)
  auto it = g_renderCache.find(q);
  if (it != g_renderCache.end())
    return it->second;

  auto [aPart, bPart] = separate(q, firstTerm(q));
  TermPtr a0 = aPart->a, a1 = aPart->b;

  std::string m = "\\psi_{" + renderTerm(a0) + "}\\left(" + renderTerm(a1) + "\\right)";

  if (isZero(a1)) {
    m = omegaStr(a0);
  }
  if (isZero(a0)) {
    m = "\\psi\\left(" + renderTerm(a1) + "\\right)";
  }
  if (isZero(a0) && eq(a1, ONE())) {
    m = "\\omega";
  } else if (lt(a1, T(succ(a0), ZERO(), ZERO())) && !eq(a1, T(succ(a0), ZERO(), ZERO()))) {
    auto [first, second] = decomposePower(aPart);
    // Fixed point check: if first equals the whole term, skip ω-power rendering
    if (eq(first, aPart)) {
      // fall through to default ψ rendering (m already set above)
    } else {
      m = omegaStr(a0);
      if (gt(first, ONE())) {
        m += "^{" + renderTerm(first) + "}";
      }
      if (gt(second, ONE())) {
        m += renderTerm(second);
      }
      int len = length1(aPart);
      if (len > 1)
        m += std::to_string(len);
      if (!isZero(bPart)) {
        m += "+" + renderTerm(bPart);
      }
      g_renderCache[q] = m;
      return m;
    }
  }

  int len = length1(aPart);
  if (len > 1)
    m += std::to_string(len);

  if (!isZero(bPart)) {
    m += "+" + renderTerm(bPart);
  }
  g_renderCache[q] = m;
  return m;
}

// ============================================================
// Extended Veblen conversion (arXiv-2310.12832v2)
// Implements ψ₀(α) = φ V(t(α)) with @ notation
// ============================================================

static bool isBelowBHO(TermPtr q) { return lt(q, BHO()); }

// ----------------------------------------------------------
// s(α): set of all exponents and coeffs in base-Ω CNF (k-function helper)
// Returns true iff every member of s(α) is φ_k(beta) = -1
// ----------------------------------------------------------
static bool sAllKMinusOne(TermPtr alpha, TermPtr beta);

// ----------------------------------------------------------
// k(α, β) from the paper:
//   -1 if α < β (when α<Ω) or "all s(α) members k(·,β) = -1"
//    0 if α = β (when α<Ω) or special conditions
//    1 if α > β (when α<Ω) or otherwise
// ----------------------------------------------------------
static int kFunction(TermPtr alpha, TermPtr beta) {
  // α < Ω case
  if (lt(alpha, OMEGA1())) {
    if (lt(alpha, beta))
      return -1;
    if (eq(alpha, beta))
      return 0;
    return 1; // α > β
  }

  // α ≥ Ω: decompose α = ξ + Ω^γ·δ
  // Find smallest Ω-exponent
  // First collect all Ω-power terms
  std::vector<std::pair<TermPtr, TermPtr>> terms; // (exponent, coeff)
  TermPtr tail = ZERO();
  {
    TermPtr curr = alpha;
    while (!isZero(curr)) {
      TermPtr head = firstTerm(curr);
      if (!isZero(head->a)) {
        auto [exp, second] = decomposePower(head);
        terms.push_back({exp, second});
      } else {
        tail = add(tail, head);
      }
      curr = curr->c;
    }
  }

  // If there's a tail (position-0 term), γ = 0, δ = tail
  // Otherwise, γ = last term's exponent, δ = last term's coefficient
  TermPtr gamma, delta;
  TermPtr xi; // ξ = everything except the Ω^γ·δ term

  if (!isZero(tail)) {
    // γ = 0, δ = tail
    gamma = ZERO();
    delta = tail;
    // ξ = all Ω-power terms (reconstruct)
    xi = ZERO();
    for (size_t i = 0; i < terms.size(); i++) {
      TermPtr t = T(ONE(), terms[i].first, ZERO());
      if (!eq(terms[i].second, ONE()) && !isZero(terms[i].second)) {
        // coefficient is not 1, need to multiply... for k function,
        // coefficient doesn't matter, just the terms
      }
      // For k function, we only need the exponent structure,
      // so reconstruct ξ from Ω-power terms
      if (!eq(terms[i].second, ONE())) {
        // coefficient added as repeated terms
        // For the k function, s(ξ) includes exponent and coeff of each term
        // The coefficient is the SECOND part from decomposePower
        // Actually s only needs the base-Ω CNF exponents and coefficients
        // which are the decomposePower results
      }
      // For the k function, we just need Xi reconstructed
      if (!isZero(t)) {
        for (int c = 0; c < 1; c++) { // simplified: just the term itself
          xi = add(xi, t);
        }
      }
    }
  } else if (!terms.empty()) {
    // γ = last (smallest) term's exponent
    gamma = terms.back().first;
    delta = terms.back().second;
    // ξ = all terms except the last
    xi = ZERO();
    // Also add the tail (already zero)
    // The Ω-powers except the smallest one
    for (size_t i = 0; i + 1 < terms.size(); i++) {
      TermPtr t = T(ONE(), terms[i].first, ZERO());
      if (!eq(terms[i].second, ONE()) && !isZero(terms[i].second)) {
        // For k-function, we consider the structure
        t = t; // keep as is for now
      }
      xi = add(xi, t);
    }
  } else {
    // α = 0 (shouldn't reach here)
    if (eq(alpha, beta))
      return 0;
    return lt(alpha, beta) ? -1 : 1;
  }

  // Check: for all ρ ∈ s(α), k(ρ, β) = -1?
  bool allMinusOne = true;

  // Check every term's exponent and coefficient
  for (auto &[exp, coeff] : terms) {
    if (kFunction(exp, beta) != -1) {
      allMinusOne = false;
      break;
    }
    if (kFunction(coeff, beta) != -1) {
      allMinusOne = false;
      break;
    }
  }
  if (!isZero(tail)) {
    if (kFunction(tail, beta) != -1) {
      allMinusOne = false;
    }
  }

  if (allMinusOne)
    return -1;

  // Check k = 0 condition:
  // for all ρ ∈ s(ξ), k(ρ, β) = -1
  bool xiAllMinusOne = true;
  // When there's a tail, ξ includes ALL Ω-power terms (γ=0 from the tail).
  // When there's no tail, ξ excludes the smallest Ω-term (which is γ).
  size_t xiEnd = isZero(tail) ? (terms.empty() ? 0 : terms.size() - 1) : terms.size();
  for (size_t i = 0; i < xiEnd; i++) {
    auto &[exp, coeff] = terms[i];
    if (kFunction(exp, beta) != -1) {
      xiAllMinusOne = false;
      break;
    }
    if (kFunction(coeff, beta) != -1) {
      xiAllMinusOne = false;
      break;
    }
  }
  if (xiAllMinusOne) {
    if (eq(gamma, beta) && eq(delta, ONE()))
      return 0;
    if (kFunction(gamma, beta) == -1 && eq(delta, beta))
      return 0;
  }

  return 1;
}

// ----------------------------------------------------------
// Compute λ = ψ₀(ξ) − 1
// For ξ = 0: ψ₀(0) = 1, so 1-1 = 0 → ZERO
// For ξ > 0: ψ₀(ξ) is a limit, so ψ₀(ξ) − 1 = ψ₀(ξ)
// ----------------------------------------------------------
static TermPtr psi0MinusOne(TermPtr xi) {
  if (isZero(xi))
    return ZERO();
  return T(ZERO(), xi, ZERO()); // ψ₀(ξ)
}

// ----------------------------------------------------------
// Multiply Ω · beta
// For beta < Ω: Ω·beta = ψ₁(beta_cnf_components)
// For beta >= Ω: Ω·beta increments each ψ₁(δ) term to ψ₁(δ+Ω)
// ----------------------------------------------------------
static TermPtr omegaTimes(TermPtr beta) {
  if (isZero(beta))
    return ZERO();
  if (isOrdinalFinite(beta)) {
    // β is a finite natural number
    int n = 0;
    TermPtr cur = beta;
    while (!isZero(cur)) {
      n++;
      cur = cur->c;
    }
    TermPtr result = ZERO();
    for (int i = 0; i < n; i++) {
      result = add(result, T(ONE(), ZERO(), ZERO()));
    }
    return result;
  }
  // β in CNF: sum of ψ₀(a)·c + ψ₁(δ)·c + ...
  // Ω·β = Ω·(ψ₀ part + ψ₁ part)
  //   Ω·ψ₀(a) = ψ₁(a)
  //   Ω·ψ₁(δ) = ψ₁(δ+Ω)
  TermPtr result = ZERO();
  TermPtr curr = beta;
  while (!isZero(curr)) {
    TermPtr head = firstTerm(curr);
    if (isZero(head->a)) {
      // ψ₀ term: head = ψ₀(a). Ω·ψ₀(a) = ψ₁(log(ψ₀(a)))
      // Using log(head) gives the ω-exponent, which handles cases
      // where ψ₀(a) ≠ a (e.g., ψ₀(Ω²) = ζ₀, log(ζ₀) = ζ₀)
      result = add(result, T(ONE(), log(head), ZERO()));
    } else {
      // ψ₁ term: head = ψ₁(δ) where δ = head->b
      // Ω·ψ₁(δ) = ψ₁(Ω+δ), NOT ψ₁(δ+Ω)
      // Addition is non-commutative: δ+Ω = Ω for δ<Ω, but Ω+δ preserves δ
      result = add(result, T(ONE(), add(OMEGA1(), head->b), ZERO()));
    }
    curr = curr->c;
  }
  return result;
}

// ----------------------------------------------------------
// Decompose ψ₀'s argument into array of (exponent, coeff) pairs + tail
// ----------------------------------------------------------
struct OmegaCnfTerm {
  TermPtr exponent; // β in Ω^β (from decomposePower)
  TermPtr second;   // ω^δ part from decomposePower
  int count;        // multiplicity
};

static void decomposeOmegaCnf(TermPtr alpha, std::vector<OmegaCnfTerm> &terms, TermPtr &tail) {
  TermPtr curr = alpha;
  while (!isZero(curr)) {
    TermPtr head = firstTerm(curr);

    if (!isZero(head->a)) {
      auto [exp, second] = decomposePower(head);

      if (!terms.empty() && eq(terms.back().exponent, exp) && eq(terms.back().second, second)) {
        terms.back().count++;
      } else {
        terms.push_back({exp, second, 1});
      }
    } else {
      tail = add(tail, head);
    }
    curr = curr->c;
  }
}

// ----------------------------------------------------------
// t(α) — the t-function from the paper
// ψ₀(α) = φ V(t(α))
// ----------------------------------------------------------
static TermPtr computeT(TermPtr alpha) {
  if (isZero(alpha))
    return ZERO();

  // Decompose α into base-Ω CNF to find smallest exponent
  std::vector<OmegaCnfTerm> terms;
  TermPtr tail = ZERO();
  decomposeOmegaCnf(alpha, terms, tail);

  // Determine β, γ, ξ (smallest Ω-exponent, coeff, upper part)
  TermPtr beta, gamma, xi;

  if (!isZero(tail)) {
    // Has a tail (< Ω term): smallest exponent is 0
    // α = (Ω-powers) + tail
    // Decompose as ξ + Ω^0·γ where γ = tail
    beta = ZERO();
    gamma = tail;
    // ξ = sum of all Ω-power terms — collect directly from original α
    xi = ZERO();
    {
      TermPtr cur = alpha;
      while (!isZero(cur)) {
        TermPtr head = firstTerm(cur);
        if (!isZero(head->a)) {
          xi = add(xi, head);
        }
        cur = cur->c;
      }
    }
  } else if (!terms.empty()) {
    // No tail. Smallest exponent = last term's exponent
    beta = terms.back().exponent;

    // γ = total coefficient at exponent β (sum ALL terms with exp = β, forward
    // CNF order)
    gamma = ZERO();
    {
      // Find first term with exponent = β, then accumulate forward
      // (largest-second first)
      int startIdx = (int)terms.size() - 1;
      while (startIdx >= 0 && eq(terms[startIdx].exponent, beta))
        startIdx--;
      startIdx++; // first index with exp = β
      for (int i = startIdx; i < (int)terms.size(); i++) {
        for (int c = 0; c < terms[i].count; c++) {
          gamma = add(gamma, terms[i].second);
        }
      }
    }

    // ξ = Ω-power terms with Ω-exponent > β
    // Use decomposePower to get each term's Ω-exponent
    xi = ZERO();
    {
      TermPtr cur = alpha;
      while (!isZero(cur)) {
        TermPtr head = firstTerm(cur);
        if (!isZero(head->a)) {
          auto [exp, _second] = decomposePower(head);
          if (gt(exp, beta)) {
            xi = add(xi, head);
          }
        }
        cur = cur->c;
      }
    }
  } else {
    // Only tail (< Ω), no Ω-powers
    // α = δ where δ < Ω
    // Decompose as ξ + Ω^β·γ with β=0?
    // α = 0 + Ω^0·α (but 0 < α < Ω, so β=0, γ=α, ξ=0)
    // Actually: α_ = ξ + Ω^β·γ where ξ = Ω^{β+1}η
    // If α < Ω: β = 0, γ = α, ξ = 0
    // Wait, Ω^0·α = 1·α = α, and ξ = 0 = Ω^1·0. ✓
    return alpha; // For α < Ω, t(α) = α (since simpler)
  }

  // λ = ψ₀(ξ) - 1
  TermPtr lambda = psi0MinusOne(xi);

  // u = k(β, λ)
  int u = kFunction(beta, lambda);

  // ρ = λ if u = -1, 1 if u = 0, 0 if u = 1
  TermPtr rho;
  if (u == -1)
    rho = lambda;
  else if (u == 0)
    rho = ONE();
  else
    rho = ZERO(); // u == 1

  // result = Ω·β + (ρ + γ - 1)
  // The -1 is for 0-based indexing: only apply when ρ+γ is finite.
  // For infinite ordinals, V handles enumeration natively.
  TermPtr delta;
  {
    TermPtr sum = add(rho, gamma);
    if (isZero(sum)) {
      delta = ZERO();
    } else if (isOrdinalFinite(sum)) {
      // sum is finite natural: subtract 1
      int n = 0;
      TermPtr cur = sum;
      while (!isZero(cur)) {
        n++;
        cur = cur->c;
      }
      if (n <= 1)
        delta = ZERO();
      else {
        delta = ZERO();
        for (int i = 1; i < n; i++) {
          delta = add(delta, ONE());
        }
      }
    } else {
      // sum is infinite: -1 does nothing (V handles enumeration)
      delta = sum;
    }
  }

  // Ω·β + delta
  TermPtr omegaBeta = omegaTimes(beta);
  return add(omegaBeta, delta);
}

// ----------------------------------------------------------
// Render V(alpha) as a Veblen φ expression with @ notation
// V(0) = ∅ → 1
// V(ξ + Ω^β·γ) = V(ξ) ∪ {(γ, V(β))}
// ----------------------------------------------------------
static std::string renderArray(TermPtr alpha, bool vMode, bool sugar, bool isPosition = false);
static std::string renderArrayMatrix(TermPtr alpha, bool sugar = true, bool isPosition = false);
static std::string renderVeblenRec(TermPtr q, bool vMode, bool sugar);
static std::string renderVeblenRecMatrix(TermPtr q, bool sugar = true);
static std::string renderPosition(TermPtr beta, bool vMode, bool sugar);
static std::string renderPositionMatrix(TermPtr beta, bool sugar = true);
static std::string renderVeblenCoeff(TermPtr c, bool vMode, bool sugar);
static std::string renderVeblenCoeffMatrix(TermPtr c, bool sugar = true);

// Check if ordinal contains any ψ_a term with a > 0 (Ω-power), recursively
static bool hasOmegaPowerDeep(TermPtr t) {
  if (isZero(t))
    return false;
  if (!isZero(t->a))
    return true;
  return hasOmegaPowerDeep(t->b) || hasOmegaPowerDeep(t->c);
}

// Render V(α) body without the outer \varphi(...) wrapper, for use as a
// position string Always uses V mode (strip φ) internally; sugar controls
// ε/ζ/η/Γ.
static std::string renderArrayBody(TermPtr alpha, bool sugar, bool isPosition = false) {
  std::string full = renderArray(alpha, true, sugar, isPosition);
  if (isZero(alpha))
    return "0";
  if (full.size() > 8 && full.substr(0, 8) == "\\varphi(" && full.back() == ')') {
    return full.substr(8, full.size() - 9);
  }
  if (full.size() > 9 && full.substr(0, 9) == "\\omega^{" && full.back() == '}') {
    return full.substr(9, full.size() - 10);
  }
  return full;
}

static std::string renderPosition(TermPtr beta, bool vMode, bool sugar) {
  if (isZero(beta))
    return "0";
  if (isFiniteNat(beta)) {
    int len = length1(beta);
    return std::to_string(len);
  }
  if (hasOmegaPowerDeep(beta)) {
    std::string body;
    if (vMode) {
      body = renderArrayBody(beta, sugar, true);
    } else {
      body = renderArray(beta, false, sugar, true);
    }
    if (body.find(',') != std::string::npos || body.find('@') != std::string::npos) {
      if (body.size() > 8 && body.substr(0, 8) == "\\varphi(" && body.back() == ')') {
        return body;
      }
      return "(" + body + ")";
    }
    return body;
  }
  return renderTerm(beta);
}

static std::string renderPositionMatrix(TermPtr beta, bool sugar) {
  if (isZero(beta))
    return "0";
  if (isFiniteNat(beta)) {
    int len = length1(beta);
    return std::to_string(len);
  }
  if (hasOmegaPowerDeep(beta)) {
    std::string m = renderArrayMatrix(beta, sugar, true);
    if (m.find('&') != std::string::npos) {
      return "(" + m + ")";
    }
    return m;
  }
  return renderTerm(beta);
}

static std::string renderVeblenCoeff(TermPtr c, bool vMode, bool sugar) {
  if (isZero(c))
    return "0";
  if (isFiniteNat(c))
    return std::to_string(length1(c));
  if (hasOmegaPowerDeep(c)) {
    std::string v = renderVeblenRec(c, vMode, sugar);
    if (!v.empty())
      return v;
  }
  return renderTerm(c);
}

static std::string renderVeblenCoeffMatrix(TermPtr c, bool sugar) {
  if (isZero(c))
    return "0";
  if (isFiniteNat(c))
    return std::to_string(length1(c));
  if (hasOmegaPowerDeep(c)) {
    std::string v = renderVeblenRecMatrix(c, sugar);
    if (!v.empty())
      return v;
  }
  return renderTerm(c);
}

static std::string renderArray(TermPtr alpha, bool vMode, bool sugar, bool isPosition) {
  if (isZero(alpha))
    return "1";

  std::vector<OmegaCnfTerm> terms;
  TermPtr tail = ZERO();
  decomposeOmegaCnf(alpha, terms, tail);

  bool hasComplexPosition = false;
  for (auto &t : terms) {
    if (!isFiniteNat(t.exponent)) {
      hasComplexPosition = true;
      break;
    }
  }

  if (!hasComplexPosition && terms.empty()) {
    if (isZero(tail))
      return "1";
    if (isZero(tail->c) && isZero(tail->a) && hasOmegaPowerDeep(tail)) {
      std::string coeffStr = renderVeblenCoeff(tail, vMode, sugar);
      // If the coefficient already starts with ω^ (e.g., ψ₀(Ω+1) → ω^(ε₀+1)),
      // V must add another ω^ layer: V(ψ₀(Ω+1)) = ω^ω^(ε₀+1).
      // For terminal forms like ε₀ = φ(1,0), no extra wrapping needed.
      if (coeffStr.size() >= 8 && coeffStr.substr(0, 8) == "\\omega^{") {
        if (isPosition && !sugar) {
          return "\\varphi(\\omega^{" + coeffStr + "})";
        }
        return "\\omega^{" + coeffStr + "}";
      }
      if (isPosition && !sugar) {
        return "\\varphi(" + coeffStr + ")";
      }
      return coeffStr;
    }
    {
      std::string coeffStr = renderVeblenCoeff(tail, vMode, sugar);
      if (coeffStr == "1")
        return "\\omega";
      return "\\omega^{" + coeffStr + "}";
    }
  }

  if (!hasComplexPosition) {
    int maxExp = 0;
    for (auto &t : terms) {
      int e = length1(t.exponent);
      if (e > maxExp)
        maxExp = e;
    }

    std::vector<TermPtr> coeffTerms(maxExp + 1, ZERO());

    for (auto &t : terms) {
      int pos = length1(t.exponent);
      TermPtr contrib = ZERO();
      if (eq(t.second, ONE()) || isZero(t.second)) {
        for (int i = 0; i < t.count; i++)
          contrib = add(contrib, ONE());
      } else {
        for (int i = 0; i < t.count; i++)
          contrib = add(contrib, t.second);
      }
      coeffTerms[pos] = add(coeffTerms[pos], contrib);
    }

    // Syntactic sugar: φ(1,x)→ε_x, φ(2,x)→ζ_x, φ(3,x)→η_x, φ(1,0,x)→Γ_x
    // Suppress in position context: position (1,0) should stay (1,0), not
    // become ε₀
    if (sugar && !isPosition) {
      std::string tailSugar = isZero(tail) ? "0" : renderVeblenCoeff(tail, vMode, sugar);
      if (maxExp == 1 && eq(coeffTerms[1], ONE())) {
        return "\\varepsilon_{" + tailSugar + "}";
      }
      if (maxExp == 1 && eq(coeffTerms[1], add(ONE(), ONE()))) {
        return "\\zeta_{" + tailSugar + "}";
      }
      if (maxExp == 1 && eq(coeffTerms[1], add(add(ONE(), ONE()), ONE()))) {
        return "\\eta_{" + tailSugar + "}";
      }
      if (maxExp == 2 && eq(coeffTerms[2], ONE()) && isZero(coeffTerms[1])) {
        return "\\Gamma_{" + tailSugar + "}";
      }
    }

    std::string result = "\\varphi(";
    bool first = true;
    for (int i = maxExp; i >= 1; i--) {
      if (!first)
        result += ",";
      result += renderVeblenCoeff(coeffTerms[i], vMode, sugar);
      first = false;
    }
    if (!first || !isZero(tail)) {
      if (!first)
        result += ",";
      if (isZero(tail))
        result += "0";
      else
        result += renderVeblenCoeff(tail, vMode, sugar);
    }
    result += ")";
    return result;
  }

  // Has non-finite positions — use @ notation
  std::string result = "\\varphi(";
  bool first = true;

  for (auto &t : terms) {
    if (!first)
      result += ",";
    std::string coeff;
    if (eq(t.second, ONE()) || isZero(t.second)) {
      coeff = std::to_string(t.count);
    } else {
      coeff = renderVeblenCoeff(t.second, vMode, sugar);
      if (coeff.find('+') != std::string::npos)
        coeff = "(" + coeff + ")";
    }
    std::string pos = renderPosition(t.exponent, vMode, sugar);
    result += coeff + "{@}" + pos;
    first = false;
  }

  if (!isZero(tail)) {
    if (!first)
      result += ",";
    std::string tcoeff = renderVeblenCoeff(tail, vMode, sugar);
    if (tcoeff.find('+') != std::string::npos)
      tcoeff = "(" + tcoeff + ")";
    result += tcoeff + "{@}0";
    first = false;
  }

  result += ")";
  return result;
}

// ----------------------------------------------------------
// Render V(t(α)) for ψ₀(α) in φ notation (matrix / two-row form)
// ----------------------------------------------------------
static std::string renderArrayMatrix(TermPtr alpha, bool sugar, bool isPosition) {
  if (isZero(alpha))
    return "1";

  std::vector<OmegaCnfTerm> terms;
  TermPtr tail = ZERO();
  decomposeOmegaCnf(alpha, terms, tail);

  bool hasComplexPosition = false;
  for (auto &t : terms) {
    if (!isFiniteNat(t.exponent)) {
      hasComplexPosition = true;
      break;
    }
  }

  if (!hasComplexPosition && terms.empty()) {
    if (isZero(tail))
      return "1";
    if (isZero(tail->c) && isZero(tail->a) && hasOmegaPowerDeep(tail)) {
      return renderVeblenCoeffMatrix(tail, sugar);
    }
    {
      std::string coeffStr = renderVeblenCoeffMatrix(tail, sugar);
      if (coeffStr == "1")
        return "\\omega";
      return "\\omega^{" + coeffStr + "}";
    }
  }

  if (!hasComplexPosition) {
    int maxExp = 0;
    for (auto &t : terms) {
      int e = length1(t.exponent);
      if (e > maxExp)
        maxExp = e;
    }
    std::vector<TermPtr> coeffTerms(maxExp + 1, ZERO());
    for (auto &t : terms) {
      int pos = length1(t.exponent);
      TermPtr contrib = ZERO();
      if (eq(t.second, ONE()) || isZero(t.second)) {
        for (int i = 0; i < t.count; i++)
          contrib = add(contrib, ONE());
      } else {
        for (int i = 0; i < t.count; i++)
          contrib = add(contrib, t.second);
      }
      coeffTerms[pos] = add(coeffTerms[pos], contrib);
    }

    if (sugar && !isPosition) {
      std::string tailSugar = isZero(tail) ? "0" : renderVeblenCoeffMatrix(tail, sugar);
      if (maxExp == 1 && eq(coeffTerms[1], ONE())) {
        return "\\varepsilon_{" + tailSugar + "}";
      }
      if (maxExp == 1 && eq(coeffTerms[1], add(ONE(), ONE()))) {
        return "\\zeta_{" + tailSugar + "}";
      }
      if (maxExp == 1 && eq(coeffTerms[1], add(add(ONE(), ONE()), ONE()))) {
        return "\\eta_{" + tailSugar + "}";
      }
      if (maxExp == 2 && eq(coeffTerms[2], ONE()) && isZero(coeffTerms[1])) {
        return "\\Gamma_{" + tailSugar + "}";
      }
    }

    std::string top, bottom;
    bool first = true;
    for (int i = maxExp; i >= 1; i--) {
      if (!first) {
        top += "&";
        bottom += "&";
      }
      top += renderVeblenCoeffMatrix(coeffTerms[i], sugar);
      bottom += std::to_string(i);
      first = false;
    }
    if (!isZero(tail)) {
      if (!first) {
        top += "&";
        bottom += "&";
      }
      top += renderVeblenCoeffMatrix(tail, sugar);
      bottom += "0";
    }
    return "\\varphi\\begin{pmatrix}" + top + "\\\\" + bottom + "\\end{pmatrix}";
  }

  std::string top, bottom;
  bool first = true;
  for (auto &t : terms) {
    if (!first) {
      top += "&";
      bottom += "&";
    }
    std::string coeff;
    if (eq(t.second, ONE()) || isZero(t.second)) {
      coeff = std::to_string(t.count);
    } else {
      coeff = renderVeblenCoeffMatrix(t.second, sugar);
      if (coeff.find('+') != std::string::npos)
        coeff = "(" + coeff + ")";
    }
    top += coeff;
    bottom += renderPositionMatrix(t.exponent, sugar);
    first = false;
  }
  if (!isZero(tail)) {
    if (!first) {
      top += "&";
      bottom += "&";
    }
    std::string tcoeff = renderVeblenCoeffMatrix(tail, sugar);
    if (tcoeff.find('+') != std::string::npos)
      tcoeff = "(" + tcoeff + ")";
    top += tcoeff;
    bottom += "0";
  }
  return "\\varphi\\begin{pmatrix}" + top + "\\\\" + bottom + "\\end{pmatrix}";
}

static std::string psi0ToVeblenMatrix(TermPtr alpha, bool sugar = true) {
  if (isZero(alpha))
    return "1";
  TermPtr t_alpha = computeT(alpha);
  return renderArrayMatrix(t_alpha, sugar);
}

static std::string renderVeblenRecMatrix(TermPtr q, bool sugar) {
  if (isZero(q))
    return "0";
  if (isOrdinalFinite(q)) {
    int len = length1(q);
    return (len <= 1) ? "1" : std::to_string(len);
  }
  if (!isBelowBHO(q))
    return "";
  auto [head, tail] = separate(q, firstTerm(q));
  if (!isZero(head->a))
    return "";
  std::string result = psi0ToVeblenMatrix(head->b, sugar);
  if (result.empty())
    return "";
  TermPtr cur = head->c;
  while (!isZero(cur) && isZero(cur->a) && eq(cur->b, head->b)) {
    std::string dup = psi0ToVeblenMatrix(cur->b, sugar);
    if (dup.empty())
      return "";
    result += "+" + dup;
    cur = cur->c;
  }
  if (!isZero(cur)) {
    std::string tailV = renderVeblenRecMatrix(cur, sugar);
    if (tailV.empty())
      return "";
    result += "+" + tailV;
  }
  if (!isZero(tail)) {
    std::string tailV = renderVeblenRecMatrix(tail, sugar);
    if (tailV.empty())
      return "";
    result += "+" + tailV;
  }
  return result;
}

std::string termToVeblenMatrix(TermPtr q) { return renderVeblenRecMatrix(q, true); }

std::string termToVeblenMatrixPlain(TermPtr q) { return renderVeblenRecMatrix(q, false); }

// ----------------------------------------------------------
// Render V(t(α)) for ψ₀(α) in φ notation
// ----------------------------------------------------------
static std::string psi0ToVeblen(TermPtr alpha, bool vMode, bool sugar) {
  if (isZero(alpha))
    return "1";

  // Apply t(α) first, then render V(t(α))
  TermPtr t_alpha = computeT(alpha);
  return renderArray(t_alpha, vMode, sugar);
}

static std::string renderVeblenRec(TermPtr q, bool vMode, bool sugar) {
  if (isZero(q))
    return "0";
  if (isOrdinalFinite(q)) {
    int len = length1(q);
    return (len <= 1) ? "1" : std::to_string(len);
  }

  if (!isBelowBHO(q))
    return "";

  auto [head, tail] = separate(q, firstTerm(q));

  if (!isZero(head->a))
    return ""; // not ψ₀

  std::string result = psi0ToVeblen(head->b, vMode, sugar);
  if (result.empty())
    return "";

  // Handle duplicate copies of the same term in head
  TermPtr cur = head->c;
  while (!isZero(cur) && isZero(cur->a) && eq(cur->b, head->b)) {
    std::string dup = psi0ToVeblen(cur->b, vMode, sugar);
    if (dup.empty())
      return "";
    result += "+" + dup;
    cur = cur->c;
  }

  // Process remaining tail after duplicates
  if (!isZero(cur)) {
    std::string tailV = renderVeblenRec(cur, vMode, sugar);
    if (tailV.empty())
      return "";
    result += "+" + tailV;
  }

  if (!isZero(tail)) {
    std::string tailV = renderVeblenRec(tail, vMode, sugar);
    if (tailV.empty())
      return "";
    result += "+" + tailV;
  }

  return result;
}

std::string termToVeblen(TermPtr q) {
  return renderVeblenRec(q, true, true); // V mode + sugar
}

std::string termToVeblenPlain(TermPtr q) {
  return renderVeblenRec(q, true, false); // V mode, no sugar
}

std::string termToString(TermPtr q, bool latex) {
  (void)latex;
  g_renderCache.clear();
  return renderTerm(q);
}

// Debug wrappers
std::pair<TermPtr, TermPtr> debugDecomposePower(TermPtr a) { return decomposePower(a); }
TermPtr debugComputeT(TermPtr alpha) { return computeT(alpha); }
