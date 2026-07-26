#include "ordinal.h"
// ============================================================
// 0-Y sequence to BMS matrix conversion
// ============================================================

Matrix zeroYToBMS(const std::vector<int> &seq) {
  int l = (int)seq.size();
  Matrix res(l);
  std::vector<int> parents(l);
  for (int i = 0; i < l; i++) {
    parents[i] = i - 1;
  }
  std::vector<int> cur(seq.begin(), seq.end());

  while (true) {
    std::vector<int> next(l);
    bool hasParent = false;

    for (int i = 0; i < l; i++) {
      int k = i;
      while (k >= 0 && cur[k] >= cur[i]) {
        k = parents[k];
      }
      parents[i] = k;
      if (k >= 0) {
        hasParent = true;
        next[i] = cur[i] - cur[k];
        int row = (int)res[i].size();
        res[i].push_back(res[k][row] + 1);
      } else {
        next[i] = 1;
        res[i].push_back(0);
      }
    }

    if (!hasParent)
      break;
    cur = std::move(next);
  }

  // Remove the last row (all-1s iteration), keeping at least one row per column
  for (int i = 0; i < l; i++) {
    if (res[i].size() > 1)
      res[i].pop_back();
  }

  // Pad to uniform length
  size_t maxRows = 0;
  for (int i = 0; i < l; i++) {
    if (res[i].size() > maxRows)
      maxRows = res[i].size();
  }
  for (int i = 0; i < l; i++) {
    while (res[i].size() < maxRows)
      res[i].push_back(0);
  }

  return res;
}

// ============================================================
// BMS matrix to 0-Y sequence conversion
// ============================================================

/// Convert a BMS matrix to its equivalent 0-Y sequence string.
/// Uses monotonic stack for O(rows × cols) parent-finding per row.
std::string bmsTo0YSequence(const Matrix &M) {
  int cols = (int)M.size();
  if (cols == 0)
    return "";
  int rows = (int)M[0].size();

  // Pad matrix to uniform row count
  Matrix S(cols);
  for (int i = 0; i < cols; i++) {
    S[i] = M[i];
    while ((int)S[i].size() < rows)
      S[i].push_back(0);
  }

  std::vector<int> result(cols, 1);

  // Process rows from bottom to top
  for (int row = rows - 1; row >= 0; row--) {
    std::vector<int> stack;
    stack.reserve(cols);
    for (int col = 0; col < cols; col++) {
      while (!stack.empty() && S[stack.back()][row] >= S[col][row]) {
        stack.pop_back();
      }
      if (!stack.empty()) {
        result[col] += result[stack.back()];
      }
      stack.push_back(col);
    }
  }

  // Format as comma-separated string
  std::string out;
  for (int i = 0; i < cols; i++) {
    if (i > 0)
      out += ",";
    out += std::to_string(result[i]);
  }
  return out;
}

// ============================================================
// 0-Y sequence expansion (Mt. Fuji algorithm)
// ============================================================

/// Build the mountain 2D structure from a 0-Y sequence
Mountain buildMountain(const std::vector<int> &seq) {
  int len = (int)seq.size();
  Mountain mountain;

  std::vector<std::pair<int, int>> bottom;
  for (int i = 0; i < len; i++)
    bottom.push_back({seq[i], 0});
  mountain.push_back(bottom);

  while (true) {
    auto &curLayer = mountain.back();
    bool hasParent = false;

    for (int x = 1; x < len; x++) {
      if (curLayer[x].second)
        continue;
      int p = x;
      while (p >= 0) {
        bool hasUpperParent = (mountain.size() == 1) || (mountain[mountain.size() - 2][p].second != 0);
        if (!hasUpperParent)
          break;
        if (curLayer[p].first < curLayer[x].first)
          break;
        p -= (mountain.size() == 1) ? 1 : mountain[mountain.size() - 2][p].second;
      }
      if (p >= 0) {
        int pVal = curLayer[p].first;
        if (pVal && pVal < curLayer[x].first) {
          curLayer[x].second = x - p;
          hasParent = true;
        }
      }
    }

    if (!hasParent)
      break;

    std::vector<std::pair<int, int>> nextLayer(len, {1, 0});
    for (int x = 1; x < len; x++) {
      if (curLayer[x].second) {
        int parentIdx = x - curLayer[x].second;
        nextLayer[x].first = curLayer[x].first - curLayer[parentIdx].first;
      }
    }
    mountain.push_back(nextLayer);
  }
  return mountain;
}

