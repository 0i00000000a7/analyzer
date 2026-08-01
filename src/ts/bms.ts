/**
 * BMS Analyzer - TypeScript wrapper around Rust/WASM core
 */

import type { Matrix, AnalysisResult, Mountain } from './types.js';
import initWasm, * as wasm from '../wasm/pkg/bms_wasm.js';
import BmsWorker from './bms-worker.ts?worker';

export { ensureLoaded, wasmModule };
let wasmModule: any = null;
let loadPromise: Promise<void> | null = null;

async function ensureLoaded(): Promise<void> {
  if (wasmModule) return;
  if (loadPromise) return loadPromise;

  loadPromise = initWasm().then(() => {
    wasmModule = wasm;
  });

  return loadPromise;
}

// ── Worker for long-running bocfToBMS ──
let bocfWorker: Worker | null = null;
let bocfWorkerReady: Promise<void> | null = null;
let bocfReqId = 0;
const bocfPending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();

function getBocfWorker(): Promise<Worker> {
  if (bocfWorker) return Promise.resolve(bocfWorker);
  if (bocfWorkerReady) return bocfWorkerReady.then(() => bocfWorker!);

  bocfWorkerReady = new Promise<void>((resolve, reject) => {
    const worker = new BmsWorker();
    bocfWorker = worker;
    worker.onmessage = (e: MessageEvent) => {
      const msg = e.data;
      if (msg.type === 'init') {
        resolve();
        return;
      }
      if (msg.type === 'init_error') {
        reject(new Error('Worker init: ' + msg.error));
        return;
      }
      const pending = bocfPending.get(msg.id);
      if (!pending) return;
      if (msg.type === 'result') {
        bocfPending.delete(msg.id);
        if (msg.error) pending.reject(new Error(msg.error));
        else pending.resolve(msg.result);
      } else if (msg.type === 'error') {
        bocfPending.delete(msg.id);
        pending.reject(new Error(msg.error));
      }
    };
    worker.onerror = (e: ErrorEvent) => {
      reject(new Error('Worker error: ' + e.message));
    };
    worker.postMessage({ type: 'init', id: 0 });
  });

  return bocfWorkerReady.then(() => bocfWorker!);
}

export function parseMatrix(input: string): Matrix {
  let cleaned = input.replace(/[^\(\)（），,\d]/g, '');
  cleaned = cleaned.replace(/[（]/g, '(');
  cleaned = cleaned.replace(/[）]/g, ')');
  cleaned = cleaned.replace(/[，]/g, ',');
  const jsonStr = '[' + cleaned.replaceAll(')(', '],[').replaceAll('(', '[').replaceAll(')', ']') + ']';
  const parsed = JSON.parse(jsonStr);
  const maxCols = parsed.reduce((max: number, row: number[]) => Math.max(max, row.length), 0);
  return parsed.map((row: number[]) => {
    const r = row.slice();
    while (r.length < maxCols) r.push(0);
    return r;
  });
}

export const EBO_MATRIX: Matrix = [
  [0, 0, 0],
  [1, 1, 1],
  [2, 1, 1],
  [3, 1, 0],
  [2, 0, 0],
];

export async function analyze(matrix: Matrix): Promise<AnalysisResult> {
  await ensureLoaded();
  return wasmModule.bmsAnalyze(matrix);
}

export function analyzeSync(matrix: Matrix): AnalysisResult {
  if (!wasmModule) {
    throw new Error('WASM module not loaded yet.');
  }
  return wasmModule.bmsAnalyze(matrix);
}

export async function matrixLexOrder(a: Matrix, b: Matrix): Promise<number> {
  await ensureLoaded();
  return wasmModule.matrixLexOrder(a, b);
}

export async function init(): Promise<void> {
  await ensureLoaded();
}

/** Convert a BMS matrix to its 0-Y sequence string via WASM */
export async function bmsTo0YSequence(matrix: Matrix): Promise<string> {
  await ensureLoaded();
  return wasmModule.bmsTo0YSequence(matrix);
}

