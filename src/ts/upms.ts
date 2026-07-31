/**
 * UPMS (Unupgrading Projection Matrix System) — expansion and BMS↔UPMS conversion.
 *
 * Column-major layout: matrix[colIndex][rowIndex].
 * Matches the reference UPMS.ts and the Python bms_upms_squeeze_converter.py.
 */

export type Column = number[];
export type Expr = Column[];

// ════════════════════════════════════════════════════════════════
// Parsing & formatting
// ════════════════════════════════════════════════════════════════

export function parseUPMS(input: string): Expr {
  if (!input || !input.trim()) return [];
  const result: Expr = [];
  
  // Handle BMS-style input like "(0)(1)" by inserting spaces between )( 
  let normalized = input.trim();
  if (normalized.includes('(') && normalized.includes(')')) {
    normalized = normalized.replace(/\)\(/g, ') (');
  }
  
  for (const raw of normalized.split(/\s+/)) {
    const token = raw.trim();
    if (!token) continue;
    
    let parts: number[];
    if (token.startsWith('(') && token.endsWith(')')) {
      parts = token
        .slice(1, -1)
        .split(',')
        .map((s) => parseInt(s.trim()));
    } else if (/^\d+$/.test(token)) {
      parts = token.split('').map((c) => parseInt(c));
    } else {
      throw new Error(`Invalid UPMS column: ${token}`);
    }
    if (parts.length > 3) {
      throw new Error(`Column ${token} has more than 3 entries`);
    }
    while (parts.length < 3) parts.push(0);
    result.push(parts);
  }
  return result;
}

export function formatUPMS(matrix: Expr): string {
  return matrix.map(formatColumn).join(' ');
}

function formatColumn(col: Column): string {
  const values = [...col];
  while (values.length > 1 && values[values.length - 1] === 0) values.pop();
  if (values.some((v) => v < 0 || v > 9)) {
    return '(' + col.join(',') + ')';
  }
  return values.join('');
}

// ════════════════════════════════════════════════════════════════
// Matrix utilities
// ════════════════════════════════════════════════════════════════

function cloneColumn(col: Column): Column {
  return col.slice();
}

function standardizeMatrix(matrix: Expr): Expr {
  if (!Array.isArray(matrix) || matrix.length === 0) return [];
  let rows = 1;
  for (const col of matrix) {
    if (!Array.isArray(col)) return [];
    rows = Math.max(rows, col.length);
  }
  const result = matrix.map((col) => {
    const out = col.slice();
    while (out.length < rows) out.push(0);
    return out;
  });
  while (rows > 1 && result.every((col) => col[rows - 1] === 0)) {
    result.forEach((col) => col.pop());
    rows--;
  }
  return result;
}

function matrixCompare(m1: Expr, m2: Expr): number {
  const a = standardizeMatrix(m1);
  const b = standardizeMatrix(m2);
  const len = Math.max(a.length, b.length);
  for (let c = 0; c < len; c++) {
    if (c >= a.length) return -1;
    if (c >= b.length) return 1;
    const cmp = sequenceCompare(a[c], b[c]);
    if (cmp !== 0) return cmp;
  }
  return 0;
}

function sequenceCompare(s1: number[], s2: number[]): number {
  const len = Math.max(s1.length, s2.length);
  for (let i = 0; i < len; i++) {
    const a = i < s1.length ? s1[i] : 0;
    const b = i < s2.length ? s2[i] : 0;
    if (a < b) return -1;
    if (a > b) return 1;
  }
  return 0;
}

// ════════════════════════════════════════════════════════════════
// Validation
// ════════════════════════════════════════════════════════════════

export function isLegalUPMSMatrix(matrix: Expr): boolean {
  if (!Array.isArray(matrix)) return false;
  if (matrix.length === 0) return true;
  for (const col of matrix) {
    if (!Array.isArray(col)) return false;
    for (const v of col) {
      if (!Number.isInteger(v) || v < 0 || !Number.isFinite(v)) return false;
    }
  }
  const m = standardizeMatrix(matrix);
  if (m.length === 0) return true;
  const rows = m[0].length;
  for (let r = 0; r < rows; r++) {
    if (m[0][r] !== 0) return false;
  }
  for (let c = 0; c < m.length; c++) {
    const col = m[c];
    for (let r = 1; r < rows; r++) {
      if (col[r] > col[r - 1]) return false;
    }
  }
  return true;
}

