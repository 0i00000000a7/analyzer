import './assets/default.css';
import katex from 'katex';
import 'katex/dist/katex.min.css';
import {
  parseMatrix,
  analyze,
  bmsTo0YSequence,
  matrixToLatex,
  parseAndEvalBOCF,
  expandBMS,
  bocfToBMS,
  termToVeblen,
  fundamentalSequence,
} from './ts/bms.js';
import { parse0Y, zeroYToBMS, zeroYExpand, buildMountain } from './ts/bms-zero-y.js';
import { triangularToBMS, bmsToTriangular } from './ts/bms-triangular.js';
import { expand1Y, expandWY, buildWYMountain, build1YMountain } from './ts/wy.js';
import { oneYToDBMS, dbmsToString, dbmsToBMS } from './ts/y_dbms.js';
import type { AnalysisResult, Matrix, Mountain, MountainNode, WYMountainResult } from './ts/types.js';

const input = document.getElementById('input') as HTMLInputElement;
const output = document.getElementById('output') as HTMLDivElement;
const outputVeblen = document.getElementById('output-veblen') as HTMLDivElement;
const output0y = document.getElementById('output-0y') as HTMLSpanElement;
const outputDbms = document.getElementById('output-dbms') as HTMLSpanElement;
const outputTriangular = document.getElementById('output-triangular') as HTMLSpanElement;
const triangularRow = document.getElementById('triangular-row') as HTMLDivElement;
const mountainRow = document.getElementById('mountain-row') as HTMLDivElement;
const mountainSvg = document.getElementById('mountain-svg') as unknown as SVGSVGElement;
const outputBms = document.getElementById('output-bms') as HTMLSpanElement;
const bmsOutputRow = document.getElementById('bms-output-row') as HTMLDivElement;
const outputAst = document.getElementById('output-ast') as HTMLPreElement;

const modeVBtn = document.getElementById('mode-v') as HTMLButtonElement;
const modeMBtn = document.getElementById('mode-m') as HTMLButtonElement;
const sugarToggle = document.getElementById('sugar-toggle') as HTMLInputElement;

const inputModeBmsBtn = document.getElementById('input-mode-bms') as HTMLButtonElement;
const inputMode0yBtn = document.getElementById('input-mode-0y') as HTMLButtonElement;
const inputModeBocfBtn = document.getElementById('input-mode-bocf') as HTMLButtonElement;
const inputMode1yBtn = document.getElementById('input-mode-1y') as HTMLButtonElement;
const inputModeWyBtn = document.getElementById('input-mode-wy') as HTMLButtonElement;

const expandRow = document.getElementById('expand-row') as HTMLDivElement;
const expandBtn = document.getElementById('expand-btn') as HTMLButtonElement;
const expandFs = document.getElementById('expand-fs') as HTMLInputElement;
const outputExpand = document.getElementById('output-expand') as HTMLSpanElement;

const bmsMatrixBtn = document.getElementById('bms-mode-matrix') as HTMLButtonElement;
const bmsFlatBtn = document.getElementById('bms-mode-flat') as HTMLButtonElement;
const bocfToBmsBtn = document.getElementById('bocf-to-bms-btn') as HTMLButtonElement;
const bmsStatus = document.getElementById('bms-status') as HTMLSpanElement;

type VeblenMode = 'v' | 'm';
type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf';
type BmsDisplayMode = 'matrix' | 'flat';

let currentVeblenMode: VeblenMode = 'v';
let currentInputMode: InputMode = 'bms';
let currentBmsMode: BmsDisplayMode = 'flat';
let lastResult: AnalysisResult | null = null;
let currentBocfOrdinal: any[] | null = null;
let current0YSeq: number[] | null = null;
let current1YSeq: number[] | null = null;
let currentWYSeq: number[] | null = null;

function isTriangularMatrix(matrix: Matrix): boolean {
  if (matrix.length < 3) return false;
  const c0 = matrix[0],
    c1 = matrix[1],
    c2 = matrix[2];
  return (
    (c0[0] ?? 0) === 0 &&
    (c0[1] ?? 0) === 0 &&
    (c0[2] ?? 0) === 0 &&
    (c1[0] ?? 0) === 1 &&
    (c1[1] ?? 0) === 0 &&
    (c1[2] ?? 0) === 0 &&
    (c2[0] ?? 0) === 2 &&
    (c2[1] ?? 0) === 1 &&
    (c2[2] ?? 0) === 0
  );
}

