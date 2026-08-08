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

export interface SSSAnalysis {
  error?: string;
  latex?: string;
}

export async function sssExpand(seq: number[], fs: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.sssExpand(seq, fs);
}

export async function sssToBocf(seq: number[]): Promise<SSSAnalysis> {
  await ensureLoaded();
  const r: string = wasmModule.sssToBocf(seq);
  if (r.startsWith('!')) {
    return { error: r.slice(1) };
  }
  return { latex: r };
}

export async function sssToNocf(seq: number[]): Promise<SSSAnalysis> {
  await ensureLoaded();
  const r: string = wasmModule.sssToNocf(seq);
  if (r.startsWith('!')) {
    return { error: r.slice(1) };
  }
  return { latex: r };
}

export async function sssToTprss(seq: number[]): Promise<SSSAnalysis> {
  await ensureLoaded();
  const r: string = wasmModule.sssToTprss(seq);
  if (r.startsWith('!')) {
    return { error: r.slice(1) };
  }
  return { latex: r };
}

export async function sssIsStandard(seq: number[]): Promise<boolean> {
  await ensureLoaded();
  return wasmModule.sssIsStandard(seq);
}

export function parseSSS(input: string): number[] {
  const tokens = input.trim().split(/[\s,]+/).filter(Boolean);
  if (tokens.length === 0) return [];
  const seq = tokens.map((t) => {
    const n = parseInt(t, 10);
    return Number.isNaN(n) ? 0 : n;
  });
  if (seq.length > 0 && seq[0] !== 0) seq.unshift(0);
  return seq;
}