// ════════════════════════════════════════════════════════════════
// UPMS expansion (from reference UPMS.ts)
// ════════════════════════════════════════════════════════════════

interface Context {
  m: Expr;
  colCount: number;
  rowCount: number;
  getBParent: (colIndex: number, b: number) => number;
  getAAncestors: (colIndex: number, a: number) => { list: number[]; mask: Uint8Array };
}

function makeContext(matrix: Expr): Context {
  const m = standardizeMatrix(matrix);
  const colCount = m.length;
  const rowCount = colCount === 0 ? 0 : m[0].length;
  const parentCache: number[][] = Array.from({ length: rowCount + 1 }, () =>
    Array(colCount).fill(-2),
  );
  const ancestorCache: ({ list: number[]; mask: Uint8Array } | null)[][] = Array.from(
    { length: rowCount + 1 },
    () => Array(colCount).fill(null),
  );

  const getZeroParent = (colIndex: number) => (colIndex > 0 ? colIndex - 1 : -1);

  const getAAncestors = (colIndex: number, a: number) => {
    if (a < 0 || a > rowCount || colIndex < 0 || colIndex >= colCount)
      return { list: [], mask: new Uint8Array(colCount) };
    const cached = ancestorCache[a][colIndex];
    if (cached !== null) return cached;
    const list: number[] = [];
    const mask = new Uint8Array(colCount);
    let current = colIndex;
    let guard = 0;
    while (current !== -1 && !mask[current] && guard++ <= colCount + 2) {
      list.push(current);
      mask[current] = 1;
      current = a === 0 ? getZeroParent(current) : getBParent(current, a);
    }
    const result = { list, mask };
    ancestorCache[a][colIndex] = result;
    return result;
  };

  const getBParent = (colIndex: number, b: number) => {
    if (b < 1 || b > rowCount || colIndex < 0 || colIndex >= colCount) return -1;
    const cached = parentCache[b][colIndex];
    if (cached !== -2) return cached;
    const row = b - 1;
    const value = m[colIndex][row];
    const ancestors = getAAncestors(colIndex, b - 1).list;
    let best = -1;
    for (let i = 0; i < ancestors.length; i++) {
      const candidate = ancestors[i];
      if (candidate >= colIndex) continue;
      if (m[candidate][row] < value) {
        best = candidate;
        break;
      }
    }
    parentCache[b][colIndex] = best;
    return best;
  };

  return { m, colCount, rowCount, getBParent, getAAncestors };
}

function lastColumnIsZero(matrix: Expr): boolean {
  if (matrix.length === 0) return true;
  const last = matrix[matrix.length - 1];
  for (let r = 0; r < last.length; r++) {
    if (last[r] !== 0) return false;
  }
  return true;
}

function findLastNonZeroRowLabel(matrix: Expr): number {
  if (matrix.length === 0) return -1;
  const last = matrix[matrix.length - 1];
  for (let r = last.length - 1; r >= 0; r--) {
    if (last[r] !== 0) return r + 1;
  }
  return -1;
}

function findBadRoot(ctx: Context) {
  const lastCol = ctx.colCount - 1;
  const t = findLastNonZeroRowLabel(ctx.m);
  if (t === -1) return null;
  const rootCol = ctx.getBParent(lastCol, t);
  if (rootCol === -1) return null;
  return { rootCol, t };
}

function computeDelta(ctx: Context, rootCol: number, t: number): number[] {
  const lastCol = ctx.colCount - 1;
  const delta = new Array(ctx.rowCount);
  for (let r = 0; r < ctx.rowCount; r++)
    delta[r] = r >= t - 1 ? 0 : ctx.m[lastCol][r] - ctx.m[rootCol][r];
  return delta;
}

function maxEntry(matrix: Expr): number {
  let max = 0;
  for (const col of matrix) {
    for (const v of col) {
      if (v > max) max = v;
    }
  }
  return max;
}

