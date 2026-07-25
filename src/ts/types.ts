/** Types for the BMS analyzer */

/** A BMS matrix row - array of integers */
export type MatrixRow = number[];

/** A BMS matrix - array of rows */
export type Matrix = MatrixRow[];

/** Result of analyzing a matrix */
export interface AnalysisResult {
  gteEBO: boolean;
  ordinal: string;
  ordinalJS: any[][];
  veblen: string;
  veblenPlain: string;
  veblenMatrix: string;
  veblenMatrixPlain: string;
  nsForm: string;
  isStandard: boolean;
}

/** Type for the WASM module */
export interface BmsWasmModule {
  bmsAnalyze(matrix: Matrix): AnalysisResult;
  matrixLexOrder(a: Matrix, b: Matrix): number;
}

/** A node in the 0-Y mountain diagram */
export interface MountainNode {
  value: number;
  parent: number; // offset to parent (0 = none)
  parentCol?: number; // absolute column index of parent (-1 = none, for WY mountains)
}

/** The mountain diagram: layers of nodes (bottom layer = original sequence) */
export type Mountain = MountainNode[][];

/** WY mountain result: layers and corresponding row ordinal labels (reversed: most significant first) */
export interface WYMountainResult {
  layers: Mountain;
  rows: number[][];
}
