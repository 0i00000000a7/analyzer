import { ref, watch, computed, type Ref } from 'vue';
import katex from 'katex';
import {
  parseMatrix,
  analyze,
  bmsTo0YSequence,
  matrixToLatex,
  parseAndEvalBOCF,
  expandBMS,
  expandUPMS,
  isLegalUPMSMatrix,
  upmsIsStandard,
  bocfIsStandard,
  zeroYIsStandard,
  oneYIsStandard,
  wyIsStandard,
  hprssIsStandard,
  lprssIsStandard,
  upmsToBMS,
  bocfToBMS,
  cancelBocfToBMS,
  termToVeblen,
  fundamentalSequence,
  bmsIsStandard,
  bmsTriangularIsStandard,
} from '../ts/bms.js';
import { parse0Y, zeroYToBMS, zeroYExpand, buildMountain } from '../ts/bms-zero-y.js';
import { triangularToBMS, bmsToTriangular } from '../ts/bms-triangular.js';
import { expand1Y, expandWY, buildWYMountain, build1YMountain } from '../ts/wy.js';
import { oneYToDBMS, dbmsToString, dbmsToBMS } from '../ts/y_dbms.js';
import { hydraAnalyze, hprssAnalyze, lprssAnalyze, expandHPRSS, expandHydra, buildHPRSSMountain, bmsToHydraAnalysis, expandLPRSS, buildLPRSSMountain } from '../ts/hydra.js';
import { ihssAnalyze, ihssExpand, ihssIsStandard } from '../ts/ihss.js';
import { mboAst, mbocfToIHSS } from '../ts/mbocf.js';
import { sssExpand, sssToBocf, sssToNocf, sssToTprss, sssIsStandard, parseSSS } from '../ts/sss.js';
import { nocfAnalyze, nocfExpand, mocfAnalyze, mocfExpand, bocfToMocf, mocfToBocf } from '../ts/ocf.js';
import { hydraIsStandard, termToLmn } from '../ts/bms.js';
import { useI18n } from './useI18n';
import type { AnalysisResult, Matrix, Mountain } from '../ts/types.js';

export type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf' | 'upms' | 'hprss' | 'lprss' | 'hydra' | 'ihss' | 'mbo' | 'sss' | 'nocf' | 'mocf';
export type VeblenMode = 'v' | 'm';
export type BocfDisplayMode = 'normal' | 'psi';
export type LmnDisplayMode = 'full' | 'simple';
export type BmsDisplayMode = 'matrix' | 'flat' | 'compact';
export type UpmsDisplayMode = 'matrix' | 'flat' | 'compact';
export type BmsCompactStyle = 'brace' | 'alpha';
export type BmsInputPreference = 'auto' | 'normal' | 'triangular';
export type MboDisplayMode = 'matrix' | 'flat' | 'compact';
export type MboCompactStyle = 'brace' | 'alpha';

function parseCompactToken(token: string): number[] {
    const result: number[] = [];
    let i = 0;
    while (i < token.length) {
      if (token[i] === '{') {
        const end = token.indexOf('}', i);
        if (end === -1) break;
        result.push(parseInt(token.slice(i + 1, end), 10));
        i = end + 1;
      } else if (/[a-zA-Z]/.test(token[i])) {
        const code = token[i].toUpperCase().charCodeAt(0) - 55; // A=10..Z=35
        result.push(code);
        i++;
      } else if (/[0-9]/.test(token[i])) {
        result.push(parseInt(token[i], 10));
        i++;
      } else {
        // Invalid character — skip it rather than producing NaN
        i++;
      }
    }
    return result;
  }

  function transformInput(raw: string, mode: InputMode): string {
    const trimmed = raw.trim();
    if (!trimmed) return raw;
    if (trimmed.includes('(') || trimmed.includes(',')) return raw;
    const tokens = trimmed.split(/\s+/).filter(Boolean);
    if (tokens.length === 0) return raw;
    if (mode === 'bms' || mode === 'upms' || mode === 'ihss') return tokens.map((t) => '(' + parseCompactToken(t).join(',') + ')').join('');
    return tokens.join(',');
  }

function isTriangularMatrix(matrix: Matrix): boolean {
  if (matrix.length < 3) return false;
  const c0 = matrix[0], c1 = matrix[1], c2 = matrix[2];
  return (
    (c0[0] ?? 0) === 0 && (c0[1] ?? 0) === 0 && (c0[2] ?? 0) === 0 &&
    (c1[0] ?? 0) === 1 && (c1[1] ?? 0) === 0 && (c1[2] ?? 0) === 0 &&
    (c2[0] ?? 0) === 2 && (c2[1] ?? 0) === 1 && (c2[2] ?? 0) === 0
  );
}

function alignMatrixStr(s: string): string {
  if (s === '(empty)' || s === '(error)') return s;
  const cols: string[][] = [];
  s.replace(/\(([^)]+)\)/g, (_, inner: string) => {
    cols.push(inner.split(','));
    return '';
  });
  for (const col of cols) {
    while (col.length > 1 && col[col.length - 1] === '0') col.pop();
  }
  let maxLen = 0;
  for (const col of cols) if (col.length > maxLen) maxLen = col.length;
  for (const col of cols) while (col.length < maxLen) col.push('0');
  return cols.map((col) => '(' + col.join(',') + ')').join('');
}

function matrixToDisplayStr(m: Matrix): string {
  return m.map((col) => '(' + col.join(',') + ')').join('');
}

