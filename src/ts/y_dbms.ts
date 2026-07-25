/**
 * 1-Y / ω-Y ↔ DBMS conversion (WASM bridge)
 */
import { ensureLoaded, wasmModule } from './bms.js';

/** Convert a 1-Y / ω-Y sequence to DBMS matrix */
export async function oneYToDBMS(seq: number[]): Promise<number[][]> {
  await ensureLoaded();
  return wasmModule.oneYToDBMS(seq);
}

/** Format a DBMS matrix as a readable string like (0)(2,1,,1) */
export async function dbmsToString(dbms: number[][]): Promise<string> {
  await ensureLoaded();
  return wasmModule.dbmsToString(dbms);
}

/** Convert DBMS matrix to standard BMS */
export async function dbmsToBMS(dbms: number[][]): Promise<number[][]> {
  await ensureLoaded();
  return wasmModule.dbmsToBMS(dbms);
}
