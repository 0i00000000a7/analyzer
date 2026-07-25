#include "ordinal.h"

// ============================================================
// BMS matrix expansion (BMS → 0-Y style expand)
// ============================================================

Matrix expandBMS(const Matrix &M, int fs) {
  int l = (int)M.size();
  if (l == 0)
    return {};

  // Determine row count
  int rows = 0;
  for (int i = 0; i < l; i++) {
    if ((int)M[i].size() > rows)
      rows = (int)M[i].size();
  }

  // Build uniform matrix (pad with 0)
  Matrix S(l);
  for (int i = 0; i < l; i++) {
    S[i] = M[i];
    while ((int)S[i].size() < rows)
      S[i].push_back(0);
  }

  // Calculate parent matrix: parents[col][row]
  std::vector<std::vector<int>> parents(l);
  for (int row = 0; row < rows; row++) {
    if (row == 0) {
      std::vector<int> stack;
      stack.reserve(l);
      for (int col = 0; col < l; col++) {
        while (!stack.empty() && S[stack.back()][0] >= S[col][0]) {
          stack.pop_back();
        }
        parents[col].push_back(stack.empty() ? -1 : stack.back());
        stack.push_back(col);
      }
    } else {
      for (int col = 0; col < l; col++) {
        int k = col;
        while (k >= 0 && S[k][row] >= S[col][row]) {
          k = parents[k][row - 1];
        }
        parents[col].push_back(k);
      }
    }
  }

  // Find the highest non-zero row in the last column
  int x = -1;
  while (x + 1 < rows && S[l - 1][x + 1] > 0) {
    x++;
  }

  // Not a limit ordinal — just remove the last column
  Matrix res;
  if (x < 0) {
    for (int i = 0; i < l - 1; i++)
      res.push_back(S[i]);
    return res;
  }

  int badRoot = parents[l - 1][x];

  // If bad root is -1 at row x, fall back to row 0
  if (badRoot < 0) {
    badRoot = parents[l - 1][0];
    if (badRoot < 0) {
      // Still no parent — just remove the last column
      Matrix res;
      for (int i = 0; i < l - 1; i++)
        res.push_back(S[i]);
      return res;
    }
  }

  int badLength = l - 1 - badRoot;

  // Ascension values for rows below x
  std::vector<int> ascValue(rows, 0);
  for (int i = 0; i < x; i++) {
    ascValue[i] = S[l - 1][i] - S[badRoot][i];
  }

  // Ascension matrix
  std::vector<std::vector<int>> ascMat(badLength, std::vector<int>(rows, 0));
  for (int i = 0; i < x; i++) {
    for (int j = 0; j < badLength; j++) {
      int k = j + badRoot;
      while (k > badRoot) {
        k = parents[k][i];
      }
      ascMat[j][i] = (k == badRoot ? 1 : 0);
    }
  }

  // Build result: keep all columns except the last
  for (int i = 0; i < l - 1; i++) {
    res.push_back(S[i]);
  }

  // Expand: repeat bad part with ascension
  for (int step = 1; step <= fs; step++) {
    for (int j = badRoot; j < l - 1; j++) {
      MatrixRow col(rows);
      for (int k = 0; k < rows; k++) {
        col[k] = S[j][k] + ascValue[k] * step * ascMat[j - badRoot][k];
      }
      res.push_back(col);
    }
  }

  return res;
}

// ============================================================
// Lexicographic order on matrices
// ============================================================

int matrixLexOrder(const Matrix &a, const Matrix &b) {
  size_t maxRows = std::max(a.size(), b.size());
  for (size_t i = 0; i < maxRows; i++) {
    size_t maxCols = std::max(a.size() > i ? a[i].size() : 0, b.size() > i ? b[i].size() : 0);
    for (size_t j = 0; j < maxCols; j++) {
      int va = (i < a.size() && j < a[i].size()) ? a[i][j] : 0;
      int vb = (i < b.size() && j < b[i].size()) ? b[i][j] : 0;
      if (va > vb)
        return 1;
      if (va < vb)
        return -1;
    }
  }
  return 0;
}