// Render button labels with KaTeX
modeVBtn.innerHTML = katex.renderToString('\\alpha @\\beta', {
  throwOnError: false,
});
modeMBtn.innerHTML = katex.renderToString('\\begin{smallmatrix}\\alpha\\\\\\beta\\end{smallmatrix}', { throwOnError: false });

function setActiveVeblenMode(mode: VeblenMode) {
  currentVeblenMode = mode;
  [modeVBtn, modeMBtn].forEach((b) => b.classList.remove('active'));
  if (mode === 'v') modeVBtn.classList.add('active');
  if (mode === 'm') modeMBtn.classList.add('active');
  renderOutput();
}

function setActiveInputMode(mode: InputMode) {
  currentInputMode = mode;
  [inputModeBmsBtn, inputMode0yBtn, inputMode1yBtn, inputModeWyBtn, inputModeBocfBtn].forEach((b) =>
    b.classList.remove('active'),
  );
  // Clear all output divs
  output.textContent = '';
  outputVeblen.textContent = '';
  output0y.textContent = '';
  outputDbms.textContent = '';
  outputBms.textContent = '';
  outputExpand.textContent = '';
  outputAst.textContent = '';
  outputAst.style.display = 'none';
  triangularRow.style.display = 'none';
  mountainRow.style.display = 'none';
  // Reset sequence stores
  current0YSeq = null;
  current1YSeq = null;
  currentWYSeq = null;
  currentBocfOrdinal = null;
  lastResult = null;

  if (mode === 'bms') {
    inputModeBmsBtn.classList.add('active');
    input.placeholder = '';
    bmsOutputRow.style.display = 'none';
    outputAst.style.display = 'none';
    expandRow.style.display = 'flex';
    bocfToBmsBtn.style.display = 'none';
    bmsStatus.style.display = 'none';
  } else if (mode === '0y') {
    inputMode0yBtn.classList.add('active');
    input.placeholder = 'e.g. 1,4,8,11';
    bmsOutputRow.style.display = 'flex';
    outputAst.style.display = 'none';
    expandRow.style.display = 'flex';
    bocfToBmsBtn.style.display = 'none';
    bmsStatus.style.display = 'none';
  } else if (mode === '1y') {
    inputMode1yBtn.classList.add('active');
    input.placeholder = 'e.g. 1,2,3,4';
    bmsOutputRow.style.display = 'none';
    outputAst.style.display = 'none';
    expandRow.style.display = 'flex';
    bocfToBmsBtn.style.display = 'none';
    bmsStatus.style.display = 'none';
  } else if (mode === 'wy') {
    inputModeWyBtn.classList.add('active');
    input.placeholder = 'e.g. 1,2,3,4';
    bmsOutputRow.style.display = 'none';
    outputAst.style.display = 'none';
    expandRow.style.display = 'flex';
    bocfToBmsBtn.style.display = 'none';
    bmsStatus.style.display = 'none';
  } else {
    inputModeBocfBtn.classList.add('active');
    input.placeholder = 'e.g. ψ(Ω) or \\psi(\\Omega)';
    bmsOutputRow.style.display = 'flex';
    outputAst.style.display = 'none';
    expandRow.style.display = 'flex';
    bocfToBmsBtn.style.display = 'inline';
    bmsStatus.style.display = 'inline';
  }
  update();
}

function setActiveBmsMode(mode: BmsDisplayMode) {
  currentBmsMode = mode;
  [bmsMatrixBtn, bmsFlatBtn].forEach((b) => b.classList.remove('active'));
  if (mode === 'matrix') bmsMatrixBtn.classList.add('active');
  else bmsFlatBtn.classList.add('active');
  const raw = outputBms.getAttribute('data-raw');
  if (raw !== null) renderBms(raw);
}

