/**
 * 0-Y sequence utilities — WASM wrappers
 */
import type { Matrix, Mountain } from './types.js';
import { ensureLoaded, wasmModule } from './bms.js';

/** Expand a 0-Y sequence by n steps via WASM */
export async function zeroYExpand(seq: number[], n: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.zeroYExpand(seq, n);
}

/** Parse a 0-Y sequence string (comma-separated integers) into a number array */
export function parse0Y(input: string): number[] {
  return input.split(',').map((x) => parseInt(x.trim()));
}

/** Convert a 0-Y sequence (number array) to a BMS matrix via WASM */
export async function zeroYToBMS(seq: number[]): Promise<Matrix> {
  await ensureLoaded();
  return wasmModule.zeroYToBMS(seq);
}

/** Convert triangular BMS to standard BMS via WASM */
export async function buildMountain(seq: number[]): Promise<Mountain> {
  await ensureLoaded();
  return wasmModule.buildMountain(seq);
}