function computeUPMSVerificationRoots(ctx: Context, rootCol: number, t: number) {
  const m = ctx.m;
  const alpha = ctx.colCount - 1;
  const y = rootCol;
  const height = ctx.rowCount;
  const maxTwice = maxEntry(m) * 2;
  const vr = new Int8Array(ctx.colCount * height);
  vr.fill(-1);
  const vrIndex = (col: number, row: number) => col * height + row;
  const inBadPart = (col: number, row: number) => col >= y && col < alpha && row < t - 1;
  const getVR = (col: number, row: number) => (inBadPart(col, row) ? vr[vrIndex(col, row)] : -1);
  const setVR = (col: number, row: number, value: number) => {
    vr[vrIndex(col, row)] = value;
  };

  const baseValue = (col: number, k: number, r: number) => m[col][r] + (r < k ? 1 : 0);

  const columnLessThanBase = (candidate: number, col: number, k: number) => {
    const limit = k + 1;
    for (let r = 0; r < limit; r++) {
      const a = r < height ? m[candidate][r] : 0;
      const b = baseValue(col, k, r);
      if (a < b) return true;
      if (a > b) return false;
    }
    return false;
  };

  const transformedXValue = (sourceCol: number, row: number, iCol: number, k: number) => {
    let value = m[sourceCol][row];
    if (row < k - 1 && getVR(sourceCol, row) === 1) value += maxTwice - m[iCol][row];
    return value;
  };

  const transformedYValue = (sourceCol: number, row: number, jCol: number, k: number) => {
    let value = m[sourceCol][row];
    if (row < k - 1) {
      const colIsJ = sourceCol === jCol;
      const containsJ = ctx.getAAncestors(sourceCol, row + 1).mask[jCol] === 1;
      if (colIsJ || containsJ) value += maxTwice - m[jCol][row];
    }
    return value;
  };

  const compareTransformedParts = (
    xStart: number,
    xEnd: number,
    yStart: number,
    jCol: number,
    iCol: number,
    k: number,
  ) => {
    const xLen = xEnd - xStart + 1;
    const yLen = alpha - yStart + 1;
    const commonCols = Math.min(xLen, yLen);
    for (let local = 0; local < commonCols; local++) {
      const xCol = xStart + local;
      const yCol = yStart + local;
      for (let row = 0; row < height; row++) {
        const xv = transformedXValue(xCol, row, iCol, k);
        const yv = transformedYValue(yCol, row, jCol, k);
        if (xv < yv) return -1;
        if (xv > yv) return 1;
      }
    }
    if (xLen < yLen) return -1;
    if (xLen > yLen) return 1;
    return 0;
  };

  for (let row = 0; row < t - 1; row++) {
    const k = row + 1;
    for (let col = y; col < alpha; col++) {
      if (col === y || row === 0) {
        setVR(col, row, 1);
        continue;
      }
      const kAncestors = ctx.getAAncestors(col, k);
      let ancestorHasVR0 = false;
      for (let a = 0; a < kAncestors.list.length; a++) {
        if (getVR(kAncestors.list[a], row) === 0) {
          ancestorHasVR0 = true;
          break;
        }
      }
      const kParent = ctx.getBParent(col, k);
      if (kAncestors.mask[y] !== 1 || ancestorHasVR0 || kParent === -1) {
        setVR(col, row, 0);
        continue;
      }
      if (kParent !== y) {
        setVR(col, row, 1);
        continue;
      }
      let earlierRowHasVR0 = false;
      for (let wRow = 0; wRow < row; wRow++) {
        if (getVR(col, wRow) === 0) {
          earlierRowHasVR0 = true;
          break;
        }
      }
      if (earlierRowHasVR0) {
        setVR(col, row, 0);
        continue;
      }
      let higherParentEscapesBadRoot = false;
      for (let vRow = row + 1; vRow < t - 1; vRow++) {
        if (ctx.getBParent(col, vRow + 1) !== y) {
          higherParentEscapesBadRoot = true;
          break;
        }
      }
      if (higherParentEscapesBadRoot) {
        setVR(col, row, 0);
        continue;
      }
      let u = -1;
      for (let candidate = col + 1; candidate <= alpha; candidate++) {
        if (columnLessThanBase(candidate, col, k)) {
          u = candidate;
          break;
        }
      }
      if (u === -1) {
        setVR(col, row, 1);
        continue;
      }
      const Ayk = m[y][row];
      const alphaAncestors = ctx.getAAncestors(alpha, k).list;
      let j = -1;
      for (let a = 0; a < alphaAncestors.length; a++) {
        if (m[alphaAncestors[a]][row] === Ayk + 1) {
          j = alphaAncestors[a];
          break;
        }
      }
      if (j === -1) j = alpha;
      const cmp = compareTransformedParts(col, u - 1, j, j, col, k);
      setVR(col, row, cmp < 0 ? 0 : 1);
    }
  }
  return { data: vr, index: vrIndex, height };
}