function alignMatrixStr(s: string): string {
  if (s === '(empty)' || s === '(error)') return s;
  const cols: string[][] = [];
  s.replace(/\(([^)]+)\)/g, (_, inner: string) => {
    cols.push(inner.split(','));
    return '';
  });
  // Trim trailing zeros per column
  for (const col of cols) {
    while (col.length > 1 && col[col.length - 1] === '0') col.pop();
  }
  // Pad to common max length
  let maxLen = 0;
  for (const col of cols) if (col.length > maxLen) maxLen = col.length;
  for (const col of cols) while (col.length < maxLen) col.push('0');
  return cols.map((col) => '(' + col.join(',') + ')').join('');
}

function renderBms(raw: string) {
  if (raw === '(empty)' || raw === '(error)') {
    outputBms.textContent = raw;
    return;
  }
  const aligned = alignMatrixStr(raw);
  if (currentBmsMode === 'matrix') {
    const matrix = parseMatrix(aligned);
    outputBms.innerHTML = katex.renderToString(matrixToLatex(matrix), {
      throwOnError: false,
    });
  } else {
    outputBms.innerHTML = katex.renderToString('\\text{' + aligned + '}', {
      throwOnError: false,
    });
  }
}

function getVeblenOutput(r: AnalysisResult, mode: VeblenMode, sugar: boolean): string | null {
  const key = mode === 'v' ? (sugar ? 'veblen' : 'veblenPlain') : sugar ? 'veblenMatrix' : 'veblenMatrixPlain';
  return (r as any)[key] || null;
}

function renderOutput() {
  if (!lastResult) return;
  const r = lastResult;
  if (r.gteEBO || !r.veblen) {
    outputVeblen.textContent = '';
    return;
  }
  const v = getVeblenOutput(r, currentVeblenMode, sugarToggle.checked);
  if (v) {
    outputVeblen.innerHTML = katex.renderToString(v, { throwOnError: false });
  } else {
    outputVeblen.textContent = '';
  }
}

input.value = '(0,0,0)(1,1,1)(2,1,0)(1,1,1)';

