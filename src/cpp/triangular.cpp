#include "ordinal.h"
#include <algorithm>
#include <set>
#include <stdexcept>
#include <vector>

// ============================================================
// Ancestor-based BMS ↔ Triangular BMS conversion
// Ported from bms_ancestor_converter_v6.py
// Supports arbitrary n-row matrices.
// ============================================================

using Column = std::vector<int>;
using Columns = std::vector<Column>;

// ── Column comparison (lexicographic) ──

static bool colEq(const Column &a, const Column &b) {
  if (a.size() != b.size())
    return false;
  for (size_t i = 0; i < a.size(); i++)
    if (a[i] != b[i])
      return false;
  return true;
}
static bool colLess(const Column &a, const Column &b) {
  size_t n = std::min(a.size(), b.size());
  for (size_t i = 0; i < n; i++)
    if (a[i] != b[i])
      return a[i] < b[i];
  return a.size() < b.size();
}
// a >= b  ⇔  !(a < b)
static bool colGeq(const Column &a, const Column &b) { return !colLess(a, b); }

/// Lexicographic compare of two column sequences (Python tuple-of-tuples style).
/// Returns <0 (a<b), 0 (a==b), >0 (a>b).
static int seqCmp(const Columns &a, const Columns &b) {
  size_t n = std::min(a.size(), b.size());
  for (size_t i = 0; i < n; i++) {
    if (colEq(a[i], b[i]))
      continue;
    return colLess(a[i], b[i]) ? -1 : 1;
  }
  if (a.size() == b.size())
    return 0;
  return a.size() < b.size() ? -1 : 1;
}

// ── Helpers ──

/// Largest 1-based row where col[row-1] > 0; 0 if all zero.
static int lastPositiveRow(const Column &col) {
  for (int i = (int)col.size() - 1; i >= 0; i--)
    if (col[i] > 0)
      return i + 1;
  return 0;
}

static Column incrementPrefix(const Column &col, int count) {
  Column r = col;
  for (int i = 0; i < count && i < (int)r.size(); i++)
    r[i]++;
  return r;
}

static Column decrementPrefix(const Column &col, int count) {
  Column r = col;
  for (int i = 0; i < count && i < (int)r.size(); i++) {
    if (r[i] == 0)
      return Column(); // sentinel failure
    r[i]--;
  }
  return r;
}

static Column incrementRow(const Column &col, int row /*1-based*/) {
  Column r = col;
  if (row >= 1 && row <= (int)r.size())
    r[row - 1]++;
  return r;
}

static Column zeroFromRow(const Column &col, int row /*1-based*/) {
  Column r = col;
  for (int i = row - 1; i < (int)r.size(); i++)
    r[i] = 0;
  return r;
}

static Column firstRowColumn(int value, int n) {
  Column r(n, 0);
  r[0] = value;
  return r;
}

/// Check if all entries in first `count` positions are > 0 (can be decremented).
static bool canDecrementPrefix(const Column &col, int count) {
  for (int i = 0; i < count && i < (int)col.size(); i++)
    if (col[i] == 0)
      return false;
  return true;
}

// ── AncestorIndex ──

class AncestorIndex {
public:
  int n;
  int columnCount;
  Columns columns;
  std::vector<std::vector<int>> parents;             // parents[row][col]; -1 = none
  std::vector<std::vector<std::set<int>>> ancestors; // ancestors[row][col]

  AncestorIndex(const Columns &cols) : columns(cols) {
    if (cols.empty()) {
      n = 0;
      columnCount = 0;
      return;
    }
    columnCount = (int)cols.size();
    n = (int)cols[0].size();

    parents.assign(n + 1, std::vector<int>(columnCount, -1));
    ancestors.assign(n + 1, std::vector<std::set<int>>(columnCount));

    // Row 0 (virtual): ancestors are all columns to the left
    for (int c = 1; c < columnCount; c++) {
      parents[0][c] = c - 1;
      for (int a = 0; a < c; a++)
        ancestors[0][c].insert(a);
    }

    // Rows 1..n
    for (int row = 1; row <= n; row++) {
      int vi = row - 1;
      for (int c = 0; c < columnCount; c++) {
        int parent = -1;
        auto &up = ancestors[row - 1][c];
        for (auto it = up.rbegin(); it != up.rend(); ++it) {
          if (cols[*it][vi] < cols[c][vi]) {
            parent = *it;
            break;
          }
        }
        parents[row][c] = parent;
        if (parent >= 0) {
          ancestors[row][c] = ancestors[row][parent];
          ancestors[row][c].insert(parent);
        }
      }
    }
  }

  bool hasAncestorColumn(int elementCol, int row, int ancestorCol) const {
    if (row < 0 || row > n || elementCol < 0 || elementCol >= columnCount || ancestorCol < 0 || ancestorCol >= columnCount)
      return false;
    return ancestors[row][elementCol].count(ancestorCol) > 0;
  }

  bool parentIsColumn(int elementCol, int row, int parentCol) const {
    if (row < 0 || row > n || elementCol < 0 || elementCol >= columnCount)
      return false;
    return parents[row][elementCol] == parentCol;
  }
};

// ============================================================
// Triangular BMS → Standard BMS
// ============================================================

