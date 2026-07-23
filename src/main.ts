import './assets/default.css';
import katex from 'katex';
import 'katex/dist/katex.min.css';
import {
  parseMatrix,
  parse0Y,
  analyze,
  zeroYToBMS,
  bmsTo0YSequence,
  matrixToLatex,
  parseAndEvalBOCF,
  expandBMS,
  bocfToBMS,
  termToVeblen,
} from './ts/bms.js';
import type { AnalysisResult, Matrix } from './ts/types.js';

const input = document.getElementById('input') as HTMLInputElement;
const output = document.getElementById('output') as HTMLDivElement;
const outputVeblen = document.getElementById('output-veblen') as HTMLDivElement;
const output0y = document.getElementById('output-0y') as HTMLSpanElement;
const outputBms = document.getElementById('output-bms') as HTMLSpanElement;
const bmsOutputRow = document.getElementById('bms-output-row') as HTMLDivElement;
const outputAst = document.getElementById('output-ast') as HTMLPreElement;

const modeVBtn = document.getElementById('mode-v') as HTMLButtonElement;
const modeMBtn = document.getElementById('mode-m') as HTMLButtonElement;
const sugarToggle = document.getElementById('sugar-toggle') as HTMLInputElement;

const inputModeBmsBtn = document.getElementById('input-mode-bms') as HTMLButtonElement;
const inputMode0yBtn = document.getElementById('input-mode-0y') as HTMLButtonElement;
const inputModeBocfBtn = document.getElementById('input-mode-bocf') as HTMLButtonElement;

const expandRow = document.getElementById('expand-row') as HTMLDivElement;
const expandBtn = document.getElementById('expand-btn') as HTMLButtonElement;
const expandFs = document.getElementById('expand-fs') as HTMLInputElement;
const outputExpand = document.getElementById('output-expand') as HTMLSpanElement;

const bmsMatrixBtn = document.getElementById('bms-mode-matrix') as HTMLButtonElement;
const bmsFlatBtn = document.getElementById('bms-mode-flat') as HTMLButtonElement;
const bocfToBmsBtn = document.getElementById('bocf-to-bms-btn') as HTMLButtonElement;
const bmsStatus = document.getElementById('bms-status') as HTMLSpanElement;

type VeblenMode = 'v' | 'm';
type InputMode = 'bms' | '0y' | 'bocf';
type BmsDisplayMode = 'matrix' | 'flat';

let currentVeblenMode: VeblenMode = 'v';
let currentInputMode: InputMode = 'bms';
let currentBmsMode: BmsDisplayMode = 'flat';
let lastResult: AnalysisResult | null = null;

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
  [inputModeBmsBtn, inputMode0yBtn, inputModeBocfBtn].forEach((b) => b.classList.remove('active'));
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
    expandRow.style.display = 'none';
    bocfToBmsBtn.style.display = 'none';
    bmsStatus.style.display = 'none';
  } else {
    inputModeBocfBtn.classList.add('active');
    input.placeholder = 'e.g. ψ(Ω) or \\psi(\\Omega)';
    bmsOutputRow.style.display = 'flex';
    outputAst.style.display = 'none';
    expandRow.style.display = 'none';
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
        outputVeblen.textContent = '';
        lastResult = null;
      }
      output0y.textContent = '';
      return;
    }

    let matrix: Matrix;

    if (currentInputMode === '0y') {
      const seq = parse0Y(input.value);
      if (seq.length === 0 || seq.some(isNaN)) throw new Error('Invalid 0-Y sequence');
      matrix = await zeroYToBMS(seq);
      const flat = matrixToDisplayStr(matrix);
      outputBms.setAttribute('data-raw', flat);
      renderBms(flat);
      output0y.textContent = '';
    } else {
      matrix = parseMatrix(input.value);
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
  }
}

modeVBtn.addEventListener('click', () => setActiveVeblenMode('v'));
modeMBtn.addEventListener('click', () => setActiveVeblenMode('m'));
sugarToggle.addEventListener('change', renderOutput);
input.addEventListener('input', update);
inputModeBmsBtn.addEventListener('click', () => setActiveInputMode('bms'));
inputMode0yBtn.addEventListener('click', () => setActiveInputMode('0y'));
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

expandBtn.addEventListener('click', async () => {
  try {
    const raw = parseInt(expandFs.value);
    const fs = isNaN(raw) ? 3 : raw;
    const matrix = parseMatrix(input.value);
    const expanded = await expandBMS(matrix, fs);
    outputExpand.textContent = matrixToDisplayStr(expanded);
  } catch {
    outputExpand.textContent = '(error)';
  }
});

setActiveInputMode('bms');
