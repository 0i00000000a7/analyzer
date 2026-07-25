#!/usr/bin/env bash
set -eu

# Build the BMS analyzer CLI for a given target platform.
# Usage: ./scripts/build-cli.sh [target]
#   target      Platform identifier (default: native)
#               native            - local build with -march=native
#               linux-x86_64      - Linux x86_64 (portable)
#               linux-aarch64     - Linux AArch64 (portable)
#               macos-x86_64      - macOS x86_64 (portable)
#               macos-arm64       - macOS arm64 (portable)
#               windows-x86_64    - Windows x86_64 (portable)

TARGET="${1:-native}"
SRC_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CC="${CXX:-g++}"
SRC_FILES=(
  src/cli/main.cpp
  src/cpp/ordinal.cpp
  src/cpp/bms_analysis.cpp
  src/cpp/zero_y.cpp
  src/cpp/bms_expand.cpp
  src/cpp/triangular.cpp
  src/cpp/wy.cpp
  src/cpp/1y.cpp
  src/cpp/y_dbms.cpp
  src/cpp/parser.cpp
)

case "$TARGET" in
  native)   OUTPUT="analyzer-cli";;
  windows-x86_64) OUTPUT="analyzer-cli-windows-x86_64.exe";;
  *)        OUTPUT="analyzer-cli-${TARGET}";;
esac

MARCH=""
[ "$TARGET" = "native" ] && MARCH="-march=native"

cd "$SRC_DIR"
set -x
"$CC" -std=c++20 -O3 $MARCH -flto -fomit-frame-pointer -I src/cpp \
  -o "$OUTPUT" \
  "${SRC_FILES[@]}"