function generateBh(
  ctx: Context,
  B: Expr,
  delta: number[],
  t: number,
  h: number,
  rootCol: number,
  vr: { data: Int8Array; index: (col: number, row: number) => number; height: number },
): Expr {
  return B.map((col, localCol) => {
    const originalCol = rootCol + localCol;
    const next = new Array(ctx.rowCount);
    for (let r = 0; r < ctx.rowCount; r++) {
      const hasVR = r < t - 1 && vr.data[vr.index(originalCol, r)] === 1;
      next[r] = col[r] + h * delta[r] * (hasVR ? 1 : 0);
    }
    return next;
  });
}

export function expandUPMS(matrix: Expr, index: number): Expr {
  if (!isLegalUPMSMatrix(matrix)) return [];
  const ctx = makeContext(matrix);
  const m = ctx.m;
  const n = Math.max(0, Math.floor(index));
  if (m.length === 0) return [];
  if (lastColumnIsZero(m)) return standardizeMatrix(m.slice(0, -1).map(cloneColumn));
  const badRoot = findBadRoot(ctx);
  if (badRoot === null) return [];
  const { rootCol, t } = badRoot;
  const G = m.slice(0, rootCol).map(cloneColumn);
  const B = m.slice(rootCol, ctx.colCount - 1).map(cloneColumn);
  const delta = computeDelta(ctx, rootCol, t);
  const vr = computeUPMSVerificationRoots(ctx, rootCol, t);
  const result: Expr = [...G, ...B.map(cloneColumn)];
  for (let h = 1; h <= n; h++) {
    const Bh = generateBh(ctx, B, delta, t, h, rootCol, vr);
    for (let i = 0; i < Bh.length; i++) result.push(Bh[i]);
  }
  return standardizeMatrix(result);
}

// ════════════════════════════════════════════════════════════════
// BMS↔UPMS conversion (from bms_upms_squeeze_converter.py)
// ════════════════════════════════════════════════════════════════

type ConvColumn = [number, number, number];
type ConvMatrix = ConvColumn[];

const EMPTY_COLUMN: ConvColumn = [-1, -1, -1];
const ZERO_COLUMN: ConvColumn = [0, 0, 0];
const BMS_BOUNDARY: ConvMatrix = [
  [0, 0, 0],
  [1, 1, 1],
  [2, 1, 0],
  [1, 1, 1],
];
const UPMS_BOUNDARY: ConvMatrix = [
  [0, 0, 0],
  [1, 1, 1],
  [2, 1, 1],
];

function columnAt(matrix: ConvMatrix, index: number): ConvColumn {
  return 0 <= index && index < matrix.length ? matrix[index] : EMPTY_COLUMN;
}

function highestNonZeroRow(col: ConvColumn): number {
  for (let r = 2; r >= 0; r--) {
    if (col[r] !== 0) return r + 1;
  }
  return 0;
}

type ParentCache = Map<string, number | null>;

function parentIndex(
  matrix: ConvMatrix,
  index: number,
  level: number,
  cache: ParentCache,
): number | null {
  const key = `${index},${level}`;
  if (cache.has(key)) return cache.get(key)!;
  const col = matrix[index];
  let result: number | null;
  if (level === 1) {
    result = null;
    for (let c = index - 1; c >= 0; c--) {
      if (col[0] > matrix[c][0]) {
        result = c;
        break;
      }
    }
  } else {
    result = null;
    let candidate = parentIndex(matrix, index, level - 1, cache);
    while (candidate !== null) {
      const colSlice = col.slice(level - 1);
      const candSlice = matrix[candidate].slice(level - 1);
      if (sequenceCompare(colSlice, candSlice) > 0) {
        result = candidate;
        break;
      }
      candidate = parentIndex(matrix, candidate, level - 1, cache);
    }
  }
  cache.set(key, result);
  return result;
}

