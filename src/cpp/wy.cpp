#include "wy.h"
#include <algorithm>
#include <cmath>
#include <vector>

// ════════════════════════════════════════════════════════════════
// ω-base digit representation: ord(n) = [0,...,0,1] (ω^n)
// Little-endian: index 0 = ω^0, index 1 = ω^1, ...
// ════════════════════════════════════════════════════════════════

using Ord = std::vector<int>;

static Ord normalize(Ord a) {
  while (a.size() > 1 && a.back() == 0)
    a.pop_back();
  return a;
}

static Ord ordPlus(const Ord &a, const Ord &b) {
  size_t len = std::max(a.size(), b.size());
  Ord res(len, 0);
  int bLen = (int)b.size();
  for (int i = (int)len - 1; i >= 0; i--) {
    if (i >= bLen - 1 && i < (int)a.size()) {
      res[i] += a[i];
    }
    if (i <= bLen - 1) {
      res[i] += b[i];
    }
  }
  return res;
}

static Ord ordMinus(const Ord &a, const Ord &b) {
  if (a.size() < b.size())
    return {-1};
  bool borrow = true;
  Ord res(a.size(), 0);
  for (int i = (int)a.size() - 1; i >= 0; i--) {
    int bi = (i < (int)b.size()) ? b[i] : 0;
    if (borrow) {
      if (a[i] > bi) {
        borrow = false;
        res[i] = a[i] - bi;
      } else {
        res.pop_back();
      }
    } else {
      res[i] = a[i];
    }
  }
  return res;
}

static Ord ord(int n) {
  Ord res(n + 1, 0);
  res[n] = 1;
  return res;
}

static int ordCmp(const Ord &a, const Ord &b) {
  if (a.size() > b.size())
    return 1;
  for (int i = (int)b.size() - 1; i >= 0; i--) {
    int va = (i < (int)a.size()) ? a[i] : 0;
    int vb = b[i];
    if (vb > va)
      return -1;
    if (vb < va)
      return 1;
  }
  return 0;
}

static Ord ordMin(const Ord &a, const Ord &b) { return ordCmp(a, b) >= 0 ? b : a; }

static int ordLength(const Ord &a) {
  for (size_t i = 0; i < a.size(); i++) {
    if (a[i] != 0)
      return (int)i + 1;
  }
  return (int)a.size();
}

// ════════════════════════════════════════════════════════════════
// Mountain graph node
// ════════════════════════════════════════════════════════════════

struct Node {
  int value;
  int x; // column index
  Ord y; // row label
  Node *up = nullptr;
  Node *down = nullptr;
  Node *left = nullptr;
  std::vector<Node *> right;
  bool isMagma = false;

  Node(int value, int x, Ord y) : value(value), x(x), y(std::move(y)) {}
};

// ════════════════════════════════════════════════════════════════
// Graph helpers
// ════════════════════════════════════════════════════════════════

static void connectH(Node *n1, Node *n2) {
  n1->right.push_back(n2);
  n2->left = n1;
}

static void connectV(Node *n1, Node *n2) {
  if (n2->up)
    n2->up->down = n1;
  n1->down = n2;
  n2->up = n1;
}

static void magmaOp(Node *node) {
  for (auto *nd : node->right) {
    Node *target = nd;
    if (target && ordCmp(target->y, node->y) == 0) {
      target->isMagma = true;
      magmaOp(target);
    }
  }
}

static Node *findNode(Node *nd, const Ord &y, bool eq = true) {
  Node *x = nd;
  while (x->up && ordCmp(x->up->y, y) + (eq ? 0 : 1) <= 0) {
    x = x->up;
  }
  return x;
}

// ════════════════════════════════════════════════════════════════
// Mountain building
// ════════════════════════════════════════════════════════════════

