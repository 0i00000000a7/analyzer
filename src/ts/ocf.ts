import initWasm, * as wasm from '../wasm/pkg/bms_wasm.js';

let wasmModule: any = null;
let loadPromise: Promise<void> | null = null;

async function ensureLoaded(): Promise<void> {
  if (wasmModule) return;
  if (!loadPromise) {
    loadPromise = initWasm().then(() => { wasmModule = wasm; });
  }
  await loadPromise;
}

export interface OcfAnalysis {
  error?: string;
  latex?: string;
}

export async function nocfAnalyze(input: string, sugar?: boolean): Promise<OcfAnalysis> {
  await ensureLoaded();
  const r = sugar ? wasmModule.nocfAnalyzeSugar(input) : wasmModule.nocfAnalyze(input);
  if (r.startsWith('!')) return { error: r.slice(1) };
  return { latex: r };
}

export async function nocfExpand(input: string, fs: number): Promise<string> {
  await ensureLoaded();
  return wasmModule.nocfExpand(input, fs);
}

export async function mocfAnalyze(input: string): Promise<OcfAnalysis> {
  await ensureLoaded();
  const r = wasmModule.mocfAnalyze(input);
  if (r.startsWith('!')) return { error: r.slice(1) };
  return { latex: r };
}

export async function mocfExpand(input: string, fs: number): Promise<string> {
  await ensureLoaded();
  return wasmModule.mocfExpand(input, fs);
}

export async function bocfToMocf(input: string): Promise<OcfAnalysis> {
  await ensureLoaded();
  const r = wasmModule.bocfToMocf(input);
  if (r.startsWith('!')) return { error: r.slice(1) };
  return { latex: r };
}