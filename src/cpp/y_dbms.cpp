#include "ordinal.h"
#include "wy.h"
#include <algorithm>
#include <sstream>
#include <string>
#include <vector>

// ============================================================
// 1-Y / ω-Y sequence ↔ DBMS (Dimensional BMS) conversion
//
// Below [1,3]: DBMS = 0-Y → BMS with trailing zeros stripped.
// [1,3]+:      DBMS derived from ω-Y mountain with ω-boundary markers.
// ============================================================

static std::vector<int> parseSeqStr(const std::string &s) {
  std::vector<int> seq;
  std::stringstream ss(s);
  std::string item;
  while (std::getline(ss, item, ','))
    seq.push_back(std::stoi(item));
  return seq;
}

/// Check if row label transition is an ω-boundary
static bool isOmegaBoundary(const std::vector<int> &a, const std::vector<int> &b) {
  if (a.empty() || b.empty())
    return false;
  if (b.size() >= 2 && b[0] == 0) {
    int extA = (a.size() >= 2) ? a[1] : 0;
    int extB = b[1];
    return extB > extA;
  }
  if (b.size() > a.size() + 1)
    return true;
  if (b.size() > a.size() && b.back() > 0)
    return true;
  return false;
}

/// Count ω-boundaries in the ω-Y mountain row labels
static int countOmegaBoundaries(const std::vector<std::vector<int>> &wyRows) {
  int count = 0;
  for (size_t i = 1; i < wyRows.size(); i++)
    if (isOmegaBoundary(wyRows[i - 1], wyRows[i]))
      count++;
  return count;
}

/// Convert a 1-Y sequence to DBMS matrix.
/// Below [1,3]: uses zeroYToBMS.  [1,3]+: returns fixed placeholder ≥(0)(1)(2,1,,1).
Matrix oneYToDBMS(const std::vector<int> &seq) {
  if (seq.empty() || seq[0] == 0)
    return {};
  for (int v : seq)
    if (v < 0)
      return {};

  // Check for ω-boundaries using ω-Y mountain
  auto [mountain, wyRows] = buildWYMountainWithRows(seq, -1, false);
  if (mountain.empty())
    return {};
  if (countOmegaBoundaries(wyRows) > 0) {
    // No stable algorithm for [1,3]+ yet; return fixed placeholder
    Matrix placeholder;
    placeholder.push_back({0});           // (0)
    placeholder.push_back({1});           // (1)
    placeholder.push_back({2, 1, -2, 1}); // (2,1,,1)
    return placeholder;
  }

  // Below [1,3]: use 0-Y → BMS conversion
  Matrix bms = zeroYToBMS(seq);
  if (bms.empty())
    return {};
  for (auto &col : bms)
    while (col.size() > 1 && col.back() == 0)
      col.pop_back();
  return bms;
}

/// Convert a DBMS matrix to a 1-Y sequence.
std::vector<int> dbmsToOneY(const Matrix &dbms) {
  if (dbms.empty() || dbms[0].empty())
    return {};

  bool hasMarker = false;
  for (auto &col : dbms)
    for (int v : col)
      if (v == -2) {
        hasMarker = true;
        break;
      }

  if (hasMarker)
    return {};

  std::string seqStr = bmsTo0YSequence(dbms);
  if (seqStr.empty())
    return {};
  return parseSeqStr(seqStr);
}

/// Format a DBMS matrix as a readable string like (0)(1)(2,1)
std::string dbmsToString(const Matrix &dbms) {
  std::string out;
  for (size_t j = 0; j < dbms.size(); j++) {
    out += "(";
    bool first = true;
    for (size_t r = 0; r < dbms[j].size(); r++) {
      if (dbms[j][r] == -2) {
        out += ",,";
        first = true;
        continue;
      }
      if (!first)
        out += ",";
      out += std::to_string(dbms[j][r]);
      first = false;
    }
    out += ")";
  }
  return out;
}

/// Convert DBMS to standard BMS. DBMS (without ω-boundary markers)
/// is triangular BMS with variable-length columns. The conversion
/// pads columns to uniform length and then calls triangularToBMS.
Matrix dbmsToBMS(const Matrix &dbms) {
  for (auto &col : dbms)
    for (int v : col)
      if (v == -2)
        return {};
  size_t maxLen = 0;
  for (auto &col : dbms)
    if (col.size() > maxLen)
      maxLen = col.size();
  if (maxLen == 0)
    return {};
  size_t target = std::max(maxLen, size_t(2));
  Matrix padded;
  for (auto &col : dbms) {
    MatrixRow r = col;
    r.resize(target, 0);
    padded.push_back(std::move(r));
  }
  return triangularToBMS(padded);
}
