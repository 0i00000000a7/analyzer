#include "ordinal.h"
#include <algorithm>
// ============================================================
// BMS analysis
// ============================================================

const Matrix EBO = {{0, 0, 0}, {1, 1, 1}, {2, 1, 1}, {3, 1, 0}, {2, 0, 0}};

// ── Parent cache (built once per BMSToBocf call) ──
static std::vector<std::vector<int>> g_parentCache;
static bool g_parentCacheReady = false;
static std::vector<std::vector<int>> g_childrenCache;

static void buildParentCache(const Matrix &M) {
  int l = (int)M.size();
  int rows = 0;
  for (int i = 0; i < l; i++)
    if ((int)M[i].size() > rows)
      rows = (int)M[i].size();

  // Pad matrix columns to uniform row count
  std::vector<std::vector<int>> S(l);
  for (int i = 0; i < l; i++) {
    S[i] = M[i];
    while ((int)S[i].size() < rows)
      S[i].push_back(0);
  }

  g_parentCache.assign(l, std::vector<int>());
  for (int row = 0; row < rows; row++) {
    if (row == 0) {
      // Row 0: monotonic stack — O(cols)
      std::vector<int> stack;
      stack.reserve(l);
      for (int col = 0; col < l; col++) {
        while (!stack.empty() && S[stack.back()][0] >= S[col][0]) {
          stack.pop_back();
        }
        g_parentCache[col].push_back(stack.empty() ? -1 : stack.back());
        stack.push_back(col);
      }
    } else {
      // Rows > 0: follow parent chain from row above
      for (int col = 0; col < l; col++) {
        int k = col;
        while (k >= 0 && S[k][row] >= S[col][row]) {
          k = g_parentCache[k][row - 1];
        }
        g_parentCache[col].push_back(k);
      }
    }
  }
  g_parentCacheReady = true;

  // Build children cache from row-0 parents
  g_childrenCache.assign(l, std::vector<int>());
  for (int i = 0; i < l; i++) {
    int p = g_parentCache[i][0];
    if (p >= 0) {
      g_childrenCache[p].push_back(i);
    }
  }
}

int findParent(const Matrix &M, int findRow, int relativeColumn) {
  if (g_parentCacheReady) {
    if (findRow == -1)
      return relativeColumn - 1;
    return g_parentCache[relativeColumn][findRow];
  }
  // Fallback (no cache — shouldn't be reached for normal BMSToBocf calls)
  if (findRow == -1) {
    return relativeColumn - 1;
  }
  int curColumn = findParent(M, findRow - 1, relativeColumn);
  while (curColumn > -1 && M[curColumn][findRow] >= M[relativeColumn][findRow]) {
    curColumn = findParent(M, findRow - 1, curColumn);
  }
  return curColumn;
}

std::vector<int> children(const Matrix &M, int n) {
  if (g_parentCacheReady) {
    return g_childrenCache[n];
  }
  // Fallback (no cache)
  std::vector<int> X;
  for (int i = 0; i < (int)M.size(); i++) {
    if (findParent(M, 0, i) == n) {
      X.push_back(i);
    }
  }
  return X;
}

// ── Upgrader cache (−2 = not computed, −1 = none, ≥0 = index) ──
static std::vector<int> g_upgraderCache;

int getUpgrader(const Matrix &M, int n) {
  // Check cache
  if ((size_t)n < g_upgraderCache.size()) {
    int cached = g_upgraderCache[n];
    if (cached != -2)
      return cached;
  }
  // Compute
  int result;
  if (M[n].size() < 3 || M[n][1] == 0 || M[n][2] == 1 || n + 1 >= (int)M.size()) {
    result = -1;
    goto done;
  }
  {
    int m = findParent(M, 1, n);
    if (m < 0 || m >= (int)M.size()) {
      result = -1;
      goto done;
    }
    MatrixRow L = {M[m][0] + 1, M[n][1], M[m][2] + 1};

    if (findParent(M, 1, n) == findParent(M, 1, n + 1)) {
      bool match = (M[n + 1].size() >= 3 && M[n + 1][0] == L[0] && M[n + 1][1] == L[1] && M[n + 1][2] == L[2]);
      if (match) {
        result = n + 1;
        goto done;
      }
    }

    int q = n;
    while (q != -1) {
      q = findParent(M, 0, q);
      if (q == -1)
        break;
      if (findParent(M, 1, n) == findParent(M, 1, q)) {
        bool match = (M[q].size() >= 3 && M[q][0] == L[0] && M[q][1] == L[1] && M[q][2] == L[2]);
        if (match && n + 1 < (int)M.size() && M[n + 1][0] > M[q][0]) {
          result = q;
          goto done;
        }
      }
    }
    result = -1;
  }
done:
  if (g_upgraderCache.empty())
    g_upgraderCache.assign(M.size(), -2);
  if ((size_t)n >= g_upgraderCache.size())
    g_upgraderCache.resize(n + 1, -2);
  g_upgraderCache[n] = result;
  return result;
}