/** Parse and evaluate a BOCF expression via WASM, returning {ast, ordinal, ordinalJS, error} */
export async function parseAndEvalBOCF(
  input: string,
): Promise<{ ast: string; ordinal: string; ordinalJS: any[]; error: string }> {
  await ensureLoaded();
  return wasmModule.parseAndEvalBOCF(input);
}

/** Expand a BMS matrix by `fs` steps via WASM */
export async function expandBMS(matrix: Matrix, fs: number): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.expandBMS(matrix, fs);
}

/** Expand a UPMS matrix by `fs` steps via WASM */
export async function expandUPMS(matrix: Matrix, fs: number): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.expandUPMS(matrix, fs);
}

/** Check UPMS legality via WASM */
export async function isLegalUPMSMatrix(matrix: Matrix): Promise<boolean> {
  await ensureLoaded();
  return wasmModule.isLegalUPMSMatrix(matrix);
}

/** Convert UPMS expression to BMS via WASM */
export async function upmsToBMS(matrix: Matrix): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.upmsToBMS(matrix);
}

/** Convert BMS expression to UPMS via WASM */
export async function bmsToUPMS(matrix: Matrix): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.bmsToUPMS(matrix);
}

/** Parse a UPMS string into a matrix via WASM */
export async function parseUPMS(input: string): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.parseUPMS(input);
}

/** Format a matrix as a compact UPMS string via WASM */
export async function formatUPMS(matrix: Matrix): Promise<string> {
  await ensureLoaded();
  return wasmModule.formatUPMS(matrix);
}

/** Convert an ordinal term (JS array) to Veblen representations */
export async function termToVeblen(term: any[]): Promise<{
  veblen: string;
  veblenPlain: string;
  veblenMatrix: string;
  veblenMatrixPlain: string;
}> {
  await ensureLoaded();
  return wasmModule.termToVeblen(term);
}

export async function fundamentalSequence(term: any[], n: number): Promise<{ term: string; termJS: any[] }> {
  await ensureLoaded();
  return wasmModule.fundamentalSequence(term, n);
}

export async function cofinality(term: any[]): Promise<{ term: string; termJS: any[] }> {
  await ensureLoaded();
  return wasmModule.cofinality(term);
}

/** Convert a BOCF expression string to BMS matrix representation via WASM (runs in Worker) */
export async function bocfToBMS(input: string, onProgress?: (s: string) => void): Promise<string> {
  const worker = await getBocfWorker();
  const id = ++bocfReqId;

  const progressHandler = (e: MessageEvent) => {
    const msg = e.data;
    if (msg.type === 'progress' && msg.id === id && onProgress) {
      onProgress(msg.data);
    }
  };
  worker.addEventListener('message', progressHandler);

  return new Promise<string>((resolve, reject) => {
    bocfPending.set(id, {
      resolve: (r: string) => {
        worker.removeEventListener('message', progressHandler);
        resolve(r);
      },
      reject: (e: Error) => {
        worker.removeEventListener('message', progressHandler);
        reject(e);
      },
    });
    worker.postMessage({ type: 'bocfToBMS', id, input });
  });
}

export function cancelBocfToBMS() {
  if (bocfWorker) {
    bocfWorker.terminate();
    bocfWorker = null;
    bocfWorkerReady = null;
  }
  for (const [, p] of bocfPending) {
    p.reject(new Error('Cancelled'));
  }
  bocfPending.clear();
}

export function matrixToDisplayStr(matrix: Matrix): string {
  return matrix.map((col) => '(' + col.join(',') + ')').join('');
}

/** Format a BMS matrix as a LaTeX pmatrix (transposed so columns become rows) */
export function matrixToLatex(matrix: Matrix): string {
  if (matrix.length === 0) return '';
  const rows = matrix[0].length;
  const cols = matrix.length;
  let latex = '\\begin{pmatrix}';
  for (let r = 0; r < rows; r++) {
    if (r > 0) latex += '\\\\';
    for (let c = 0; c < cols; c++) {
      if (c > 0) latex += '&';
      latex += String(matrix[c][r]);
    }
  }
  latex += '\\end{pmatrix}';
  return latex;
}