function ancestorIndices(
  matrix: ConvMatrix,
  index: number,
  level: number,
  cache: ParentCache,
): number[] {
  const result: number[] = [];
  let candidate = parentIndex(matrix, index, level, cache);
  while (candidate !== null) {
    result.push(candidate);
    candidate = parentIndex(matrix, candidate, level, cache);
  }
  return result;
}

function isAncestor(
  matrix: ConvMatrix,
  ancestor: number,
  index: number,
  level: number,
  cache: ParentCache,
): boolean {
  return ancestorIndices(matrix, index, level, cache).includes(ancestor);
}

function childAboveParent(
  matrix: ConvMatrix,
  index: number,
  level: number,
  parent: number,
  cache: ParentCache,
): number | null {
  if (parentIndex(matrix, index, level, cache) === parent) return index;
  for (const anc of ancestorIndices(matrix, index, level, cache)) {
    if (parentIndex(matrix, anc, level, cache) === parent) return anc;
  }
  return null;
}

function addMatrices(left: ConvMatrix, right: ConvMatrix): ConvMatrix {
  return left.map((lc, i) => {
    const rc = right[i];
    return [lc[0] + rc[0], lc[1] + rc[1], lc[2] + rc[2]] as ConvColumn;
  });
}

function scaleMatrix(factor: number, matrix: ConvMatrix): ConvMatrix {
  return matrix.map((col) => [col[0] * factor, col[1] * factor, col[2] * factor] as ConvColumn);
}

