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

export interface IHSSAnalysis {
  error?: string;
  matrix?: number[][];
  latex?: string;
  worm?: number[];
  limit?: boolean;
}

export async function ihssAnalyze(input: string): Promise<IHSSAnalysis> {
  await ensureLoaded();
  return wasmModule.ihssAnalyze(input);
}

export async function ihssExpand(input: string, n: number): Promise<string> {
  await ensureLoaded();
  return wasmModule.ihssExpand(input, n);
}

export async function ihssIsStandard(input: string): Promise<boolean> {
  await ensureLoaded();
  return wasmModule.ihssIsStandard(input);
}