async function update() {
  try {
    const raw = parseInt(expandFs.value);
    const fs = isNaN(raw) ? 3 : raw;

    if (currentInputMode === 'bocf') {
      const r = await parseAndEvalBOCF(input.value);
      if (r.error) {
        output.textContent = '(error)';
        outputVeblen.textContent = '';
        output0y.textContent = '';
        outputAst.textContent = r.error;
        outputBms.textContent = '';
        return;
      }
      outputAst.textContent = r.ast;
      output.innerHTML = r.ordinal ? katex.renderToString(r.ordinal, { throwOnError: false }) : '';
      if (r.ordinalJS) {
        currentBocfOrdinal = r.ordinalJS;
        const v = await termToVeblen(r.ordinalJS);
        lastResult = {
          gteEBO: false,
          ordinal: r.ordinal,
          ordinalJS: r.ordinalJS,
          ...v,
          nsForm: '',
          isStandard: true,
        } as AnalysisResult;
        renderOutput();
      } else {
        currentBocfOrdinal = null;
        outputVeblen.textContent = '';
        lastResult = null;
      }
      output0y.textContent = '';
      triangularRow.style.display = 'none';
      mountainRow.style.display = 'none';
      return;
    }

    let matrix: Matrix;

    if (currentInputMode === '0y') {
      const seq = parse0Y(input.value);
      if (seq.length === 0 || seq.some(isNaN) || input.value.trim() === '') {
        current0YSeq = null;
        output.textContent = '';
        outputVeblen.textContent = '';
        output0y.textContent = '';
        outputBms.textContent = '';
        triangularRow.style.display = 'none';
        mountainRow.style.display = 'none';
        return;
      }
      current0YSeq = seq;
      matrix = await zeroYToBMS(seq);
      const flat = matrixToDisplayStr(matrix);
      outputBms.setAttribute('data-raw', flat);
      renderBms(flat);
      output0y.textContent = '';
      // Compute and draw mountain diagram
      try {
        const mountain = await buildMountain(seq);
        drawMountain(mountain);
      } catch {
        mountainRow.style.display = 'none';
      }
    } else if (currentInputMode === '1y') {
      const seq = parse0Y(input.value);
      if (seq.length === 0 || seq.some(isNaN) || input.value.trim() === '') {
        current1YSeq = null;
        outputExpand.textContent = '';
        return;
      }
      current1YSeq = seq;
      // Show initial sequence
      outputExpand.textContent = seq.join(',');
      output.textContent = '';
      outputVeblen.textContent = '';
      output0y.textContent = '';
      outputBms.textContent = '';
      triangularRow.style.display = 'none';
      bmsOutputRow.style.display = 'none';
      // Compute and draw mountain diagram using pure 1-Y extraction
      try {
        const result = await build1YMountain(seq);
        console.log('1Y mountain result:', JSON.stringify(result));
        draw1YMountain(result.layers, result.rows);
      } catch {
        mountainRow.style.display = 'none';
      }
      // Compute and display DBMS
      try {
        const dbms = await oneYToDBMS(seq);
        const dbmsStr = await dbmsToString(dbms);
        const hasOmega = dbmsStr.includes(',,');
        const displayStr = hasOmega ? '\\geq\\text{' + dbmsStr + '}' : '\\text{' + dbmsStr + '}';
        outputDbms.innerHTML = katex.renderToString(displayStr, {
          throwOnError: false,
        });
        if (dbms.length > 0 && !hasOmega) {
          const bms = await dbmsToBMS(dbms);
          if (bms.length > 0) {
            const flat = bms.map((c) => '(' + c.join(',') + ')').join('');
            outputBms.setAttribute('data-raw', flat);
            renderBms(flat);
            bmsOutputRow.style.display = 'flex';
            try {
              const r = await analyze(bms);
              output.innerHTML = katex.renderToString(r.ordinal, { throwOnError: false });
              if (r.veblen) {
                outputVeblen.innerHTML = katex.renderToString(r.veblen, { throwOnError: false });
              }
              const seq0y = await bmsTo0YSequence(bms);
              output0y.innerHTML = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
            } catch {
              output.textContent = '(error)';
            }
          } else {
            bmsOutputRow.style.display = 'none';
          }
        } else {
          bmsOutputRow.style.display = 'none';
          output.textContent = '';
          outputVeblen.textContent = '';
          output0y.textContent = '';
        }
      } catch {
        outputDbms.textContent = '';
        bmsOutputRow.style.display = 'none';
      }
      return;
    } else if (currentInputMode === 'wy') {
      const seq = parse0Y(input.value);
      if (seq.length === 0 || seq.some(isNaN) || input.value.trim() === '') {
        currentWYSeq = null;
        outputExpand.textContent = '';
        return;
      }
      currentWYSeq = seq;
      outputExpand.textContent = seq.join(',');
      output.textContent = '';
      outputVeblen.textContent = '';
      output0y.textContent = '';
      outputBms.textContent = '';
      triangularRow.style.display = 'none';
      bmsOutputRow.style.display = 'none';
      // Compute and draw mountain diagram
      try {
        const result = await buildWYMountain(seq, -1);
        console.log('WY mountain result:', JSON.stringify(result));
        drawWYMountain(result.layers, result.rows);
      } catch {
        mountainRow.style.display = 'none';
      }
      // Compute and display DBMS
      try {
        const dbms = await oneYToDBMS(seq);
        const dbmsStr = await dbmsToString(dbms);
        const hasOmega = dbmsStr.includes(',,');
        const displayStr = hasOmega ? '\\geq\\text{' + dbmsStr + '}' : '\\text{' + dbmsStr + '}';
        outputDbms.innerHTML = katex.renderToString(displayStr, {
          throwOnError: false,
        });
        if (dbms.length > 0 && !hasOmega) {
          const bms = await dbmsToBMS(dbms);
          if (bms.length > 0) {
            const flat = bms.map((c) => '(' + c.join(',') + ')').join('');
            outputBms.setAttribute('data-raw', flat);
            renderBms(flat);
            bmsOutputRow.style.display = 'flex';
            try {
              const r = await analyze(bms);
              output.innerHTML = katex.renderToString(r.ordinal, { throwOnError: false });
              if (r.veblen) {
                outputVeblen.innerHTML = katex.renderToString(r.veblen, { throwOnError: false });
              }
              const seq0y = await bmsTo0YSequence(bms);
              output0y.innerHTML = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
            } catch {
              output.textContent = '(error)';
            }
          } else {
            bmsOutputRow.style.display = 'none';
          }
        } else {
          bmsOutputRow.style.display = 'none';
          output.textContent = '';
          outputVeblen.textContent = '';
          output0y.textContent = '';
        }
      } catch {
        outputDbms.textContent = '';
        bmsOutputRow.style.display = 'none';
      }
      return;
    } else {
      matrix = parseMatrix(input.value);
      mountainRow.style.display = 'none';
    }

    // Triangular BMS conversion (all input modes)
    if (currentInputMode === 'bms' && matrix.length >= 3 && isTriangularMatrix(matrix)) {
      const raw = matrixToDisplayStr(matrix);
      const aligned = alignMatrixStr(raw);
      triangularRow.style.display = 'flex';
      outputTriangular.innerHTML = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
      matrix = await triangularToBMS(matrix);
    } else {
      const triMatrix = await bmsToTriangular(matrix);
      if (triMatrix && triMatrix.length > 0) {
        triangularRow.style.display = 'flex';
        const raw = matrixToDisplayStr(triMatrix);
        const aligned = alignMatrixStr(raw);
        outputTriangular.innerHTML = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
      } else {
        triangularRow.style.display = 'none';
      }
    }

    const r = await analyze(matrix);
    lastResult = r;

    output.innerHTML = katex.renderToString(r.ordinal, { throwOnError: false });

    if (currentInputMode === 'bms') {
      const t0y = performance.now();
      const seq = await bmsTo0YSequence(matrix);
      console.log('bmsTo0Y: ' + (performance.now() - t0y).toFixed(2) + 'ms');
      output0y.innerHTML = seq ? katex.renderToString(seq, { throwOnError: false }) : '';
    }

    if (r.veblen) {
      renderOutput();
    } else {
      outputVeblen.textContent = '';
    }
  } catch {
    output.textContent = '(error)';
    outputVeblen.textContent = '';
    output0y.textContent = '';
    outputAst.textContent = '';
    triangularRow.style.display = 'none';
    mountainRow.style.display = 'none';
  }
}