function convFundamentalSequence(
  matrix: ConvMatrix,
  number: number,
  system: 'bms' | 'upms',
): ConvMatrix {
  if (number < 0) throw new Error('Fundamental-sequence index must be nonnegative');
  const source = matrix.map((c) => [...c] as ConvColumn);
  if (source.length === 0) throw new Error('Empty expression');
  if (
    source[source.length - 1][0] === 0 &&
    source[source.length - 1][1] === 0 &&
    source[source.length - 1][2] === 0
  )
    throw new Error('Successor expression');

  const lastIndex = source.length - 1;
  const lastColumn = source[lastIndex];
  const m = highestNonZeroRow(lastColumn);
  const cache: ParentCache = new Map();
  const parent = parentIndex(source, lastIndex, m, cache);
  if (parent === null) throw new Error(`Last column has no ${m}-parent`);

  const parentColumn = source[parent];
  const d: ConvColumn = [
    lastColumn[0] - (0 < m - 1 ? parentColumn[0] : parentColumn[0]),
    lastColumn[1] - (1 < m - 1 ? parentColumn[1] : parentColumn[1]),
    lastColumn[2] - (2 < m - 1 ? parentColumn[2] : parentColumn[2]),
  ];
  for (let r = 0; r < 3; r++) {
    if (r >= m - 1) d[r] = 0;
    else d[r] = lastColumn[r] - parentColumn[r];
  }
  const k = highestNonZeroRow(d);
  const prefix = source.slice(0, -1);

  if (number === 0) return prefix;
  if (number === 1) {
    return prefix.concat([[parentColumn[0] + d[0], parentColumn[1] + d[1], parentColumn[2] + d[2]]]);
  }

  const base = source.slice(parent, -1);

  let correction: ConvMatrix;
  if (system === 'bms' || k <= 1) {
    correction = [];
    for (let pos = parent; pos < lastIndex; pos++) {
      const values: number[] = [];
      for (let row = 1; row <= 3; row++) {
        const active =
          row <= k &&
          (pos === parent || isAncestor(source, parent, pos, row, cache));
        values.push(active ? d[row - 1] : 0);
      }
      correction.push([values[0], values[1], values[2]]);
    }
  } else {
    const h = 2 * Math.max(...source.flatMap((c) => [c[0], c[1], c[2]]));

    const targets: Map<number, ConvMatrix> = new Map();
    for (let level = 2; level <= k; level++) {
      const z = childAboveParent(source, lastIndex, level, parent, cache);
      if (z === null) throw new Error(`Could not define Y_${level}`);
      const yPrime = source.slice(z);
      const dI: ConvMatrix = [];
      for (let pos = z; pos < source.length; pos++) {
        const values: number[] = [];
        for (let row = 1; row <= 3; row++) {
          const active =
            row < level &&
            (pos === z || isAncestor(source, z, pos, row, cache));
          values.push(active ? h - source[z][row - 1] : 0);
        }
        dI.push([values[0], values[1], values[2]]);
      }
      targets.set(level, addMatrices(yPrime, dI));
    }

    const fullLength = source.length - parent;
    const vectors: Map<number, number[]> = new Map();
    vectors.set(1, new Array(fullLength).fill(1));

    for (let level = 2; level <= k; level++) {
      const vector = new Array(fullLength).fill(0);
      vector[0] = 1;
      vector[fullLength - 1] = 1;
      for (let localPos = 1; localPos < fullLength - 1; localPos++) {
        const pos = parent + localPos;
        if (!isAncestor(source, parent, pos, level, cache)) continue;
        if (vectors.get(level - 1)![localPos] === 0) continue;

        const zPrime = childAboveParent(source, pos, level, parent, cache);
        if (zPrime === null) continue;

        const xPrime = source.slice(zPrime);
        const dZ: ConvMatrix = [];
        for (let matrixPos = zPrime; matrixPos < source.length; matrixPos++) {
          const fromEnd = source.length - matrixPos;
          const values: number[] = [];
          for (let row = 1; row <= 3; row++) {
            const active =
              row < level && vectors.get(row)![vectors.get(row)!.length - fromEnd] === 1;
            values.push(active ? h - source[zPrime][row - 1] : 0);
          }
          dZ.push([values[0], values[1], values[2]]);
        }
        const xValue = addMatrices(xPrime, dZ);
        vector[localPos] = matrixCompare(xValue, targets.get(level)!) < 0 ? 0 : 1;
      }
      vectors.set(level, vector);
    }

    correction = [];
    for (let localPos = 0; localPos < base.length; localPos++) {
      const values: number[] = [];
      for (let row = 1; row <= 3; row++) {
        values.push(row <= k ? d[row - 1] * vectors.get(row)![localPos] : 0);
      }
      correction.push([values[0], values[1], values[2]]);
    }
  }

  const block = addMatrices(base, correction);
  let result = prefix.concat(block);
  for (let factor = 2; factor < number; factor++) {
    result = result.concat(addMatrices(base, scaleMatrix(factor, correction)));
  }
  return result;
}

// ── UPMS → BMS rewrite ──

