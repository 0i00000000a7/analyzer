/**
 * Triangular BMS conversion — WASM wrappers
 */
import type { Matrix } from './types.js';
import { ensureLoaded, wasmModule } from './bms.js';

export async function triangularToBMS(matrix: Matrix): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.triangularToBMS(matrix);
}

/** Convert standard BMS to triangular BMS via WASM */
export async function bmsToTriangular(matrix: Matrix): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.bmsToTriangular(matrix);
}
