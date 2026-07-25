/**
 * 1-Y / ω-Y Sequence expansion (WASM bridge)
 */
import { ensureLoaded, wasmModule } from './bms.js';
import type { Mountain, WYMountainResult } from './types.js';

/** Expand a 1-Y sequence by fs steps */
export async function expand1Y(seq: number[], fs: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.expand1Y(seq, fs);
}

/** Expand a ω-Y sequence by fs steps */
export async function expandWY(seq: number[], fs: number): Promise<number[]> {
  await ensureLoaded();
  return wasmModule.expandWY(seq, fs);
}

/** Build a WY mountain for display (n=1 for 1-Y, n=-1 for ω-Y)
 * Returns { layers, rows } where rows are ordinal string labels per layer */
export async function buildWYMountain(seq: number[], n: number, consistent: boolean = false): Promise<WYMountainResult> {
  await ensureLoaded();
  return wasmModule.buildWYMountain(seq, n, consistent);
}

/** Build a pure 1-Y mountain diagram (extraction-based, not from ω-Y truncation) */
export async function build1YMountain(seq: number[]): Promise<WYMountainResult> {
  await ensureLoaded();
  return wasmModule.build1YMountain(seq);
}