static std::vector<Node *> generateMountain(const std::vector<int> &seq) {
  int len = (int)seq.size();
  std::vector<Node *> mt(len);
  for (int i = 0; i < len; i++) {
    auto *nd = new Node(seq[i], i, {0});
    auto *base = new Node(-1, i, {-1});
    mt[i] = nd;
    connectV(nd, base);
    if (i > 0) {
      connectH(mt[i - 1]->down, nd);
    }
  }
  return mt;
}

static std::vector<Node *> copyMountain(const std::vector<Node *> &seq) {
  int len = (int)seq.size();
  std::vector<Node *> mt(len);
  for (int i = 0; i < len; i++) {
    auto *nd = new Node(seq[i]->value, i, {0});
    auto *base = new Node(-1, i, {-1});
    mt[i] = nd;
    connectV(nd, base);
    if (i > 0) {
      // Find the correct parent (matching reference copyMountain)
      Node *parent = seq[i]->left;
      while (parent && parent->x > 0) {
        parent = findNode(parent, ordMin(seq[i]->y, seq[parent->x]->y));
        if (ordCmp(parent->y, seq[parent->x]->y) == 0)
          break;
        parent = parent->left;
      }
      connectH(mt[parent->x]->down, nd);
    }
  }
  return mt;
}

// ════════════════════════════════════════════════════════════════
// drawMountain — builds the deep mountain structure
// n = -1: ω-Y (full depth), n = 0: 0-Y, n = 1: 1-Y, etc.
// ════════════════════════════════════════════════════════════════

static std::vector<Node *> drawMountain(const std::vector<Node *> &seq, int n = -1, bool consistent = false) {
  auto mt = copyMountain(seq);
  int len = (int)seq.size();
  for (int i = 0; i < len; i++) {
    Node *nd1 = mt[i];
    while (true) {
      if (!nd1->left)
        break;
      bool flag = false;
      Node *p = nd1;
      while (p && p->value >= nd1->value) {
        p = p->left;
        while (p && p->up && ordCmp(p->up->y, nd1->y) <= 0) {
          p = p->up;
        }
      }
      if (!p) {
        if (consistent) {
          flag = true;
          p = nd1->left;
        } else
          break;
      }
      Ord diff = ordMinus(nd1->y, p->y);
      int dy = (int)diff.size();
      if (dy >= 1)
        flag = true;
      if (n >= 0) {
        dy = std::min(dy, n);
        if (n != 0 && dy >= n)
          break;
      }
      Ord newy = ordPlus(nd1->y, ord(dy));
      if (consistent && flag) {
        p = findNode(nd1->left, newy, false);
        auto *newNode = new Node(nd1->value, i, newy);
        connectH(p, newNode);
        connectV(newNode, nd1);
        break;
      }
      int newValue = nd1->value - p->value;
      auto *newNode = new Node(newValue, i, newy);
      connectH(p, newNode);
      connectV(newNode, nd1);
      nd1 = newNode;
    }
  }
  return mt;
}

// ════════════════════════════════════════════════════════════════
// w-Y mountain expansion
// ════════════════════════════════════════════════════════════════

