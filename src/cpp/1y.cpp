#include "1y.h"
#include <vector>

using Ord = std::vector<int>;

static Ord normalize(Ord a) {
  while (a.size() > 1 && a.back() == 0)
    a.pop_back();
  return a;
}

// Row ordinal: base at ext k = ω·k, diff depth d = ω·k + d
static Ord rowOrdinal(int ext, int diffDepth) {
  if (ext == 0 && diffDepth == 0)
    return {0};
  if (ext == 0)
    return {diffDepth};
  Ord ord(2, 0);
  ord[1] = ext;
  ord[0] = diffDepth;
  return normalize(ord);
}

// Definition (1): first smaller to left
static int findParent(const std::vector<int> &seq, int i) {
  for (int j = i - 1; j >= 0; j--)
    if (seq[j] < seq[i])
      return j;
  return -1;
}

// Check ancestor relationship in a parent array
static bool isAncestorOf(const std::vector<int> &parents, int col, int anc) {
  while (col >= 0) {
    if (col == anc)
      return true;
    col = parents[col];
  }
  return false;
}

std::pair<Mountain, std::vector<std::vector<int>>> build1YMountainWithRows(const std::vector<int> &seq) {
  Mountain result;
  std::vector<Ord> rowLabels;

  std::vector<int> cur = seq;
  std::vector<int> extractedParents; // def(6) parents from previous round

  for (int ext = 0;; ext++) {
    int n = (int)cur.size();

    // ── Base layer ──
    // Items = 1 have no parent by definition
    std::vector<int> baseParent(n, -1);
    if (ext == 0 || extractedParents.empty()) {
      // Original seq: definition (1)
      for (int i = 0; i < n; i++)
        if (cur[i] > 1)
          baseParent[i] = findParent(cur, i);
    } else {
      // Extracted seq: def(6) parents, items = 1 have no parent
      for (int i = 0; i < n; i++)
        if (cur[i] > 1)
          baseParent[i] = extractedParents[i];
    }

    std::vector<std::pair<int, int>> base(n);
    for (int i = 0; i < n; i++)
      base[i] = {cur[i], baseParent[i] >= 0 ? i - baseParent[i] : 0};
    result.push_back(base);
    rowLabels.push_back(rowOrdinal(ext, 0));
    int firstLayerIdx = (int)result.size() - 1;

    std::vector<std::vector<int>> parentInLayers;
    parentInLayers.push_back(baseParent);

    std::vector<int> prevVals = cur;

    // ── Inner loop: diff layers ──
    for (int diffDepth = 1;; diffDepth++) {
      std::vector<int> curDiffs(n, -1);
      std::vector<int> layerParents(n, -1);

      for (int i = 0; i < n; i++) {
        if (prevVals[i] <= 0)
          continue; // sentinel
        if (prevVals[i] == 1)
          continue; // =1 → no parent, no diff

        int p = -1;
        if (diffDepth == 1) {
          // First diff: diff value from def(1) parent, ancestry def(3)
          int baseP = baseParent[i];
          if (baseP >= 0) {
            int diffVal = prevVals[i] - prevVals[baseP];
            for (int j = i - 1; j >= 0; j--) {
              if (prevVals[j] <= 0 || baseParent[j] < 0)
                continue;
              int jDiffVal = prevVals[j] - prevVals[baseParent[j]];
              if (jDiffVal >= diffVal)
                continue;
              if (isAncestorOf(baseParent, i, j)) {
                p = j;
                break;
              }
            }
            if (p < 0)
              p = baseP;
            curDiffs[i] = prevVals[i] - prevVals[baseP];
            layerParents[i] = p;
          }
        } else {
          // L2+: def(3) for parent and diff value
          const auto &belowParents = parentInLayers.back();
          for (int j = i - 1; j >= 0; j--) {
            if (prevVals[j] <= 0)
              continue;
            if (prevVals[j] >= prevVals[i])
              continue;
            if (isAncestorOf(belowParents, i, j)) {
              p = j;
              break;
            }
          }
          if (p >= 0) {
            curDiffs[i] = prevVals[i] - prevVals[p];
            layerParents[i] = p;
          }
        }
      }

      // Check for non-sentinel values
      bool hasValue = false;
      for (int i = 0; i < n; i++)
        if (curDiffs[i] >= 0) {
          hasValue = true;
          break;
        }
      if (!hasValue)
        break;

      // Convergence: stop when ALL non-sentinel diffs = 1.
      bool allConverged = true;
      for (int i = 0; i < n; i++)
        if (curDiffs[i] >= 0 && curDiffs[i] != 1) {
          allConverged = false;
          break;
        }

      // Push diff layer with left-leg parent distances for display
      std::vector<std::pair<int, int>> diffLayer(n);
      for (int i = 0; i < n; i++) {
        if (curDiffs[i] < 0) {
          diffLayer[i] = {-1, -1};
          continue;
        }
        int legP = (diffDepth == 1) ? baseParent[i] : parentInLayers.back()[i];
        diffLayer[i] = {curDiffs[i], legP >= 0 ? i - legP : 0};
      }

      result.push_back(diffLayer);
      rowLabels.push_back(rowOrdinal(ext, diffDepth));
      parentInLayers.push_back(layerParents);

      if (allConverged)
        break;

      prevVals = curDiffs;
    }

    // ── Check topmost values ──
    std::vector<int> topmost(n, -1);
    for (int col = 0; col < n; col++)
      for (int L = (int)result.size() - 1; L >= firstLayerIdx; L--)
        if (result[L][col].first >= 0) {
          topmost[col] = result[L][col].first;
          break;
        }

    bool allOne = true;
    for (int i = 0; i < n; i++)
      if (topmost[i] >= 0 && topmost[i] != 1) {
        allOne = false;
        break;
      }
    if (allOne)
      break;

    // ── Extract: form the new base sequence ──
    std::vector<int> next(n);
    for (int col = 0; col < n; col++)
      for (int L = (int)result.size() - 1; L >= firstLayerIdx; L--)
        if (result[L][col].first >= 0) {
          next[col] = result[L][col].first;
          break;
        }

    if (next == cur)
      break;

    // Relative layer of each column's topmost (for def(6) traversal)
    std::vector<int> topLayerRel(n, 0);
    for (int col = 0; col < n; col++)
      for (int L = (int)result.size() - 1; L >= firstLayerIdx; L--)
        if (result[L][col].first >= 0) {
          topLayerRel[col] = L - firstLayerIdx;
          break;
        }

    // ── Definition (6): extraction parents (quasi-parents) ──
    // First, find the quasi-parent for each column > 1 via leg-walking
    std::vector<int> quasiParent(n, -1);
    for (int col = 1; col < n; col++) {
      if (next[col] <= 1)
        continue;

      int relLayer = topLayerRel[col];
      if (relLayer == 0)
        continue;

      int curCol = col;
      while (relLayer > 0) {
        const auto &belowParents = parentInLayers[relLayer - 1];
        int pCol = belowParents[curCol];
        if (pCol < 0)
          break;

        if (topLayerRel[pCol] == relLayer - 1) {
          quasiParent[col] = pCol;
          break;
        }

        curCol = pCol;
        if (topLayerRel[curCol] == relLayer) {
          quasiParent[col] = curCol;
          break;
        }
      }
    }

    // Now find the actual extraction parent: scan left from j-1 to 0
    // for the rightmost k where next[k] < next[j] AND k is a
    // quasi-ancestor of j (k is in the quasi-parent chain of j).
    extractedParents.assign(n, -1);
    for (int col = 1; col < n; col++) {
      if (next[col] <= 1)
        continue;

      // Build quasi-ancestor chain for this column
      std::vector<bool> inChain(n, false);
      int c = col;
      while (c >= 0) {
        inChain[c] = true;
        if (quasiParent[c] < 0)
          break;
        c = quasiParent[c];
      }

      // Scan left for rightmost k with smaller value AND in chain
      for (int k = col - 1; k >= 0; k--) {
        if (next[k] >= 0 && next[k] < next[col] && inChain[k]) {
          extractedParents[col] = k;
          break;
        }
      }
    }

    cur = next;
  }

  return {result, rowLabels};
}

Mountain build1YMountain(const std::vector<int> &seq) { return build1YMountainWithRows(seq).first; }