modeVBtn.addEventListener('click', () => setActiveVeblenMode('v'));
modeMBtn.addEventListener('click', () => setActiveVeblenMode('m'));
sugarToggle.addEventListener('change', renderOutput);
input.addEventListener('input', update);
inputModeBmsBtn.addEventListener('click', () => setActiveInputMode('bms'));
inputMode0yBtn.addEventListener('click', () => setActiveInputMode('0y'));
inputMode1yBtn.addEventListener('click', () => setActiveInputMode('1y'));
inputModeWyBtn.addEventListener('click', () => setActiveInputMode('wy'));
inputModeBocfBtn.addEventListener('click', () => setActiveInputMode('bocf'));

bmsMatrixBtn.addEventListener('click', () => setActiveBmsMode('matrix'));
bmsFlatBtn.addEventListener('click', () => setActiveBmsMode('flat'));

bocfToBmsBtn.addEventListener('click', async () => {
  bmsStatus.textContent = 'searching...';
  outputBms.textContent = '';
  // Yield to let the browser paint "searching..." before blocking WASM call
  await new Promise((r) => setTimeout(r, 50));
  const startTime = performance.now();
  try {
    const bms = await bocfToBMS(input.value, (cur: string) => {
      const elapsed = ((performance.now() - startTime) / 1000).toFixed(1);
      bmsStatus.textContent = 'iter ' + cur + ' (' + elapsed + 's)';
    });
    outputBms.setAttribute('data-raw', bms);
    renderBms(bms);
    if (bms !== '(empty)') {
      const matrix = parseMatrix(bms);
      const t0 = performance.now();
      const seq = await bmsTo0YSequence(matrix);
      console.log('bmsTo0Y: ' + (performance.now() - t0).toFixed(1) + 'ms');
      output0y.innerHTML = seq ? katex.renderToString(seq, { throwOnError: false }) : '';

      // Triangular BMS display
      const triMatrix = await bmsToTriangular(matrix);
      if (triMatrix && triMatrix.length > 0) {
        triangularRow.style.display = 'flex';
        const raw = matrixToDisplayStr(triMatrix);
        const aligned = alignMatrixStr(raw);
        outputTriangular.innerHTML = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
      } else {
        triangularRow.style.display = 'none';
      }
    }
    const elapsed = ((performance.now() - startTime) / 1000).toFixed(3);
    bmsStatus.textContent = 'Done (' + elapsed + 's)';
  } catch (e) {
    bmsStatus.textContent = String(e);
  }
});

