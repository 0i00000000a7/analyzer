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