export function useAnalysis(
  inputMode: Ref<InputMode>,
  inputValue: Ref<string>,
  veblenMode: Ref<VeblenMode>,
  sugarEnabled: Ref<boolean>,
  bocfDisplayMode: Ref<BocfDisplayMode>,
  bmsDisplayMode: Ref<BmsDisplayMode>,
  upmsDisplayMode: Ref<UpmsDisplayMode>,
  bmsCompactStyle: Ref<BmsCompactStyle>,
  upmsCompactStyle: Ref<BmsCompactStyle>,
  bmsInputPref: Ref<BmsInputPreference>,
  mboDisplayMode: Ref<MboDisplayMode>,
  mboCompactStyle: Ref<MboCompactStyle>,
  mboSugar: Ref<boolean>,
  nocfSugar: Ref<boolean>,
  lmnDisplayMode: Ref<LmnDisplayMode>,
) {
  const { t } = useI18n();
  const ordinalHtml = ref('');
  const veblenHtml = ref('');
  const lmnHtml = ref('');
  const zeroYHtml = ref('');
  const dbmsHtml = ref('');
  const bmsHtml = ref('');
  const triangularHtml = ref('');
  const upmsHtml = ref('');
  const hydraHtml = ref('');
  const hprssHtml = ref('');
  const lprssHtml = ref('');
  const mboHtml = ref('');
  const mboMatrix = ref('');
  const mboAstHtml = ref('');
  const sssNocfHtml = ref('');
  const sssTprssHtml = ref('');
  const nocfHtml = ref('');
  const mocfHtml = ref('');
  const bocfMocfHtml = ref('');

  const showDbmsRow = ref(false);
  const showBmsRow = ref(false);
  const showTriangularRow = ref(false);
  const showUpmsRow = ref(false);
  const showMountainRow = ref(false);
  const showHydraRow = ref(false);
  const showHprssRow = ref(false);
  const showLprssRow = ref(false);
  const showMboRow = ref(false);
  const showMboAstRow = ref(false);
  const showSssNocfRow = ref(false);
  const showSssTprssRow = ref(false);

  const bmsRaw = ref('');
  const upmsRaw = ref('');
  const nonStandard = ref(false);
  const forceNonStandard = ref(false);
  const pendingMatrix = ref<Matrix | null>(null);
  const bocfNonStandardWarning = ref(false);
  const hydraNonStandardWarning = ref(false);
  const ihssNonStandardWarning = ref(false);
  const sssNonStandardWarning = ref(false);
  const mountainType = ref<'0y' | '1y' | 'wy' | 'hprss' | 'lprss' | 'hydra' | null>(null);
  const mountainData = ref<Mountain | null>(null);
  const mountainRowLabels = ref<number[][] | null>(null);

  let lastResult: AnalysisResult | null = null;
  let currentBocfOrdinal: any[] | null = null;
  let current0YSeq: number[] | null = null;
  let current1YSeq: number[] | null = null;
  let currentWYSeq: number[] | null = null;
  let currentUPMSMatrix: number[][] | null = null;
  let currentHPRSSSeq: number[] | null = null;
  const currentLPRSSSeq = ref<number[]>([]);
  let currentMBOcfInput = '';
  let currentMBOcfLatex = '';
  let currentMboInput = '';
  let currentSSSSeq: number[] | null = null;
  let currentNocfInput = '';
  let currentMocfInput = '';

  // BOCF conversion state
  const converting = ref(false);
  const convertStatus = ref('');

  // Expand state
  const expandResult = ref('');
  const expandFs = ref('3');

  function getVeblenOutput(r: AnalysisResult, mode: VeblenMode, sugar: boolean): string | null {
    const key = mode === 'v' ? (sugar ? 'veblen' : 'veblenPlain') : sugar ? 'veblenMatrix' : 'veblenMatrixPlain';
    return (r as any)[key] || null;
  }

  function getBocfOutput(r: AnalysisResult, mode: BocfDisplayMode): string | null {
    return mode === 'psi' ? (r.psiSimple || null) : (r.ordinal || null);
  }

  async function refreshLmnRow(): Promise<void> {
    const term = lastResult?.ordinalJS;
    if (!term || term.length === 0) {
      lmnHtml.value = '';
      return;
    }
    try {
      const l = await termToLmn(term);
      if (lmnDisplayMode.value === 'full') {
        lmnHtml.value = katex.renderToString(l.full, { throwOnError: false });
      } else {
        lmnHtml.value = katex.renderToString('\\text{' + l.bracket + '}', { throwOnError: false });
      }
    } catch {
      lmnHtml.value = '';
    }
  }

  function renderLmnZero(): void {
    lmnHtml.value = katex.renderToString(
      lmnDisplayMode.value === 'full' ? '\\psi_0(0)' : '\\text{0}',
      { throwOnError: false },
    );
  }

  function formatCompactVal(n: number, style: BmsCompactStyle): string {
    if (n >= 36) return `\\{${n}\\}`;
    if (style === 'brace' && n >= 10) return `\\{${n}\\}`;
    if (style === 'alpha' && n >= 10) return String.fromCharCode(55 + n); // A=10..Z=35
    return String(n);
  }

  function renderBms(raw: string): string {
    if (raw === '(empty)' || raw === '(error)') return raw;
    const aligned = alignMatrixStr(raw);
    if (bmsDisplayMode.value === 'matrix') {
      const matrix = parseMatrix(aligned);
      return katex.renderToString(matrixToLatex(matrix), { throwOnError: false });
    }
    if (bmsDisplayMode.value === 'compact') {
      const matrix = parseMatrix(aligned);
      const style = bmsCompactStyle.value;
      const compact = matrix.map(col => {
        const values = [...col];
        while (values.length > 1 && values[values.length - 1] === 0) values.pop();
        return values.map(v => formatCompactVal(v, style)).join('');
      }).join(' ');
      return katex.renderToString('\\text{' + compact + '}', { throwOnError: false });
    }
    return katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
  }

  function renderHydra(s: string): string {
  return katex.renderToString(s.replace(/ψ/g, '\\psi '), { throwOnError: false });
}

function renderUpms(raw: string): string {
    if (raw === '(empty)' || raw === '(error)') return raw;
    const aligned = alignMatrixStr(raw);
    if (upmsDisplayMode.value === 'matrix') {
      const matrix = parseMatrix(aligned);
      return katex.renderToString(matrixToLatex(matrix), { throwOnError: false });
    }
    if (upmsDisplayMode.value === 'compact') {
      const matrix = parseMatrix(aligned);
      const style = upmsCompactStyle.value;
      const compact = matrix.map(col => {
        const values = [...col];
        while (values.length > 1 && values[values.length - 1] === 0) values.pop();
        return values.map(v => formatCompactVal(v, style)).join('');
      }).join(' ');
      return katex.renderToString('\\text{' + compact + '}', { throwOnError: false });
    }
    return katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
  }

  function renderMbo(raw: string): string {
    if (raw === '(empty)' || raw === '(error)') return raw;
    const aligned = alignMatrixStr(raw);
    if (mboDisplayMode.value === 'matrix') {
      const matrix = parseMatrix(aligned);
      return katex.renderToString(matrixToLatex(matrix), { throwOnError: false });
    }
    if (mboDisplayMode.value === 'compact') {
      const matrix = parseMatrix(aligned);
      const style = mboCompactStyle.value;
      const compact = matrix.map(col => {
        const values = [...col];
        while (values.length > 1 && values[values.length - 1] === 0) values.pop();
        return values.map(v => formatCompactVal(v, style)).join('');
      }).join(' ');
      return katex.renderToString('\\text{' + compact + '}', { throwOnError: false });
    }
    return katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
  }

  function applyMboSugar(latex: string): string {
    if (!mboSugar.value) return latex;
    let s = latex;
    // ψ_M(M × n) → Ω_n (natural n ≥ 2)
    s = s.replace(/\\psi_{M}\(M \\times (\d+)\)/g, '\\Omega_{$1}');
    // ψ_M(M) → Ω (n = 1)
    s = s.replace(/\\psi_{M}\(M\)/g, '\\Omega');
    // ψ_Ω(1) → ω
    s = s.replace(/\\psi_{\\Omega}\(1\)/g, '\\omega');
    return s;
  }

  function clearOutputs() {
    ordinalHtml.value = '';
    veblenHtml.value = '';
    lmnHtml.value = '';
    zeroYHtml.value = '';
    dbmsHtml.value = '';
    bmsHtml.value = '';
    triangularHtml.value = '';
    upmsHtml.value = '';
    hydraHtml.value = '';
    hprssHtml.value = '';
    lprssHtml.value = '';
    mboHtml.value = '';
    mboMatrix.value = '';
    sssNocfHtml.value = '';
    sssTprssHtml.value = '';
    nocfHtml.value = '';
    mocfHtml.value = '';
    bocfMocfHtml.value = '';
    showDbmsRow.value = false;
    showBmsRow.value = false;
    showTriangularRow.value = false;
    showUpmsRow.value = false;
    showMountainRow.value = false;
    showHydraRow.value = false;
    showHprssRow.value = false;
    showLprssRow.value = false;
    showMboRow.value = false;
    showMboAstRow.value = false;
    showSssNocfRow.value = false;
    showSssTprssRow.value = false;
    bmsRaw.value = '';
    upmsRaw.value = '';
    mountainType.value = null;
    mountainData.value = null;
    mountainRowLabels.value = null;
    lastResult = null;
    currentBocfOrdinal = null;
    current0YSeq = null;
    current1YSeq = null;
    currentWYSeq = null;
    currentUPMSMatrix = null;
    currentHPRSSSeq = null;
    currentLPRSSSeq.value = [];
    currentMBOcfInput = '';
    currentMBOcfLatex = '';
    currentMboInput = '';
    nonStandard.value = false;
    bocfNonStandardWarning.value = false;
    hydraNonStandardWarning.value = false;
    ihssNonStandardWarning.value = false;
    sssNonStandardWarning.value = false;
    pendingMatrix.value = null;
  }

  async function checkSeqStandard(seq: number[], checker: (s: number[]) => Promise<boolean>): Promise<boolean> {
    if (forceNonStandard.value) return true;
    const std = await checker(seq);
    if (!std) {
      nonStandard.value = true;
      pendingMatrix.value = seq.map((v) => [v]);
      return false;
    }
    return true;
  }

  async function update(forceNonStd = false) {
    try {
      if (!forceNonStd) forceNonStandard.value = false;
      clearOutputs();
      const rawInput = transformInput(inputValue.value, inputMode.value);

      if (inputMode.value === 'bocf') {
        await handleBocf(rawInput);
        return;
      }

      let matrix: Matrix;

      if (inputMode.value === '0y') {
        await handle0Y(rawInput);
        return;
      }
      if (inputMode.value === '1y') {
        await handle1Y(rawInput);
        return;
      }
      if (inputMode.value === 'wy') {
        await handleWY(rawInput);
        return;
      }
      if (inputMode.value === 'upms') {
        await handleUPMS(rawInput);
        return;
      }
      if (inputMode.value === 'hprss') {
        await handleHPRSS(rawInput);
        return;
      }
      if (inputMode.value === 'lprss') {
        await handleLPRSS(rawInput);
        return;
      }
      if (inputMode.value === 'hydra') {
        await handleHydra(rawInput);
        return;
      }
      if (inputMode.value === 'ihss') {
        await handleIHSS(rawInput);
        return;
      }
      if (inputMode.value === 'mbo') {
        await handleMboAst(rawInput);
        return;
      }
      if (inputMode.value === 'sss') {
        await handleSSS(rawInput);
        return;
      }
      if (inputMode.value === 'nocf') {
        await handleNocf(rawInput);
        return;
      }
      if (inputMode.value === 'mocf') {
        await handleMocf(rawInput);
        return;
      }

      // BMS mode
      matrix = parseMatrix(rawInput);
      const bmsFlat = matrixToDisplayStr(matrix);
      bmsRaw.value = bmsFlat;
      bmsHtml.value = renderBms(bmsFlat);
      showBmsRow.value = true;
      const pref = bmsInputPref.value;
      const isTri = matrix.length >= 3 && isTriangularMatrix(matrix);

      if (pref === 'normal' || (pref === 'auto' && !isTri)) {
        // Normal BMS input → show Tri BMS conversion
        const triMatrix = await bmsToTriangular(matrix);
        if (triMatrix && triMatrix.length > 0) {
          showTriangularRow.value = true;
          const raw = matrixToDisplayStr(triMatrix);
          const aligned = alignMatrixStr(raw);
          triangularHtml.value = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
        }
      } else if (pref === 'triangular' || (pref === 'auto' && isTri)) {
        // Triangular BMS input → check standardness of the triangular form, then convert
        if (isTri) {
          if (!forceNonStandard.value) {
            const triStd = await bmsTriangularIsStandard(matrix);
            if (!triStd) {
              nonStandard.value = true;
              pendingMatrix.value = matrix;
              return;
            }
          }
          matrix = await triangularToBMS(matrix);
        }
      }

      const r = await analyze(matrix);
      if (!r.isStandard && !forceNonStandard.value) {
        nonStandard.value = true;
        pendingMatrix.value = matrix;
        return;
      }
      nonStandard.value = false;
      lastResult = r;
      const bocfOut = getBocfOutput(r, bocfDisplayMode.value);
      ordinalHtml.value = bocfOut ? katex.renderToString(bocfOut, { throwOnError: false }) : '';
      refreshLmnRow();
      const seq = await bmsTo0YSequence(matrix);
      zeroYHtml.value = seq ? katex.renderToString(seq, { throwOnError: false }) : '';

      if (r.veblen && !r.gteEBO) {
        const v = getVeblenOutput(r, veblenMode.value, sugarEnabled.value);
        if (v) veblenHtml.value = katex.renderToString(v, { throwOnError: false });
      }

      const ha = await bmsToHydraAnalysis(matrix);
      if (ha.hydra) {
        hydraHtml.value = renderHydra(ha.hydra);
        showHydraRow.value = true;
      }
      if (ha.hprss && ha.hprss.length > 0) {
        hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
        showHprssRow.value = true;
      }
      if (ha.lprss && ha.lprss.length > 0) {
        lprssHtml.value = katex.renderToString('\\text{' + ha.lprss.join(',') + '}', { throwOnError: false });
        showLprssRow.value = true;
      }
    } catch {
      ordinalHtml.value = '(error)';
    }
  }

  async function handleBocf(rawInput: string) {
    const r = await parseAndEvalBOCF(rawInput);
    if (r.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    if (r.ordinalJS) {
      currentBocfOrdinal = r.ordinalJS;

      // Check standardness with intermediate warning level
      if (!forceNonStandard.value) {
        const std = await bocfIsStandard(rawInput);
        if (std === 2) {
          nonStandard.value = true;
          pendingMatrix.value = [[]];
          return;
        } else if (std === 1) {
          bocfNonStandardWarning.value = true;
        }
      }

      const v = await termToVeblen(r.ordinalJS);
      lastResult = { gteEBO: false, ordinal: r.ordinal, ordinalJS: r.ordinalJS, psiSimple: r.psiSimple, ...v, nsForm: '', isStandard: true } as AnalysisResult;
      const bocfOut = getBocfOutput(lastResult, bocfDisplayMode.value);
      ordinalHtml.value = bocfOut ? katex.renderToString(bocfOut, { throwOnError: false }) : '';
      if (lastResult.veblen) {
        const veblenOut = getVeblenOutput(lastResult, veblenMode.value, sugarEnabled.value);
        if (veblenOut) veblenHtml.value = katex.renderToString(veblenOut, { throwOnError: false });
      }
      refreshLmnRow();
      const mocf = await bocfToMocf(rawInput);
      if (mocf.error) {
        bocfMocfHtml.value = '';
      } else if (mocf.latex) {
        bocfMocfHtml.value = katex.renderToString(mocf.latex, { throwOnError: false });
      }
    }
    showBmsRow.value = true;
    try {
      const bms = await bocfToBMS(rawInput, () => {});
      if (bms !== '(empty)') {
        const bmsMatrix = parseMatrix(bms);
        const ha = await bmsToHydraAnalysis(bmsMatrix);
        if (ha.hydra) {
          hydraHtml.value = renderHydra(ha.hydra);
          showHydraRow.value = true;
        }
        if (ha.hprss && ha.hprss.length > 0) {
          hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
          showHprssRow.value = true;
        }
        if (ha.lprss && ha.lprss.length > 0) {
          lprssHtml.value = katex.renderToString('\\text{' + ha.lprss.join(',') + '}', { throwOnError: false });
          showLprssRow.value = true;
        }
      }
    } catch {}
  }

  async function handle0Y(rawInput: string) {
    if (rawInput === '') {
      // Continue to analysis with empty matrix
      const matrix: Matrix = [];
      const r = await analyze(matrix);
      lastResult = r;
      const bocfOut = getBocfOutput(r, bocfDisplayMode.value);
      ordinalHtml.value = bocfOut ? katex.renderToString(bocfOut, { throwOnError: false }) : '';
      refreshLmnRow();
      bmsRaw.value = '';
      bmsHtml.value = '';
      showBmsRow.value = true;
      return;
    }
    const seq = parse0Y(rawInput);
    if (seq.length === 0 || seq.some(isNaN)) return;
    if (!(await checkSeqStandard(seq, zeroYIsStandard))) return;

    current0YSeq = seq;
    const matrix = await zeroYToBMS(seq);
    const flat = matrixToDisplayStr(matrix);
    bmsRaw.value = flat;
    bmsHtml.value = renderBms(flat);
    showBmsRow.value = true;

    const r = await analyze(matrix);
    lastResult = r;
    ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });
    refreshLmnRow();

    if (r.veblen && !r.gteEBO) {
      const v = getVeblenOutput(r, veblenMode.value, sugarEnabled.value);
      if (v) veblenHtml.value = katex.renderToString(v, { throwOnError: false });
    }

    try {
      const ha = await bmsToHydraAnalysis(matrix);
      if (ha.hydra) {
        hydraHtml.value = renderHydra(ha.hydra);
        showHydraRow.value = true;
      }
      if (ha.hprss && ha.hprss.length > 0) {
        hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
        showHprssRow.value = true;
      }
      if (ha.lprss && ha.lprss.length > 0) {
        lprssHtml.value = katex.renderToString('\\text{' + ha.lprss.join(',') + '}', { throwOnError: false });
        showLprssRow.value = true;
      }
    } catch {}

    try {
      const mountain = await buildMountain(seq);
      if (mountain.length) {
        mountainData.value = mountain;
        mountainType.value = '0y';
        mountainRowLabels.value = null;
        showMountainRow.value = true;
      }
    } catch {}
  }

  async function handle1Y(rawInput: string) {
    let seq = parse0Y(rawInput);
    if (seq.some(isNaN) || rawInput === '') seq = [0];
    current1YSeq = seq;
    if (rawInput !== '' && !(await checkSeqStandard(seq, oneYIsStandard))) return;

    try {
      const result = await build1YMountain(seq);
      mountainData.value = result.layers;
      mountainRowLabels.value = result.rows;
      mountainType.value = '1y';
      showMountainRow.value = true;
    } catch {}

    try {
      const dbms = await oneYToDBMS(seq);
      const dbmsStr = await dbmsToString(dbms);
      const hasOmega = dbmsStr.includes(',,');
      const displayStr = hasOmega ? '\\geq\\text{' + dbmsStr + '}' : '\\text{' + dbmsStr + '}';
      dbmsHtml.value = katex.renderToString(displayStr, { throwOnError: false });
      showDbmsRow.value = true;

      if (dbms.length > 0 && !hasOmega) {
        const bms = await dbmsToBMS(dbms);
        if (bms.length > 0) {
          const flat = bms.map((c) => '(' + c.join(',') + ')').join('');
          bmsRaw.value = flat;
          bmsHtml.value = renderBms(flat);
          showBmsRow.value = true;
          const r = await analyze(bms);
          lastResult = r;
          ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });
          if (r.veblen) veblenHtml.value = katex.renderToString(r.veblen, { throwOnError: false });
          refreshLmnRow();
          const seq0y = await bmsTo0YSequence(bms);
          zeroYHtml.value = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
        } else {
          ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
          veblenHtml.value = katex.renderToString('0', { throwOnError: false });
          renderLmnZero();
        }
      } else if (hasOmega) {
        // no BMS for omega
      } else {
        ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
        veblenHtml.value = katex.renderToString('0', { throwOnError: false });
          renderLmnZero();
      }
    } catch {}
  }

  async function handleWY(rawInput: string) {
    let seq = parse0Y(rawInput);
    if (seq.some(isNaN) || rawInput === '') seq = [0];
    currentWYSeq = seq;
    if (rawInput !== '' && !(await checkSeqStandard(seq, wyIsStandard))) return;

    try {
      const result = await buildWYMountain(seq, -1);
      mountainData.value = result.layers;
      mountainRowLabels.value = result.rows;
      mountainType.value = 'wy';
      showMountainRow.value = true;
    } catch {}

    try {
      const dbms = await oneYToDBMS(seq);
      const dbmsStr = await dbmsToString(dbms);
      const hasOmega = dbmsStr.includes(',,');
      const displayStr = hasOmega ? '\\geq\\text{' + dbmsStr + '}' : '\\text{' + dbmsStr + '}';
      dbmsHtml.value = katex.renderToString(displayStr, { throwOnError: false });
      showDbmsRow.value = true;

      if (dbms.length > 0 && !hasOmega) {
        const bms = await dbmsToBMS(dbms);
        if (bms.length > 0) {
          const flat = bms.map((c) => '(' + c.join(',') + ')').join('');
          bmsRaw.value = flat;
          bmsHtml.value = renderBms(flat);
          showBmsRow.value = true;
          const r = await analyze(bms);
          lastResult = r;
          ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });
          if (r.veblen) veblenHtml.value = katex.renderToString(r.veblen, { throwOnError: false });
          refreshLmnRow();
          const seq0y = await bmsTo0YSequence(bms);
          zeroYHtml.value = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
        } else {
          ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
          veblenHtml.value = katex.renderToString('0', { throwOnError: false });
          renderLmnZero();
        }
      } else if (hasOmega) {
        // no BMS for omega
      } else {
        ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
        veblenHtml.value = katex.renderToString('0', { throwOnError: false });
          renderLmnZero();
      }
    } catch {}
  }

  async function handleUPMS(rawInput: string) {
    if (!rawInput.trim()) return;
    // Reuse BMS parser for UPMS - transformInput handles "0 11" -> "(0)(1,1)"
    const transformed = transformInput(rawInput, 'bms');
    const expr = parseMatrix(transformed);
    if (expr.length === 0) return;
    currentUPMSMatrix = expr;

    if (!forceNonStandard.value) {
      const std = await upmsIsStandard(expr);
      if (!std) {
        nonStandard.value = true;
        pendingMatrix.value = expr;
        return;
      }
    }

    // Use UPMS-specific formatting
    const upmsFlat = matrixToDisplayStr(expr);
    upmsRaw.value = upmsFlat;
    upmsHtml.value = renderUpms(upmsFlat);
    showUpmsRow.value = true;

    try {
      const bmsExpr = await upmsToBMS(expr);
      const matrix = bmsExpr;
      if (matrix.length > 0) {
        const flat = matrixToDisplayStr(matrix);
        bmsRaw.value = flat;
        bmsHtml.value = renderBms(flat);
        showBmsRow.value = true;

        const r = await analyze(matrix);
        lastResult = r;
        ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });
        refreshLmnRow();

        if (r.veblen && !r.gteEBO) {
          const v = getVeblenOutput(r, veblenMode.value, sugarEnabled.value);
          if (v) veblenHtml.value = katex.renderToString(v, { throwOnError: false });
        }

        const seq = await bmsTo0YSequence(matrix);
        zeroYHtml.value = seq ? katex.renderToString(seq, { throwOnError: false }) : '';

        const triMatrix = await bmsToTriangular(matrix);
        if (triMatrix && triMatrix.length > 0) {
          showTriangularRow.value = true;
          const raw = matrixToDisplayStr(triMatrix);
          const aligned = alignMatrixStr(raw);
          triangularHtml.value = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
        }
      }
    } catch (e) {
      console.error('UPMS conversion error:', e);
      ordinalHtml.value = '(error: ' + (e as Error).message + ')';
    }
  }

  async function handleHPRSS(rawInput: string) {
    const seq = parse0Y(rawInput);
    if (seq.length === 0 || seq.some(isNaN)) return;
    if (!(await checkSeqStandard(seq, hprssIsStandard))) return;
    currentHPRSSSeq = seq;

    const ha = await hprssAnalyze(seq);
    if (ha.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    ordinalHtml.value = ha.ordinal ? katex.renderToString(ha.ordinal, { throwOnError: false }) : '';

    if (ha.veblen) {
      veblenHtml.value = katex.renderToString(ha.veblen, { throwOnError: false });
    }

    if (ha.hydra) {
      hydraHtml.value = renderHydra(ha.hydra);
      showHydraRow.value = true;
    }

    // Show HPRSS sequence
    if (ha.hprss && ha.hprss.length > 0) {
      hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
      showHprssRow.value = true;
    }

    // Show BMS conversion
    if (ha.bms && ha.bms.length > 0) {
      const flat = matrixToDisplayStr(ha.bms);
      bmsRaw.value = flat;
      bmsHtml.value = renderBms(flat);
      showBmsRow.value = true;
    }

    // Mountain
    try {
      const mountain = await buildHPRSSMountain(seq);
      if (mountain && mountain.length) {
        mountainData.value = mountain;
        mountainType.value = 'hprss';
        mountainRowLabels.value = null;
        showMountainRow.value = true;
      }
    } catch {}
  }

  async function handleLPRSS(rawInput: string) {
    const seq = parse0Y(rawInput);
    if (seq.length === 0 || seq.some(isNaN)) return;
    if (!(await checkSeqStandard(seq, lprssIsStandard))) return;
    currentLPRSSSeq.value = seq;

    const ha = await lprssAnalyze(seq);
    if (ha.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    ordinalHtml.value = ha.ordinal ? katex.renderToString(ha.ordinal, { throwOnError: false }) : '';

    if (ha.veblen) {
      veblenHtml.value = katex.renderToString(ha.veblen, { throwOnError: false });
    }

    // Show hydra
    if (ha.hydra) {
      hydraHtml.value = renderHydra(ha.hydra);
      showHydraRow.value = true;
    }

    // Show HPRSS
    if (ha.hprss && ha.hprss.length > 0) {
      hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
      showHprssRow.value = true;
    }

    // Show LPRSS
    if (ha.lprss && ha.lprss.length > 0) {
      lprssHtml.value = katex.renderToString('\\text{' + ha.lprss.join(',') + '}', { throwOnError: false });
      showLprssRow.value = true;
    }

    // Show BMS conversion
    if (ha.bms && ha.bms.length > 0) {
      const flat = matrixToDisplayStr(ha.bms);
      bmsRaw.value = flat;
      bmsHtml.value = renderBms(flat);
      showBmsRow.value = true;
    }

    // Mountain
    try {
      const mountain = await buildLPRSSMountain(seq);
      if (mountain && mountain.length) {
        mountainData.value = mountain;
        mountainType.value = 'lprss';
        mountainRowLabels.value = null;
        showMountainRow.value = true;
      }
    } catch {}
  }

  async function handleHydra(rawInput: string) {
    const ha = await hydraAnalyze(rawInput);
    if (ha.error) {
      ordinalHtml.value = '(error)';
      return;
    }

    // Check standardness with intermediate warning level
    if (!forceNonStandard.value) {
      const std = await hydraIsStandard(rawInput);
      if (std === 2) {
        nonStandard.value = true;
        pendingMatrix.value = [[]];
        return;
      } else if (std === 1) {
        hydraNonStandardWarning.value = true;
      }
    }

    ordinalHtml.value = ha.ordinal ? katex.renderToString(ha.ordinal, { throwOnError: false }) : '';

    if (ha.veblen) {
      veblenHtml.value = katex.renderToString(ha.veblen, { throwOnError: false });
    }

    if (ha.hydra) {
      hydraHtml.value = renderHydra(ha.hydra);
      showHydraRow.value = true;
    }

    if (ha.hprss && ha.hprss.length > 0) {
      hprssHtml.value = katex.renderToString('\\text{' + ha.hprss.join(',') + '}', { throwOnError: false });
      showHprssRow.value = true;
    }

    if (ha.lprss && ha.lprss.length > 0) {
      lprssHtml.value = katex.renderToString('\\text{' + ha.lprss.join(',') + '}', { throwOnError: false });
      showLprssRow.value = true;
    }

    // Show BMS conversion
    if (ha.bms && ha.bms.length > 0) {
      const flat = matrixToDisplayStr(ha.bms);
      bmsRaw.value = flat;
      bmsHtml.value = renderBms(flat);
      showBmsRow.value = true;
    }
  }

  // IHSS Hydra input. No analysis to other notations yet — only the
  // Mahlo-BOCF formatting, the IHSS value matrix, the worm, and expansion.
  async function handleIHSS(rawInput: string) {
    const ih = await ihssAnalyze(rawInput);
    if (ih.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    currentMBOcfInput = rawInput;
    try {
      ihssNonStandardWarning.value = !(await ihssIsStandard(rawInput));
    } catch {
      ihssNonStandardWarning.value = false;
    }
    if (ih.latex) {
      currentMBOcfLatex = ih.latex;
      mboHtml.value = katex.renderToString(applyMboSugar(ih.latex), { throwOnError: false });
      showMboRow.value = true;
    }
    if (ih.matrix && ih.matrix.length > 0) {
      mboMatrix.value = renderMbo(matrixToDisplayStr(ih.matrix));
      showMboRow.value = true;
    }
  }

  // Mahlo BOCF page: parse AST and reverse-construct the IHSS value matrix.
  async function handleMboAst(rawInput: string) {
    const r = await mboAst(rawInput);
    if (r.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    if (r.ast) {
      mboAstHtml.value = r.ast;
      showMboAstRow.value = true;
    }
    const ih = await mbocfToIHSS(rawInput);
    if (ih.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    currentMboInput = ih.format || '';
    if (ih.matrix && ih.matrix.length > 0) {
      mboMatrix.value = renderMbo(matrixToDisplayStr(ih.matrix));
      showMboRow.value = true;
    }
  }

  // SSS page: convert the sequence to its BOCF and NOCF ordinals.
  async function handleSSS(rawInput: string) {
    const seq = parseSSS(rawInput);
    if (seq.length === 0) return;
    currentSSSSeq = seq;
    try {
      sssNonStandardWarning.value = !(await sssIsStandard(seq));
    } catch {
      sssNonStandardWarning.value = false;
    }
    const r = await sssToBocf(seq);
    if (r.error) {
      ordinalHtml.value = '(error)';
      return;
    }
    ordinalHtml.value = r.latex
      ? katex.renderToString(r.latex, { throwOnError: false })
      : '';
    const n = await sssToNocf(seq);
    if (!n.error && n.latex) {
      sssNocfHtml.value = katex.renderToString(n.latex, { throwOnError: false });
      showSssNocfRow.value = true;
    }
    const tp = await sssToTprss(seq);
    if (!tp.error && tp.latex) {
      sssTprssHtml.value = katex.renderToString('\\text{' + tp.latex + '}', { throwOnError: false });
      showSssTprssRow.value = true;
    }
  }

  async function handleNocf(rawInput: string) {
    if (!rawInput.trim()) return;
    currentNocfInput = rawInput.trim();
    const r = await nocfAnalyze(rawInput, nocfSugar.value);
    if (r.error) {
      nocfHtml.value = '(error)';
      return;
    }
    nocfHtml.value = katex.renderToString(r.latex ?? '', { throwOnError: false });
  }

  async function handleMocf(rawInput: string) {
    if (!rawInput.trim()) return;
    currentMocfInput = rawInput.trim();
    const r = await mocfAnalyze(rawInput);
    if (r.error) {
      mocfHtml.value = '(error)';
      ordinalHtml.value = '';
      return;
    }
    mocfHtml.value = katex.renderToString(r.latex ?? '', { throwOnError: false });
    const b = await mocfToBocf(rawInput);
    if (b.error) {
      ordinalHtml.value = '';
      veblenHtml.value = '';
      lmnHtml.value = '';
      return;
    }
    const v = await termToVeblen(b.ordinalJS ?? []);
    lastResult = {
      gteEBO: false,
      ordinal: b.latex ?? '',
      ordinalJS: b.ordinalJS ?? [],
      psiSimple: b.psiSimple ?? '',
      ...v,
      nsForm: '',
      isStandard: true,
    } as AnalysisResult;
    const bocfOut = getBocfOutput(lastResult, bocfDisplayMode.value);
    ordinalHtml.value = bocfOut ? katex.renderToString(bocfOut, { throwOnError: false }) : '';
    if (lastResult.veblen) {
      const veblenOut = getVeblenOutput(lastResult, veblenMode.value, sugarEnabled.value);
      veblenHtml.value = veblenOut ? katex.renderToString(veblenOut, { throwOnError: false }) : '';
    }
    refreshLmnRow();
  }

  // BOCF conversion
  async function convertBocfToBms() {
    converting.value = true;
    convertStatus.value = t('status.searching');
    bmsHtml.value = '';
    const startTime = performance.now();
    try {
      const bms = await bocfToBMS(inputValue.value, (cur: string) => {
        const elapsed = ((performance.now() - startTime) / 1000).toFixed(1);
        convertStatus.value = t('status.iter') + ' ' + cur + ' (' + elapsed + 's)';
      });
      converting.value = false;
      bmsRaw.value = bms;
      bmsHtml.value = renderBms(bms);
      if (bms !== '(empty)') {
        const matrix = parseMatrix(bms);
        const seq = await bmsTo0YSequence(matrix);
        zeroYHtml.value = seq ? katex.renderToString(seq, { throwOnError: false }) : '';
        const triMatrix = await bmsToTriangular(matrix);
        if (triMatrix && triMatrix.length > 0) {
          showTriangularRow.value = true;
          const raw = matrixToDisplayStr(triMatrix);
          const aligned = alignMatrixStr(raw);
          triangularHtml.value = katex.renderToString('\\text{' + aligned + '}', { throwOnError: false });
        }
      }
      const elapsed = ((performance.now() - startTime) / 1000).toFixed(3);
      convertStatus.value = t('status.done') + ' (' + elapsed + 's)';
    } catch (e) {
      converting.value = false;
      convertStatus.value = String(e);
    }
  }

  function cancelConvert() {
    cancelBocfToBMS();
    converting.value = false;
    convertStatus.value = t('status.cancelled');
    bmsHtml.value = '';
  }

  // Expand
  async function doExpand() {
    try {
      const raw = parseInt(expandFs.value);
      const fs = isNaN(raw) ? 3 : raw;

      if (inputMode.value === '1y') {
        if (!current1YSeq) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await expand1Y(current1YSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'wy') {
        if (!currentWYSeq) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await expandWY(currentWYSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'bocf') {
        if (!currentBocfOrdinal) { expandResult.value = t('status.expand.noOrdinal'); return; }
        const r = await fundamentalSequence(currentBocfOrdinal, fs);
        expandResult.value = r.term ? katex.renderToString(r.term, { throwOnError: false }) : '0';
      } else if (inputMode.value === '0y') {
        if (!current0YSeq) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await zeroYExpand(current0YSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'upms') {
        if (!currentUPMSMatrix) { expandResult.value = t('status.expand.noMatrix'); return; }
        const expanded = await expandUPMS(currentUPMSMatrix, fs);
        expandResult.value = renderUpms(matrixToDisplayStr(expanded));
      } else if (inputMode.value === 'hprss') {
        if (!currentHPRSSSeq) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await expandHPRSS(currentHPRSSSeq, fs);
        if (expanded && expanded.length > 0) {
          expandResult.value = expanded.join(',');
        }
      } else if (inputMode.value === 'lprss') {
        if (!currentLPRSSSeq.value || currentLPRSSSeq.value.length === 0) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await expandLPRSS(currentLPRSSSeq.value, fs);
        if (expanded && expanded.length > 0) {
          expandResult.value = expanded.join(',');
        }
      } else if (inputMode.value === 'ihss') {
        if (!currentMBOcfInput) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await ihssExpand(currentMBOcfInput, fs);
        expandResult.value = expanded
          ? renderMbo(expanded)
          : katex.renderToString('0', { throwOnError: false });
      } else if (inputMode.value === 'mbo') {
        if (!currentMboInput) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await ihssExpand(currentMboInput, fs);
        if (!expanded) {
          expandResult.value = katex.renderToString('0', { throwOnError: false });
        } else {
          const ih = await ihssAnalyze(expanded);
          expandResult.value = ih.latex
            ? katex.renderToString(applyMboSugar(ih.latex), { throwOnError: false })
            : katex.renderToString('\\text{' + expanded + '}', { throwOnError: false });
        }
      } else if (inputMode.value === 'sss') {
        if (!currentSSSSeq || currentSSSSeq.length === 0) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await sssExpand(currentSSSSeq, fs);
        expandResult.value = expanded.length > 0 ? expanded.join(',') : '0';
      } else if (inputMode.value === 'nocf') {
        if (!currentNocfInput) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await nocfExpand(currentNocfInput, fs);
        expandResult.value = katex.renderToString(expanded, { throwOnError: false });
      } else if (inputMode.value === 'mocf') {
        if (!currentMocfInput) { expandResult.value = t('status.expand.noSequence'); return; }
        const expanded = await mocfExpand(currentMocfInput, fs);
        expandResult.value = katex.renderToString(expanded, { throwOnError: false });
      } else {
        const rawInput = transformInput(inputValue.value, inputMode.value);
        const matrix = parseMatrix(rawInput);
        const expanded = await expandBMS(matrix, fs);
        expandResult.value = renderBms(matrixToDisplayStr(expanded));
      }
    } catch {
      expandResult.value = '(error)';
    }
  }

  // Watch input changes
  watch([inputMode, inputValue, veblenMode, sugarEnabled, bmsInputPref], () => {
    update();
  }, { immediate: true });

  // Re-render BMS and UPMS when display mode changes
  watch(bmsDisplayMode, () => {
    if (bmsRaw.value) bmsHtml.value = renderBms(bmsRaw.value);
  });
  watch(bmsCompactStyle, () => {
    if (bmsRaw.value) bmsHtml.value = renderBms(bmsRaw.value);
  });
  watch(upmsDisplayMode, () => {
    if (upmsRaw.value) upmsHtml.value = renderUpms(upmsRaw.value);
  });
  watch(upmsCompactStyle, () => {
    if (upmsRaw.value) upmsHtml.value = renderUpms(upmsRaw.value);
  });

  // Re-render the Mahlo BOCF / IHSS Hydra output when its display settings change
  watch([mboDisplayMode, mboCompactStyle, mboSugar], () => {
    if (currentMBOcfLatex) mboHtml.value = katex.renderToString(applyMboSugar(currentMBOcfLatex), { throwOnError: false });
    if (currentMBOcfInput) {
      ihssAnalyze(currentMBOcfInput).then((ih) => {
        if (ih.matrix && ih.matrix.length > 0) {
          mboMatrix.value = renderMbo(matrixToDisplayStr(ih.matrix));
        }
      });
    }
  });

  watch([veblenMode, sugarEnabled], () => {
    if (lastResult?.veblen && !lastResult.gteEBO) {
      const v = getVeblenOutput(lastResult, veblenMode.value, sugarEnabled.value);
      veblenHtml.value = v ? katex.renderToString(v, { throwOnError: false }) : '';
    }
  });

  watch(bocfDisplayMode, () => {
    if (lastResult) {
      const bocfOut = getBocfOutput(lastResult, bocfDisplayMode.value);
      ordinalHtml.value = bocfOut ? katex.renderToString(bocfOut, { throwOnError: false }) : '';
    }
  });

  watch(lmnDisplayMode, () => {
    refreshLmnRow();
  });

  function forceNonStandardConvert() {
    if (!pendingMatrix.value) return;
    forceNonStandard.value = true;
    nonStandard.value = false;
    update(true);
  }

  return {
    ordinalHtml, veblenHtml, lmnHtml, zeroYHtml, dbmsHtml, bmsHtml, triangularHtml, upmsHtml,
    hydraHtml, hprssHtml, lprssHtml, mboHtml, mboMatrix, mboAstHtml, sssNocfHtml, sssTprssHtml, nocfHtml, mocfHtml, bocfMocfHtml,
    showDbmsRow, showBmsRow, showTriangularRow, showUpmsRow, showMountainRow,
    showHydraRow, showHprssRow, showLprssRow, showMboRow, showMboAstRow, showSssNocfRow, showSssTprssRow,
    bmsRaw, mountainType, mountainData, mountainRowLabels,
    nonStandard, forceNonStandardConvert,
    bocfNonStandardWarning, hydraNonStandardWarning, ihssNonStandardWarning, sssNonStandardWarning,
    converting, convertStatus, convertBocfToBms, cancelConvert,
    expandResult, expandFs, doExpand,
  };
}
