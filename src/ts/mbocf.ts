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

export interface MboAstResult {
  error?: string;
  ast?: string;
  latex?: string;
}

export async function mboAst(input: string): Promise<MboAstResult> {
  await ensureLoaded();
  return wasmModule.mboAst(input);
}

export interface MboToIHSSResult {
  error?: string;
  matrix?: number[][];
  latex?: string;
  format?: string;
}

export async function mbocfToIHSS(input: string): Promise<MboToIHSSResult> {
  await ensureLoaded();
  return wasmModule.mbocfToIHSS(input);
}