function matrixToDisplayStr(m: Matrix): string {
  return m.map((col) => '(' + col.join(',') + ')').join('');
}

function drawMountain(mountain: Mountain) {
  if (!mountain.length) {
    mountainRow.style.display = 'none';
    return;
  }
  let layers = mountain.length;
  if (layers > 1 && mountain[layers - 1].every((n: MountainNode) => n.value === 1)) layers--;
  const cols = mountain[0].length;
  const colLabelW = 20;
  const gapX = 50;
  const gapY = 55;
  const padX = 25 + colLabelW;
  const padY = 30;
  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;

  // Node center
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY;
  // Connection points: offset above/below each node center
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off; // above node
  const cyB = (layer: number) => cy(layer) + off; // below node

  // Draw connections
  for (let layer = 1; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const belowNode = mountain[layer - 1][col];
      // Vertical: from below upper node to above child
      svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(layer - 1)}" stroke="#888" stroke-width="1.5" stroke-linecap="round"/>`;
      // Diagonal: from below upper node to above child's parent
      if (belowNode.parent > 0) {
        const pcol = col - belowNode.parent;
        if (pcol >= 0) {
          svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(pcol)}" y2="${cyA(layer - 1)}" stroke="#888" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }
    }
  }

  // Draw row labels (left side)
  svg += `<g font-size="16" fill="#888" text-anchor="end">`;
  svg += `<text x="${padX - colLabelW + 4}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="13" fill="#aaa">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    svg += `<text x="${padX - colLabelW + 4}" y="${cy(layer) + 1}" dominant-baseline="middle">${layer}</text>`;
  }
  svg += `</g>`;

  // Draw values as text
  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      const x = cx(col);
      const y = cy(layer);
      svg += `<text x="${x}" y="${y + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="#333" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  const container = document.getElementById('mountain-svg-container') as HTMLDivElement;
  if (container) container.innerHTML = svg;
  mountainRow.style.display = 'block';
}

/** Convert little-endian ordinal array to LaTeX-style string */
function ordinalToLatex(ord: number[]): string {
  let end = ord.length;
  while (end > 0 && ord[end - 1] === 0) end--;
  if (end === 0) return '0';
  const parts: string[] = [];
  let first = true;
  for (let i = end - 1; i >= 0; i--) {
    const c = ord[i];
    if (c === 0) continue;
    if (!first) parts.push('+');
    first = false;
    if (i === 0) {
      parts.push(String(c));
    } else if (i === 1) {
      parts.push(c === 1 ? 'ω' : 'ω' + c);
    } else {
      parts.push('ω<tspan baseline-shift="super" font-size="0.65em">' + i + '</tspan>' + (c > 1 ? c : ''));
    }
  }
  return parts.join('');
}

/** Number of separator lines between two consecutive row labels.
 *  Based on the highest ω-index where coefficients differ:
 *  index 0 → 0 lines (finite), index 1 → 1 line (ω), index 2 → 2 lines (ω²), ... */
function separatorLineCount(rowA: number[], rowB: number[]): number {
  const maxLen = Math.max(rowA.length, rowB.length);
  for (let i = maxLen - 1; i >= 0; i--) {
    const a = i < rowA.length ? rowA[i] : 0;
    const b = i < rowB.length ? rowB[i] : 0;
    if (a !== b) return i === 0 ? 0 : i;
  }
  return 0;
}