Matrix triangularToBMS(const Matrix &M) {
  Columns cols;
  for (auto &c : M)
    cols.push_back(c);
  if (cols.empty())
    return {};
  int n = (int)cols[0].size();
  if (n < 2)
    return {};

  int idx = (int)cols.size() - 1;

  while (idx >= 0) {
    Column &x = cols[idx];
    int rowNminus2 = (n >= 2) ? x[n - 2] : 0;
    if (rowNminus2 > 0) {
      idx--;
      continue;
    }

    int k = lastPositiveRow(x);
    if (k + 2 > n) {
      idx--;
      continue;
    }

    Column y = incrementPrefix(x, k + 1);
    Column z = incrementPrefix(y, k + 2);

    int yIdx = idx + 1;
    int machineStart = idx + 2;

    if (yIdx >= (int)cols.size() || !colEq(cols[yIdx], y) || machineStart >= (int)cols.size() ||
        colLess(cols[machineStart], z)) {
      idx--;
      continue;
    }

    AncestorIndex ancestor(cols);
    Columns xPrime;
    int cursor = machineStart;
    int xEnd = cursor;
    int lastL = -1;
    bool lastStoppedByXParent = false;

    while (true) {
      if (cursor >= (int)cols.size() || colLess(cols[cursor], z)) {
        xEnd = cursor;
        break;
      }

      Column &t = cols[cursor];

      // Find matching rows l (0..k+1) where t[l] has ancestor in y
      int l = -1;
      for (int row = 0; row <= k + 1; row++) {
        if (ancestor.hasAncestorColumn(cursor, row, yIdx))
          l = row; // take max
      }
      if (l < 0)
        return {}; // non-standard

      bool stoppedByXParent = (l <= k) && ancestor.parentIsColumn(cursor, l + 1, idx);

      Column tPrime = decrementPrefix(t, l);
      if (tPrime.empty())
        return {};

      if (stoppedByXParent)
        tPrime = zeroFromRow(tPrime, l + 2);

      xPrime.push_back(tPrime);
      cursor++;

      lastL = l;
      lastStoppedByXParent = stoppedByXParent;

      if (stoppedByXParent) {
        xEnd = cursor;
        break;
      }
    }

    // Determine if we keep y and the original X
    bool keepCase1 = false;
    if (xEnd < (int)cols.size()) {
      Column frc = firstRowColumn(z[0], n);
      keepCase1 = colGeq(cols[xEnd], frc);
    }

    bool keepCase2 = false;
    if (lastL >= 0 && xEnd > 0 && xEnd - 1 < (int)cols.size()) {
      if (cols[xEnd - 1][lastL] == 0 && ancestor.parentIsColumn(xEnd - 1, lastL, yIdx))
        keepCase2 = true;
    }

    bool keepCase3 =
        lastStoppedByXParent && lastL + 1 < n && xEnd > 0 && (xEnd - 1) < (int)cols.size() && cols[xEnd - 1][lastL + 1] > 0;

    bool keepOriginalYX = keepCase1 || keepCase2 || keepCase3;

    if (keepOriginalYX) {
      // Keep y and x_prime after x, original X remains
      cols.insert(cols.begin() + idx + 1, xPrime.begin(), xPrime.end());
    } else {
      // Replace y..X with x_prime only
      cols.erase(cols.begin() + idx + 1, cols.begin() + xEnd);
      cols.insert(cols.begin() + idx + 1, xPrime.begin(), xPrime.end());
    }

    idx--;
  }

  return cols;
}

// ============================================================
// Standard BMS → Triangular BMS
// ============================================================

Matrix bmsToTriangular(const Matrix &M) {
  Columns cols;
  for (auto &c : M)
    cols.push_back(c);
  if (cols.empty())
    return {};
  int n = (int)cols[0].size();
  if (n < 2)
    return {};

  int idx = 0;
  int steps = 0;
  const int stepLimit = 100000;

  while (idx < (int)cols.size()) {
    steps++;
    if (steps > stepLimit)
      return {};

    Column &x = cols[idx];
    int k = lastPositiveRow(x);
    if (k >= n - 1) {
      idx++;
      continue;
    }

    Column y = incrementPrefix(x, k + 1);
    Column z = incrementRow(y, k + 2);

    int xStart = idx + 1;
    if (xStart >= (int)cols.size() || colLess(cols[xStart], z)) {
      idx++;
      continue;
    }

    int xEnd = xStart;
    while (xEnd < (int)cols.size() && colGeq(cols[xEnd], z))
      xEnd++;

    AncestorIndex ancestor(cols);
    Columns xPrime;

    for (int cursor = xStart; cursor < xEnd; cursor++) {
      Column &t = cols[cursor];

      int l = -1;
      for (int row = 0; row <= k + 1; row++) {
        if (ancestor.hasAncestorColumn(cursor, row, idx))
          l = row;
      }
      if (l < 0)
        return {};

      bool isLast = (cursor == xEnd - 1);
      if (isLast) {
        if (l < 0 || l >= n)
          return {};
        if (ancestor.parentIsColumn(cursor, l, idx) && t[l] == 0)
          l--;
      }
      if (l < 0)
        return {};

      Column tPrime = incrementPrefix(t, l);
      xPrime.push_back(tPrime);
    }

    // comparison matrix: (y, ...x_prime, (y[0]+1,0,...,0))
    // remainder: columns starting from xEnd
    Columns insertion;
    insertion.push_back(y);
    insertion.insert(insertion.end(), xPrime.begin(), xPrime.end());

    Columns comparison = insertion;
    comparison.push_back(firstRowColumn(y[0] + 1, n));

    Columns remainder(cols.begin() + xEnd, cols.end());

    // Always delete X
    cols.erase(cols.begin() + xStart, cols.begin() + xEnd);

    // Insert if comparison > remainder (lexicographic tuple-of-tuples)
    if (seqCmp(comparison, remainder) > 0)
      cols.insert(cols.begin() + xStart, insertion.begin(), insertion.end());

    idx++;
  }

  return cols;
}