static bool rowEq3(const MatrixRow &r, int a, int b, int c) { return r.size() >= 3 && r[0] == a && r[1] == b && r[2] == c; }

// ── Column result cache (populated on demand, cleared per BMSToBocf call) ──
static std::vector<TermPtr> g_columnCache;
static std::vector<bool> g_columnCached;
static TermPtr getCachedNotStandardExpr(const Matrix &M, int n);

// ── Index-of-column cache ──
static std::vector<TermPtr> g_indexCache;
static std::vector<bool> g_indexCached;
static TermPtr getCachedIndexOfColumn(const Matrix &M, int n);

TermPtr getIndexOfColumn(const Matrix &M, int n) {
  if (M[n].size() < 2 || M[n][1] == 0) {
    return ZERO();
  }
  if (M[n].size() < 3 || M[n][2] == 0) {
    int upgradeIdx = getUpgrader(M, n);
    TermPtr upgradingTermAdm = (upgradeIdx >= 0) ? lastTerm(getCachedIndexOfColumn(M, upgradeIdx)) : ONE();
    return add(getCachedIndexOfColumn(M, findParent(M, 1, n)), upgradingTermAdm);
  }

  TermPtr omega_power_x_counter = ONE();
  for (int i : children(M, n)) {
    if (M[i].size() < 3)
      continue;
    if (!rowEq3(M[i], M[n][0] + 1, M[n][1], 1)) {
      continue;
    }
    TermPtr q = ZERO();
    for (int j : children(M, i)) {
      q = add(q, getCachedNotStandardExpr(M, j));
    }
    omega_power_x_counter = add(omega_power_x_counter, exp(q));
  }
  return add(getCachedIndexOfColumn(M, findParent(M, 1, n)), exp(omega_power_x_counter));
}

TermPtr NotStandardExprFromColumn(const Matrix &M, int n) {
  TermPtr omegaMultiplication = ZERO();
  { // upgrader section
    for (int i : children(M, n)) {
      if (M[i].size() >= 3 && rowEq3(M[i], M[n][0] + 1, M[n][1], 1)) {
        continue;
      }
      bool isUpgrader = std::find(g_upgraderCache.begin(), g_upgraderCache.end(), i) != g_upgraderCache.end();
      if (isUpgrader) {
        auto c = children(M, i);
        if (!c.empty()) {
          if (M[c.back()].size() >= 3 && rowEq3(M[c.back()], M[i][0] + 1, M[i][1], 1)) {
            continue;
          }
        } else {
          continue;
        }
      }
      omegaMultiplication = add(omegaMultiplication, getCachedNotStandardExpr(M, i));
    }
  }

  TermPtr result = T(getCachedIndexOfColumn(M, n), omegaMultiplication, ZERO());
  return result;
}

// ── Column result cache implementation ──
static TermPtr getCachedNotStandardExpr(const Matrix &M, int n) {
  // Fast path: already cached (lock-free read for already set flag)
  if (g_columnCached.size() > (size_t)n && g_columnCached[n]) {
    return g_columnCache[n];
  }
  TermPtr result = NotStandardExprFromColumn(M, n);
  if (g_columnCached.size() <= (size_t)n || !g_columnCached[n]) {
    if (g_columnCached.size() <= (size_t)n) {
      g_columnCache.resize(n + 1);
      g_columnCached.resize(n + 1, false);
    }
    g_columnCache[n] = result;
    g_columnCached[n] = true;
  }
  return result;
}

static TermPtr getCachedIndexOfColumn(const Matrix &M, int n) {
  // Fast path: already cached (lock-free read for already set flag)
  if (g_indexCached.size() > (size_t)n && g_indexCached[n]) {
    return g_indexCache[n];
  }
  TermPtr result = getIndexOfColumn(M, n);
  if (g_indexCached.size() <= (size_t)n || !g_indexCached[n]) {
    if (g_indexCached.size() <= (size_t)n) {
      g_indexCache.resize(n + 1);
      g_indexCached.resize(n + 1, false);
    }
    g_indexCache[n] = result;
    g_indexCached[n] = true;
  }
  return result;
}

TermPtr BMSToBocf(const Matrix &M) {
  buildParentCache(M);

  // Precompute upgrader cache for the entire matrix
  int l = (int)M.size();
  g_upgraderCache.assign(l, -2);
  for (int x = 0; x < l; x++) {
    g_upgraderCache[x] = getUpgrader(M, x);
  }

  TermPtr S = ZERO();
  for (int i = 0; i < (int)M.size(); i++) {
    if (M[i].size() >= 1 && M[i][0] == 0 && (M[i].size() < 2 || M[i][1] == 0) && (M[i].size() < 3 || M[i][2] == 0)) {
      S = add(S, getCachedNotStandardExpr(M, i));
    }
  }
  g_parentCacheReady = false;
  g_parentCache.clear();
  g_childrenCache.clear();
  g_upgraderCache.clear();
  g_columnCache.clear();
  g_columnCached.clear();
  g_indexCache.clear();
  g_indexCached.clear();
  TermPtr result = standardForm(S);
  return result;
}
