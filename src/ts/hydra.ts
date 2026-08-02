import initWasm, * as wasm from '../wasm/pkg/bms_wasm.js';

let wasmModule: any = null;
let loadPromise: Promise<void> | null = null;

async function ensureLoaded(): Promise<void> {
  if (wasmModule) return;
  if (!loadPromise) {
    loadPromise = initWasm().then(() => {
      wasmModule = wasm;
    });
  }
  await loadPromise;
}

export interface HydraAnalysis {
  error?: string;
  hydra?: string;
  legal?: boolean;
  ordinal?: string;
  ordinalJS?: any;
  veblen?: string;
  veblenPlain?: string;
  veblenMatrix?: string;
  veblenMatrixPlain?: string;
  bms?: number[][];
  hprss?: number[];
  lprss?: number[];
  zeroY?: string;
}

export async function hydraAnalyze(input: string): Promise<HydraAnalysis> {
  await ensureLoaded();
  return wasmModule.hydraAnalyze(input);
}

export async function hprssAnalyze(seq: number[]): Promise<HydraAnalysis> {
  await ensureLoaded();
  return wasmModule.hprssAnalyze(seq);
}

export async function lprssAnalyze(seq: number[]): Promise<HydraAnalysis> {
  await ensureLoaded();
  return wasmModule.lprssAnalyze(seq);
}

export async function expandHPRSS(seq: number[], n: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.expandHPRSS(seq, n);
}

export async function buildHPRSSMountain(seq: number[]): Promise<any> {
  await ensureLoaded();
  return wasmModule.buildHPRSSMountain(seq);
}

export async function expandLPRSS(seq: number[], n: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.expandLPRSS(seq, n);
}

export async function buildLPRSSMountain(seq: number[]): Promise<any> {
  await ensureLoaded();
  return wasmModule.buildLPRSSMountain(seq);
}

export async function expandHydra(input: string, n: number): Promise<string> {
  await ensureLoaded();
  return wasmModule.expandHydra(input, n);
}

export async function hprssToHydra(seq: number[]): Promise<string> {
  await ensureLoaded();
  return wasmModule.hprssToHydra(seq);
}

export async function hydraToHPRSS(input: string): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.hydraToHPRSS(input);
}

export async function hydraToBMS(input: string): Promise<number[][]> {
  await ensureLoaded();
  return wasmModule.hydraToBMS(input);
}

export async function bmsToHydra(matrix: number[][]): Promise<string> {
  await ensureLoaded();
  return wasmModule.bmsToHydra(matrix);
}

export async function bmsToHydraAnalysis(matrix: number[][]): Promise<{ hydra?: string; hprss?: number[]; lprss?: number[] }> {
  await ensureLoaded();
  return wasmModule.bmsToHydraAnalysis(matrix);
}