/** Draw WY mountain diagram (original style — gray connections, no separators) */
function drawWYMountain(mountain: Mountain, rowLabels: number[][]) {
  if (!mountain.length || !mountain[0].length) {
    mountainRow.style.display = 'none';
    return;
  }
  const layers = mountain.length;
  const cols = mountain[0].length;
  const gapX = 50;
  const gapY = 55;
  const padX = 65;
  const padY = 30;
  const extraGap = 30;

  // Compute layer shift: layers above ω-boundaries get lifted
  const layerShift: number[] = new Array(layers).fill(0);
  let totalShift = 0;
  for (let k = 1; k < layers; k++) {
    const prevRow = k - 1 < rowLabels.length ? rowLabels[k - 1] : [k - 1];
    const curRow = k < rowLabels.length ? rowLabels[k] : [k];
    if (separatorLineCount(prevRow, curRow) > 0) totalShift += extraGap;
    layerShift[k] = totalShift;
  }

  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2 + totalShift;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY + totalShift - layerShift[layer];
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off;
  const cyB = (layer: number) => cy(layer) + off;
  // Normal (unshifted) y for maintaining diagonal angle across ω-boundaries
  const normalYDisp = gapY - 2 * off; // 37px

  // Track last drawn row per column
  const lastRow: number[] = new Array(cols).fill(-1);

  // Separator lines based on ordinal difference between consecutive rows
  svg += `<g stroke="#999" fill="none">`;
  for (let k = 1; k < layers; k++) {
    const prevRow = k - 1 < rowLabels.length ? rowLabels[k - 1] : [k - 1];
    const curRow = k < rowLabels.length ? rowLabels[k] : [k];
    const nLines = separatorLineCount(prevRow, curRow);
    if (nLines === 0) continue;
    const sepExt = 12;
    const lineSpacing = 4;
    // Place lines between the top of the vertical extension and the top of the next row
    const gapTop = cyB(k) + normalYDisp; // where diagonal meets vertical extension
    const gapBottom = cyA(k - 1); // top of the row below
    const gapMid = (gapTop + gapBottom) / 2;
    const yStart = gapMid - ((nLines - 1) * lineSpacing) / 2;
    for (let n = 0; n < nLines; n++) {
      const y = yStart + n * lineSpacing;
      svg += `<line x1="${padX - sepExt}" y1="${y}" x2="${padX + (cols - 1) * gapX + sepExt}" y2="${y}" stroke-width="1.5"/>`;
    }
  }
  svg += `</g>`;

  // Draw connections
  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      if (node.value < 0) continue;

      // Right leg
      if (lastRow[col] >= 0) {
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(lastRow[col])}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
      }

      // Left leg: diagonal at normal angle, then vertical extended through any extra gap
      const parentCol = node.parentCol ?? -1;
      if (parentCol >= 0 && parentCol < cols && layer > 0) {
        const diagEndY = cyB(layer) + normalYDisp; // normal 37px drop
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(parentCol)}" y2="${diagEndY}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
        // Extend from normal endpoint through gap to actual layer position
        if (Math.abs(diagEndY - cyA(layer - 1)) > 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${diagEndY}" x2="${cx(parentCol)}" y2="${cyA(layer - 1)}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
        }
        // Further extension to earlier nodes in parent column
        if (lastRow[parentCol] >= 0 && lastRow[parentCol] < layer - 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${cyA(layer - 1)}" x2="${cx(parentCol)}" y2="${cyA(lastRow[parentCol])}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }

      lastRow[col] = layer;
    }
  }

  // Row labels
  svg += `<g font-size="14" fill="#888" text-anchor="end">`;
  svg += `<text x="${padX - 20}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="12" fill="#aaa">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    const ord = layer < rowLabels.length ? rowLabels[layer] : [layer];
    const label = ordinalToLatex(ord);
    svg += `<text x="${padX - 20}" y="${cy(layer) + 1}" dominant-baseline="middle">${label}</text>`;
  }
  svg += `</g>`;

  // Values
  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      if (node.value < 0) continue;
      svg += `<text x="${cx(col)}" y="${cy(layer) + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="#333" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  const container = document.getElementById('mountain-svg-container') as HTMLDivElement;
  if (container) container.innerHTML = svg;
  mountainRow.style.display = 'block';
}