static std::vector<Node *> expandwYMountain(std::vector<Node *> &seq, int fs, int n = -1,
                                            bool consistent = false, int depth = 0) {
  if (depth > 100)
    return seq;
  auto mt1 = drawMountain(seq, n, consistent);
  std::vector<Node *> mt2;

  if (seq.back()->value <= 1 || fs <= 0) {
    seq.pop_back();
    mt2 = drawMountain(seq, n);
    return mt2;
  }

  seq.back()->value -= 1;
  mt2 = drawMountain(seq, n, consistent);

  int len = (int)seq.size();
  std::vector<Node *> idx(len);
  Node *nd = nullptr;
  for (int i = 0; i < len; i++) {
    nd = mt1[i];
    while (nd->up)
      nd = nd->up;
    idx[i] = nd;
  }

  bool iterate = false;
  std::vector<Node *> diagonal, diagonal2;
  Node *top1 = nullptr;
  Node *root;
  int bl;

  if (n > 0) {
    diagonal = copyMountain(idx);
    if (idx.back()->value > 1 && !diagonal.empty()) {
      iterate = true;
      diagonal2 = expandwYMountain(diagonal, fs, n, consistent, depth + 1);
    }
  }

  if (iterate) {
    bl = (int)std::round((double)(diagonal2.size() - len + 1) / fs);
    if (bl > 0 && len - 1 - bl >= 0) {
      root = idx[len - 1 - bl];
      top1 = new Node(1, len - 1, ord(n));
    } else {
      iterate = false;
    }
  }

  if (!iterate) {
    auto *xd = mt2[idx.back()->x];
    while (xd->up)
      xd = xd->up;
    idx.back() = xd;
    root = nd->left;
    top1 = nd;
  }

  bl = len - 1 - root->x;
  std::vector<Node *> rc;
  nd = mt2[root->x]->down;
  while (nd && ordCmp(root->y, nd->y) >= 0) {
    magmaOp(nd);
    rc.push_back(nd);
    if (!nd->up)
      break;
    nd = nd->up;
  }
  rc.push_back(top1);

  for (int i = 0; i < fs; i++) {
    int dis = (i + 1) * bl;
    auto *nd2 = mt2.back()->down;
    int ir = 1;
    Ord yr = rc[ir]->y;
    std::vector<Node *> ref(rc.size() - 1, nullptr);

    while (nd2) {
      if (!nd2->up || ordCmp(nd2->up->y, yr) >= 0) {
        ref[ir - 1] = nd2;
        ir++;
        if (ir >= (int)rc.size())
          break;
        yr = rc[ir]->y;
      }
      nd2 = nd2->up;
    }

    std::vector<Node *> tops(bl), roots(bl);
    for (int j = 0; j < bl; j++) {
      tops[j] = new Node(-1, (int)mt2.size(), {-1});
      roots[j] = new Node(-1, (int)mt2.size(), {-1});
      mt2.push_back(new Node(-2, (int)mt2.size(), {0}));
    }

    for (int j = 0; j < bl; j++) {
      if (i == fs - 1 && j == bl - 1)
        break;
      auto *nd3 = mt2[root->x + j + 1];
      int ir2 = 0;
      auto *thisRef = ref[ir2];
      Node *newLeft = nullptr; // shared variable like reference's global newLeft

      while (nd3) {
        if (nd3->isMagma) {
          ir2++;
          if (ir2 < (int)ref.size())
            thisRef = ref[ir2];

          // wildfire edge
          auto *thisNode = nd3->left;
          newLeft = findNode(mt2[thisNode->x + dis], nd3->y, false);
          auto *newRight = new Node(-1, nd3->x + dis, nd3->y);
          connectH(newLeft, newRight);
          connectV(newRight, tops[j]);
          tops[j] = newRight;
          if (ordCmp(newRight->y, {0}) == 0)
            mt2[nd3->x + dis] = newRight;

          // magma edge
          auto *magmaNode = (n == 1) ? (nd3->up ? nd3->up->left : nd3->left) : nd3->left;
          auto *newLeft2 = findNode(mt2[magmaNode->x + dis], nd3->y);
          while (newLeft2->up && ordCmp(newLeft2->y, thisRef->y) < 0) {
            int dyLen = (int)ordMinus(newLeft2->up->y, newLeft2->y).size();
            for (int k = 0; k < dyLen; k++) {
              auto *newRight2 = new Node(-1, nd3->x + dis, ordPlus(newLeft2->y, ord(k)));
              connectH(newLeft2, newRight2);
              connectV(newRight2, tops[j]);
              tops[j] = newRight2;
              if (ordCmp(newRight2->y, {0}) == 0)
                mt2[nd3->x + dis] = newRight2;
            }
            newLeft2 = newLeft2->up;
          }
        } else {
          // eruption edge
          auto *thisNode = nd3->left;
          auto dy = ordMinus(nd3->y, rc[ir2]->y);
          auto *newRight = new Node(-1, nd3->x + dis, ordPlus(thisRef->y, dy));

          if (thisNode->x < root->x) {
            newLeft = thisNode;
          } else {
            newLeft = findNode(mt2[thisNode->x + dis], newRight->y, false);
          }
          connectH(newLeft, newRight);
          connectV(newRight, tops[j]);
          tops[j] = newRight;
          if (ordCmp(newRight->y, {0}) == 0)
            mt2[nd3->x + dis] = newRight;
        }
        if (!nd3->up)
          break;
        nd3 = nd3->up;
      }

      auto *xd = tops[j];
      if (iterate) {
        xd->value = diagonal2[xd->x]->value;
      } else {
        xd->value = idx[root->x + j + 1]->value;
      }
      while (ordCmp(xd->y, {0}) > 0) {
        xd->down->value = (consistent && xd->y[0] == 0) ? xd->value : xd->value + xd->left->value;
        xd = xd->down;
      }
    }
  }

  mt2.pop_back();
  return mt2;
}