function upmsToBMSRewrite(matrix: ConvMatrix): ConvMatrix {
  let current: ConvMatrix = matrix.map((c) => [...c] as ConvColumn);
  let pointer = 0;
  let loopGuard = 0;

  while (JSON.stringify(columnAt(current, pointer)) !== JSON.stringify(EMPTY_COLUMN)) {
    loopGuard++;
    if (loopGuard > 100_000) throw new Error('UPMS → BMS pointer loop');

    if (current[pointer][2] === 1) {
      pointer++;
      continue;
    }

    const x = current[pointer];
    const a = x[0],
      b = x[1];
    const low: ConvColumn = [a + 1, b + 1, 1];
    const case1Pattern: ConvColumn[] = [
      low,
      [a + 2, b, 0],
      [a + 3, b + 1, 1],
      [a + 4, b + 1, 0],
    ];

    const offsets = [1, 2, 3, 4].map((o) => columnAt(current, pointer + o));
    if (offsets.every((c, i) => JSON.stringify(c) === JSON.stringify(case1Pattern[i]))) {
      const high: ConvColumn = [a + 3, b + 1, 0];
      const followingPair: ConvColumn[] = [
        [a + 4, b + 2, 1],
        [a + 5, b + 1, 0],
      ];

      let scan = pointer + 4;
      let xEnd: number;
      while (true) {
        const col = columnAt(current, scan);
        if (
          sequenceCompare(col, high) < 0 ||
          (JSON.stringify(col) === JSON.stringify(high) &&
            matrixCompare(
              [columnAt(current, scan + 1), columnAt(current, scan + 2)],
              followingPair,
            ) < 0)
        ) {
          xEnd = scan - 1;
          break;
        }
        scan++;
      }

      scan = xEnd + 1;
      while (sequenceCompare(columnAt(current, scan), low) >= 0) scan++;
      const yEnd = scan - 1;

      const xBlock = current.slice(pointer + 1, xEnd + 1);
      const yBlock = current.slice(xEnd + 1, yEnd + 1);
      let zBlock: ConvMatrix = [];
      let insertZ = false;

      if (yBlock.length > 0) {
        const prefixPart = current.slice(pointer, yEnd + 1);
        const expanded = convFundamentalSequence(
          prefixPart.concat([low]),
          2,
          'upms',
        );
        if (
          expanded.slice(0, prefixPart.length).some((c, i) => JSON.stringify(c) !== JSON.stringify(prefixPart[i]))
        )
          throw new Error('FS(A,2) did not preserve xXY');
        zBlock = expanded.slice(prefixPart.length);
        insertZ = matrixCompare(current.slice(pointer), expanded.concat([[a + 2, 0, 0]])) < 0;
      }

      const xPrime = xBlock.slice(1).map((c) => [c[0] - 2, c[1], c[2]] as ConvColumn);
      const replacement = xPrime.concat(insertZ ? zBlock : yBlock);
      current = current.slice(0, pointer).concat(replacement).concat(current.slice(yEnd + 1));
      continue;
    }

    const case2 =
      JSON.stringify(columnAt(current, pointer + 1)) === JSON.stringify(low) &&
      JSON.stringify(columnAt(current, pointer + 2)) === JSON.stringify([a + 2, b + 1, 0] as ConvColumn) &&
      sequenceCompare(columnAt(current, pointer + 3), low) >= 0;

    if (case2) {
      let scan = pointer + 2;
      while (sequenceCompare(columnAt(current, scan), low) >= 0) scan++;
      const xEnd = scan - 1;
      const xX = current.slice(pointer, xEnd + 1);
      const expanded = convFundamentalSequence(xX.concat([low]), 2, 'upms');
      if (
        expanded.slice(0, xX.length).some((c, i) => JSON.stringify(c) !== JSON.stringify(xX[i]))
      )
        throw new Error('FS(A,2) did not preserve xX');
      const yBlock = expanded.slice(xX.length);
      const insertY = matrixCompare(current.slice(pointer), expanded.concat([[a + 2, 0, 0]])) < 0;
      const tail = current.slice(xEnd + 1);
      current = current
        .slice(0, pointer)
        .concat(xX.slice(0, 3))
        .concat(insertY ? yBlock : [])
        .concat(tail);
      pointer++;
      continue;
    }

    pointer++;
  }

  return current;
}

// ── Squeeze search ──

function smallestSqueezingIndex(y: ConvMatrix, x: ConvMatrix, cap: number): number {
  const converted = (index: number): ConvMatrix => {
    const candidate = convFundamentalSequence(y, index, 'upms');
    return upmsToBMSRaw(candidate);
  };

  if (matrixCompare(converted(0), x) >= 0) return 0;

  let lower = 0;
  let upper = 1;
  while (upper <= cap && matrixCompare(converted(upper), x) < 0) {
    lower = upper;
    if (upper === cap) break;
    upper = Math.min(cap, upper * 2);
  }

  if (matrixCompare(converted(upper), x) < 0) {
    for (let i = 0; i <= cap; i++) {
      if (matrixCompare(converted(i), x) >= 0) return i;
    }
    throw new Error(`No n in 0..${cap} satisfies U2B(y[n]) >= x`);
  }

  let low = lower + 1;
  let high = upper;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (matrixCompare(converted(mid), x) >= 0) high = mid;
    else low = mid + 1;
  }

  if (low > 0 && matrixCompare(converted(low - 1), x) >= 0) {
    for (let i = lower + 1; i <= low; i++) {
      if (matrixCompare(converted(i), x) >= 0) return i;
    }
  }
  return low;
}