/** Draw 1-Y mountain diagram with extraction separators and dotted extraction lines */
function draw1YMountain(mountain: Mountain, rowLabels: number[][]) {
  if (!mountain.length) {
    mountainRow.style.display = 'none';
    return;
  }
  const layers = mountain.length;
  const cols = mountain.reduce((max, layer) => Math.max(max, layer.length), 0);
  if (cols === 0) {
    mountainRow.style.display = 'none';
    return;
  }
  const gapX = 50;
  const gapY = 55;
  const padX = 65;
  const padY = 30;
  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY;
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off;
  const cyB = (layer: number) => cy(layer) + off;

  // Detect extraction base layers (row label starts with 0, meaning ω·e row)
  const isNewExtraction: boolean[] = new Array(layers).fill(false);
  for (let i = 1; i < layers; i++) {
    const row = rowLabels[i] || [i];
    isNewExtraction[i] = row.length > 0 && row[0] === 0;
  }

  // Draw separator lines (background)
  svg += `<g stroke="#999" fill="none">`;
  for (let layer = 0; layer < layers; layer++) {
    if (!isNewExtraction[layer]) continue;
    const sepY = (cy(layer) + cy(layer - 1)) / 2;
    const sepExt = 12;
    svg += `<line x1="${padX - sepExt}" y1="${sepY}" x2="${padX + (cols - 1) * gapX + sepExt}" y2="${sepY}" stroke-width="1.5"/>`;
  }
  svg += `</g>`;

  // Track last drawn row per column
  const lastRow: number[] = new Array(cols).fill(-1);

  // Draw connections
  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = col < mountain[layer].length ? mountain[layer][col] : null;
      if (!node || node.value < 0) continue;

      // Right leg: dotted for extraction base layers, solid for others
      if (lastRow[col] >= 0) {
        const strokeStyle = isNewExtraction[layer] ? 'stroke="#888" stroke-dasharray="4,3"' : 'stroke="#000"';
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(lastRow[col])}" ${strokeStyle} stroke-width="1.5" stroke-linecap="round"/>`;
      }

      // Left leg: only for layers above base (L0 has no left legs).
      // Skip for extraction base layers (ω-boundary crossings).
      const parentCol = node.parentCol ?? -1;
      if (parentCol >= 0 && parentCol < cols && layer > 0 && !isNewExtraction[layer]) {
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(parentCol)}" y2="${cyA(layer - 1)}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
        if (lastRow[parentCol] >= 0 && lastRow[parentCol] < layer - 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${cyA(layer - 1)}" x2="${cx(parentCol)}" y2="${cyA(lastRow[parentCol])}" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }

      lastRow[col] = layer;
    }
  }

  // Row labels
  svg += `<g font-size="14" fill="#888" text-anchor="end">`;
  svg += `<text x="${padX - 20}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="12" fill="#aaa">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    const ord = layer < rowLabels.length ? rowLabels[layer] : [layer];
    const label = ordinalToLatex(ord);
    svg += `<text x="${padX - 20}" y="${cy(layer) + 1}" dominant-baseline="middle">${label}</text>`;
  }
  svg += `</g>`;

  // Values
  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = col < mountain[layer].length ? mountain[layer][col] : null;
      if (!node || node.value < 0) continue;
      svg += `<text x="${cx(col)}" y="${cy(layer) + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="#333" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  const container = document.getElementById('mountain-svg-container') as HTMLDivElement;
  if (container) container.innerHTML = svg;
  mountainRow.style.display = 'block';
}

expandBtn.addEventListener('click', async () => {
  try {
    const raw = parseInt(expandFs.value);
    const fs = isNaN(raw) ? 3 : raw;
    if (currentInputMode === '1y') {
      if (!current1YSeq) {
        outputExpand.textContent = '(no sequence)';
        return;
      }
      const expanded = await expand1Y(current1YSeq, fs);
      outputExpand.textContent = expanded.join(',');
    } else if (currentInputMode === 'wy') {
      if (!currentWYSeq) {
        outputExpand.textContent = '(no sequence)';
        return;
      }
      const expanded = await expandWY(currentWYSeq, fs);
      outputExpand.textContent = expanded.join(',');
    } else if (currentInputMode === 'bocf') {
      if (!currentBocfOrdinal) {
        outputExpand.textContent = '(no ordinal)';
        return;
      }
      const r = await fundamentalSequence(currentBocfOrdinal, fs);
      console.log('r.term:', r.term);
      outputExpand.innerHTML = r.term ? katex.renderToString(r.term, { throwOnError: false }) : '0';
    } else if (currentInputMode === '0y') {
      if (!current0YSeq) {
        outputExpand.textContent = '(no sequence)';
        return;
      }
      const expanded = await zeroYExpand(current0YSeq, fs);
      outputExpand.textContent = expanded.join(',');
    } else {
      const matrix = parseMatrix(input.value);
      const expanded = await expandBMS(matrix, fs);
      outputExpand.textContent = matrixToDisplayStr(expanded);
    }
  } catch {
    outputExpand.textContent = '(error)';
  }
});

setActiveInputMode('bms');
