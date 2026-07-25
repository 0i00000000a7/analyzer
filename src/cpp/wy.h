#ifndef WY_EXPAND_H
#define WY_EXPAND_H

#include "ordinal.h"
#include <string>
#include <vector>

/// Build a WY mountain for display (n = 0 → 0-Y style, n = 1 → 1-Y, n = -1 → ω-Y)
Mountain buildWYMountain(const std::vector<int> &seq, int n, bool consistent = false);

/// Build a WY mountain and also return the row label (ordinal) for each layer
std::pair<Mountain, std::vector<std::vector<int>>> buildWYMountainWithRows(const std::vector<int> &seq, int n,
                                                                           bool consistent = false);

/// 1-Y expansion by fs steps
std::vector<int> expand1Y(const std::vector<int> &seq, int fs);

/// ω-Y expansion by fs steps
std::vector<int> expandWY(const std::vector<int> &seq, int fs);

/// n-Y expansion by fs steps (n >= 0, n=0 → 0-Y, n=1 → 1-Y, n=-1 → ω-Y)
std::vector<int> expandNY(const std::vector<int> &seq, int fs, int n);

#endif
