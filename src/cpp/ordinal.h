#ifndef BMS_ORDINAL_H
#define BMS_ORDINAL_H

#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <vector>

struct Term;
using TermPtr = std::shared_ptr<const struct Term>;

// ψ_a(b) + c  represented as [a, b, c]
// [] (nullptr) represents zero
struct Term {
  TermPtr a; // index of the psi function
  TermPtr b; // argument of the psi function
  TermPtr c; // rest of the sum (tail)

  Term() = delete;
  Term(TermPtr a_, TermPtr b_, TermPtr c_) : a(std::move(a_)), b(std::move(b_)), c(std::move(c_)) {}
};

inline TermPtr ZERO() { return nullptr; }
inline bool isZero(TermPtr t) { return t == nullptr; }

inline TermPtr T(TermPtr a, TermPtr b, TermPtr c = ZERO()) {
  return std::make_shared<const Term>(std::move(a), std::move(b), std::move(c));
}

// Constants
inline TermPtr ONE() {
  static auto v = T(ZERO(), ZERO());
  return v;
}
inline TermPtr OMEGA() {
  static auto v = T(ZERO(), ONE());
  return v;
}
inline TermPtr OMEGA1() {
  static auto v = T(ONE(), ZERO());
  return v;
}
inline TermPtr EPSILON0() {
  static auto v = T(ZERO(), OMEGA1());
  return v;
}

bool isOrdinalFinite(TermPtr a);
int length1(TermPtr a);
bool eq(TermPtr a, TermPtr b);
bool lt(TermPtr a, TermPtr b);
inline bool gt(TermPtr a, TermPtr b) { return !lt(a, b) && !eq(a, b); }
inline bool le(TermPtr a, TermPtr b) { return lt(a, b) || eq(a, b); }

TermPtr firstTerm(TermPtr a);
TermPtr lastTerm(TermPtr a);
std::vector<TermPtr> everyTerms(TermPtr a);

TermPtr add(TermPtr a, TermPtr b);
inline TermPtr succ(TermPtr a) { return add(a, ONE()); }
TermPtr mulFinite(TermPtr a, TermPtr b);
TermPtr mul(TermPtr a, TermPtr b);
TermPtr sub(TermPtr a, TermPtr b);
std::pair<TermPtr, TermPtr> separate(TermPtr a, TermPtr b);
TermPtr truncate(TermPtr a, TermPtr b);
TermPtr exp(TermPtr a);
TermPtr log(TermPtr a);

/// Maximum depth of subscript (a-component) nesting.
int subscriptDepth(TermPtr t);

TermPtr mergePsiAddends(TermPtr a, TermPtr b, TermPtr c);
TermPtr standardForm(TermPtr a);

// Fundamental sequences
bool isSucc(TermPtr a);
TermPtr pred(TermPtr a);
TermPtr cofinality(TermPtr a);
TermPtr fundamentalSequence(TermPtr a, int n);
TermPtr fundamentalSequence(TermPtr a, TermPtr index);

using MatrixRow = std::vector<int>;
using Matrix = std::vector<MatrixRow>;

// BMS → BOCF conversion core functions
int findParent(const Matrix &M, int findRow, int relativeColumn);
std::vector<int> children(const Matrix &M, int n);
int getUpgrader(const Matrix &M, int n);
TermPtr getIndexOfColumn(const Matrix &M, int n);
TermPtr NotStandardExprFromColumn(const Matrix &M, int n);
TermPtr BMSToBocf(const Matrix &M);

// 0-Y and expansion utilities
using Mountain = std::vector<std::vector<std::pair<int, int>>>;
Mountain buildMountain(const std::vector<int> &seq);
Matrix zeroYToBMS(const std::vector<int> &seq);
std::vector<int> zeroYExpand(const std::vector<int> &seq, int n);
std::string zeroYExpand(const std::string &seqStr, int n);
std::string bmsTo0YSequence(const Matrix &M);
Matrix expandBMS(const Matrix &M, int fs);
Matrix triangularToBMS(const Matrix &M);
Matrix bmsToTriangular(const Matrix &M);

// 1-Y ↔ DBMS conversion (below [1,3] = triangular BMS)
Matrix oneYToDBMS(const std::vector<int> &seq);
std::vector<int> dbmsToOneY(const Matrix &dbms);
std::string dbmsToString(const Matrix &dbms);
Matrix dbmsToBMS(const Matrix &dbms);

// Matrix comparison
int matrixLexOrder(const Matrix &a, const Matrix &b);

// EBO (Extended Buchholz Ordinal) check
extern const Matrix EBO;
bool isEqEBO(const Matrix &M);
bool isGtEBO(const Matrix &M);
inline bool isGteEBO(const Matrix &M) { return isEqEBO(M) || isGtEBO(M); }

// String conversion
std::string termToString(TermPtr q, bool latex = true);
std::string termToVeblen(TermPtr q);
std::string termToVeblenPlain(TermPtr q);
std::string termToVeblenMatrix(TermPtr q);
std::string termToVeblenMatrixPlain(TermPtr q);
inline TermPtr BHO() {
  static auto v = T(ZERO(), T(succ(ONE()), ZERO()));
  return v;
}

// Debug helpers
std::pair<TermPtr, TermPtr> debugDecomposePower(TermPtr a);
TermPtr debugComputeT(TermPtr alpha);

#endif // BMS_ORDINAL_H