function upmsToBMSRaw(matrix: ConvMatrix): ConvMatrix {
  if (JSON.stringify(matrix) === JSON.stringify(UPMS_BOUNDARY)) return BMS_BOUNDARY;
  return upmsToBMSRewrite(matrix);
}

function bmsToUPMSSqueeze(matrix: ConvMatrix): ConvMatrix {
  const x = matrix;
  if (JSON.stringify(x) === JSON.stringify(BMS_BOUNDARY)) return UPMS_BOUNDARY;
  if (matrixCompare(x, BMS_BOUNDARY) > 0) throw new Error('BMS input above boundary');

  let y: ConvMatrix = UPMS_BOUNDARY.map((c) => [...c] as ConvColumn);
  const visited = new Set<string>();
  const maxIter = Math.max(1000, 50 * (x.length + 1));

  for (let iter = 0; iter < maxIter; iter++) {
    const key = JSON.stringify(y);
    if (visited.has(key)) throw new Error('Squeeze search entered a cycle');
    visited.add(key);

    const convertedY = upmsToBMSRaw(y);
    if (JSON.stringify(convertedY) === JSON.stringify(x)) return y;
    if (matrixCompare(convertedY, x) < 0) throw new Error('Squeeze descended below target');

    if (
      y.length > 0 &&
      y[y.length - 1][0] === 0 &&
      y[y.length - 1][1] === 0 &&
      y[y.length - 1][2] === 0
    ) {
      const candidate = y.slice(0, -1);
      const convertedCandidate = upmsToBMSRaw(candidate);
      if (matrixCompare(convertedCandidate, x) < 0)
        throw new Error('BMS target between successor and predecessor');
      y = candidate;
      continue;
    }

    const cap = 5 * x.length;
    const index = smallestSqueezingIndex(y, x, cap);
    const candidate = convFundamentalSequence(y, index, 'upms');
    if (JSON.stringify(candidate) === JSON.stringify(y))
      throw new Error('Squeeze step did not decrease');
    y = candidate;
  }

  throw new Error('Squeeze exceeded iteration limit');
}

// ════════════════════════════════════════════════════════════════
// Public conversion API
// ════════════════════════════════════════════════════════════════

/** Convert UPMS expression to BMS. Input/output are column-major Expr. */
export function upmsToBMS(input: Expr): Expr {
  const source = standardizeMatrix(input);
  if (source.every((col) => col.length <= 2 || col[2] === 0)) return source;
  const convMatrix: ConvMatrix = source.map(
    (col) => [col[0] || 0, col[1] || 0, col[2] || 0] as ConvColumn,
  );
  if (matrixCompare(convMatrix, UPMS_BOUNDARY) > 0)
    throw new Error('UPMS input above boundary 0 111 211');
  const result = upmsToBMSRaw(convMatrix);
  const roundTrip = bmsToUPMSSqueeze(result);
  if (JSON.stringify(roundTrip) !== JSON.stringify(convMatrix))
    throw new Error('Converted BMS failed standardness round trip');
  return result.map((c) => [c[0], c[1], c[2]]);
}

/** Convert BMS expression to UPMS. Input/output are column-major Expr. */
export function bmsToUPMS(input: Expr): Expr {
  const source = standardizeMatrix(input);
  if (source.every((col) => col.length <= 2 || col[2] === 0)) return source;
  const convMatrix: ConvMatrix = source.map(
    (col) => [col[0] || 0, col[1] || 0, col[2] || 0] as ConvColumn,
  );
  const result = bmsToUPMSSqueeze(convMatrix);
  const roundTrip = upmsToBMSRaw(result);
  if (JSON.stringify(roundTrip) !== JSON.stringify(convMatrix))
    throw new Error('Converted UPMS failed standardness round trip');
  return result.map((c) => [c[0], c[1], c[2]]);
}

/** Convert UPMS column-major Expr to the project's Matrix format (also column-major, so just standardize) */
export function upmsExprToMatrixMSMatrix(expr: Expr): number[][] {
  return standardizeMatrix(expr);
}

/** Convert project's Matrix to UPMS column-major Expr (both are column-major) */
export function bmsMatrixToUpmsExpr(matrix: number[][]): Expr {
  return standardizeMatrix(matrix);
}