/// Expand a 0-Y sequence by n steps
std::vector<int> zeroYExpand(const std::vector<int> &seq, int n) {
  if (seq.empty())
    return {};
  Mountain mountain = buildMountain(seq);

  int height = (int)mountain.size();
  int cutPos = (int)mountain[0].size() - 1;

  // Find cut height: how many layers the last element has a parent
  int cutHeight = 0;
  while (cutHeight + 1 < height && mountain[cutHeight][cutPos].second)
    cutHeight++;

  if (cutHeight == 0) {
    // Last element is 0-height (no parent chain) — simply remove it
    std::vector<int> result = seq;
    result.pop_back();
    return result;
  }

  int badRootPos = cutPos - mountain[cutHeight - 1][cutPos].second;
  int badLen = cutPos - badRootPos;

  Mountain result = mountain;

  // Remove last column from all layers
  for (int y = 0; y < height; y++)
    result[y].pop_back();

  // Create Mt. Fuji shell (copy bad part with offset adjustments)
  for (int i = 1; i <= n; i++) {
    for (int x = badRootPos; x < cutPos; x++) {
      for (int y = 0; y < height; y++) {
        int origOffset = mountain[y][x].second;
        bool hasParent = (origOffset != 0);

        if (x == badRootPos && y < cutHeight - 1) {
          // First new column in this iteration: copy the cut's offset
          result[y].push_back({-1, mountain[y][cutPos].second});
        } else if (!hasParent) {
          // No parent: copy value as-is (no NaN needed)
          result[y].push_back({mountain[y][x].first, 0});
        } else if (hasParent && x - origOffset >= badRootPos && (x > badRootPos || y < cutHeight)) {
          // Parent is within the bad part: keep original offset
          result[y].push_back({-1, origOffset});
        } else {
          // Parent is outside: adjust offset by badLen * iteration
          result[y].push_back({-1, origOffset + badLen * i});
        }
      }
    }
  }

  // Recompute NaN values from bottom to top, left to right
  int resultLen = (int)result[0].size();
  for (int x = 0; x < resultLen; x++) {
    for (int y = height - 1; y >= 0; y--) {
      if (result[y][x].first == -1) {
        int offset = result[y][x].second;
        int parentIdx = x - offset;
        int upperVal = (y + 1 < height) ? result[y + 1][x].first : 0;
        int parentVal = (parentIdx >= 0) ? result[y][parentIdx].first : 0;
        result[y][x].first = upperVal + parentVal;
      }
    }
  }

  // Extract top row as the result sequence
  std::vector<int> out;
  for (int x = 0; x < resultLen; x++)
    out.push_back(result[0][x].first);
  return out;
}

/// Expand a 0-Y sequence string by n steps
std::string zeroYExpand(const std::string &seqStr, int n) {
  std::vector<int> seq;
  size_t pos = 0;
  while (pos < seqStr.size()) {
    size_t comma = seqStr.find(',', pos);
    if (comma == std::string::npos)
      comma = seqStr.size();
    seq.push_back(std::stoi(seqStr.substr(pos, comma - pos)));
    pos = comma + 1;
  }

  auto result = zeroYExpand(seq, n);

  std::string out;
  for (size_t i = 0; i < result.size(); i++) {
    if (i > 0)
      out += ",";
    out += std::to_string(result[i]);
  }
  return out;
}