static std::vector<int> expandwY(const std::vector<int> &seq, int fs, int n, bool consistent = false) {
  auto mt = generateMountain(seq);
  auto result = expandwYMountain(mt, fs, n, consistent);
  std::vector<int> values;
  for (auto *node : result)
    values.push_back(node->value);
  for (auto *node : result)
    delete node;
  return values;
}

// ════════════════════════════════════════════════════════════════
// buildWYMountain — returns layered mountain for display
// Uses same logic as reference wy.js reorganizeMountain
// ════════════════════════════════════════════════════════════════

Mountain buildWYMountain(const std::vector<int> &seq, int n, bool consistent) {
  return buildWYMountainWithRows(seq, n, consistent).first;
}

std::pair<Mountain, std::vector<Ord>> buildWYMountainWithRows(const std::vector<int> &seq, int n, bool consistent) {
  auto mt = generateMountain(seq);
  auto drawn = drawMountain(mt, n, consistent);

  int len = (int)drawn.size();
  std::vector<Node *> idx(len);
  for (int i = 0; i < len; i++)
    idx[i] = drawn[i];

  Mountain result;
  std::vector<Ord> rowLabels;
  Ord row = {0};
  bool running = true;

  while (running) {
    running = false;
    Ord row0;
    bool hasRow0 = false;
    std::vector<std::pair<int, int>> layer(len, {0, 0});

    for (int i = 0; i < len; i++) {
      if (!idx[i] || ordCmp(row, idx[i]->y) < 0) {
        layer[i] = {-1, -1};
      } else {
        running = true;
        int parentCol = (idx[i]->left) ? idx[i]->left->x : -1;
        int parentDist = (parentCol >= 0) ? (i - parentCol) : 0;
        layer[i] = {idx[i]->value, parentDist};
        idx[i] = idx[i]->up;
      }
      if (idx[i]) {
        if (!hasRow0 || ordCmp(idx[i]->y, row0) < 0) {
          row0 = idx[i]->y;
          hasRow0 = true;
        }
      }
    }

    if (running) {
      result.push_back(layer);
      rowLabels.push_back(row);
      if (hasRow0) {
        row = row0;
      } else {
        row = ordPlus(row, {1});
      }
    }
  }

  for (auto *node : drawn)
    delete node;
  for (auto *node : mt)
    delete node;
  return {result, rowLabels};
}

// Public API
// ════════════════════════════════════════════════════════════════

std::vector<int> expand1Y(const std::vector<int> &seq, int fs) { return expandwY(seq, fs, 1); }

std::vector<int> expandWY(const std::vector<int> &seq, int fs) { return expandwY(seq, fs, -1); }

std::vector<int> expandNY(const std::vector<int> &seq, int fs, int n) {
  if (n >= 0)
    return expandwY(seq, fs, n);
  return expandwY(seq, fs, -1);
}
