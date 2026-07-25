#ifndef Y1_MOUNTAIN_H
#define Y1_MOUNTAIN_H

#include "ordinal.h"
#include <vector>

/// Build a pure 1-Y mountain diagram following the extraction algorithm.
/// Returns { layers, rowLabels } where each layer has (value, parentDist) pairs.
/// The algorithm computes successive difference sequences, extracts top elements,
/// and repeats until all top elements are 1.
std::pair<Mountain, std::vector<std::vector<int>>> build1YMountainWithRows(const std::vector<int> &seq);

/// Build just the mountain layers (no row labels)
Mountain build1YMountain(const std::vector<int> &seq);

#endif
