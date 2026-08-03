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
  upmsToBMS,
  bocfToBMS,
  cancelBocfToBMS,
  termToVeblen,
  fundamentalSequence,
} from '../ts/bms.js';
import { parse0Y, zeroYToBMS, zeroYExpand, buildMountain } from '../ts/bms-zero-y.js';
import { triangularToBMS, bmsToTriangular } from '../ts/bms-triangular.js';
import { expand1Y, expandWY, buildWYMountain, build1YMountain } from '../ts/wy.js';
import { oneYToDBMS, dbmsToString, dbmsToBMS } from '../ts/y_dbms.js';
import { hydraAnalyze, hprssAnalyze, lprssAnalyze, expandHPRSS, expandHydra, buildHPRSSMountain, bmsToHydraAnalysis, expandLPRSS, buildLPRSSMountain } from '../ts/hydra.js';
import type { AnalysisResult, Matrix, Mountain } from '../ts/types.js';

export type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf' | 'upms' | 'hprss' | 'lprss' | 'hydra';
export type VeblenMode = 'v' | 'm';
export type BmsDisplayMode = 'matrix' | 'flat' | 'compact';
export type UpmsDisplayMode = 'matrix' | 'flat' | 'compact';
export type BmsCompactStyle = 'brace' | 'alpha';
export type BmsInputPreference = 'auto' | 'normal' | 'triangular';

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
      } else {
        result.push(parseInt(token[i], 10));
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
    if (mode === 'bms' || mode === 'upms') return tokens.map((t) => '(' + parseCompactToken(t).join(',') + ')').join('');
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
  bmsDisplayMode: Ref<BmsDisplayMode>,
  upmsDisplayMode: Ref<UpmsDisplayMode>,
  bmsCompactStyle: Ref<BmsCompactStyle>,
  upmsCompactStyle: Ref<BmsCompactStyle>,
  bmsInputPref: Ref<BmsInputPreference>,
) {
  const ordinalHtml = ref('');
  const veblenHtml = ref('');
  const zeroYHtml = ref('');
  const dbmsHtml = ref('');
  const bmsHtml = ref('');
  const triangularHtml = ref('');
  const upmsHtml = ref('');
  const hydraHtml = ref('');
  const hprssHtml = ref('');
  const lprssHtml = ref('');

  const showDbmsRow = ref(false);
  const showBmsRow = ref(false);
  const showTriangularRow = ref(false);
  const showUpmsRow = ref(false);
  const showMountainRow = ref(false);
  const showHydraRow = ref(false);
  const showHprssRow = ref(false);
  const showLprssRow = ref(false);

  const bmsRaw = ref('');
  const upmsRaw = ref('');
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

  function clearOutputs() {
    ordinalHtml.value = '';
    veblenHtml.value = '';
    zeroYHtml.value = '';
    dbmsHtml.value = '';
    bmsHtml.value = '';
    triangularHtml.value = '';
    upmsHtml.value = '';
    hydraHtml.value = '';
    hprssHtml.value = '';
    lprssHtml.value = '';
    showDbmsRow.value = false;
    showBmsRow.value = false;
    showTriangularRow.value = false;
    showUpmsRow.value = false;
    showMountainRow.value = false;
    showHydraRow.value = false;
    showHprssRow.value = false;
    showLprssRow.value = false;
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
  }

  async function update() {
    try {
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
        // Triangular BMS input → convert to normal BMS
        if (isTri) {
          matrix = await triangularToBMS(matrix);
        }
      }

      const r = await analyze(matrix);
      lastResult = r;
      ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });

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
    ordinalHtml.value = r.ordinal ? katex.renderToString(r.ordinal, { throwOnError: false }) : '';
    if (r.ordinalJS) {
      currentBocfOrdinal = r.ordinalJS;
      const v = await termToVeblen(r.ordinalJS);
      lastResult = { gteEBO: false, ordinal: r.ordinal, ordinalJS: r.ordinalJS, ...v, nsForm: '', isStandard: true } as AnalysisResult;
      if (lastResult.veblen) {
        const veblenOut = getVeblenOutput(lastResult, veblenMode.value, sugarEnabled.value);
        if (veblenOut) veblenHtml.value = katex.renderToString(veblenOut, { throwOnError: false });
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
      ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });
      bmsRaw.value = '';
      bmsHtml.value = '';
      showBmsRow.value = true;
      return;
    }
    const seq = parse0Y(rawInput);
    if (seq.length === 0 || seq.some(isNaN)) return;

    current0YSeq = seq;
    const matrix = await zeroYToBMS(seq);
    const flat = matrixToDisplayStr(matrix);
    bmsRaw.value = flat;
    bmsHtml.value = renderBms(flat);
    showBmsRow.value = true;

    const r = await analyze(matrix);
    lastResult = r;
    ordinalHtml.value = katex.renderToString(r.ordinal, { throwOnError: false });

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
          const seq0y = await bmsTo0YSequence(bms);
          zeroYHtml.value = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
        } else {
          ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
          veblenHtml.value = katex.renderToString('0', { throwOnError: false });
        }
      } else if (hasOmega) {
        // no BMS for omega
      } else {
        ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
        veblenHtml.value = katex.renderToString('0', { throwOnError: false });
      }
    } catch {}
  }

  async function handleWY(rawInput: string) {
    let seq = parse0Y(rawInput);
    if (seq.some(isNaN) || rawInput === '') seq = [0];
    currentWYSeq = seq;

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
          const seq0y = await bmsTo0YSequence(bms);
          zeroYHtml.value = seq0y ? katex.renderToString(seq0y, { throwOnError: false }) : '';
        } else {
          ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
          veblenHtml.value = katex.renderToString('0', { throwOnError: false });
        }
      } else if (hasOmega) {
        // no BMS for omega
      } else {
        ordinalHtml.value = katex.renderToString('0', { throwOnError: false });
        veblenHtml.value = katex.renderToString('0', { throwOnError: false });
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

  // BOCF conversion
  async function convertBocfToBms() {
    converting.value = true;
    convertStatus.value = 'searching...';
    bmsHtml.value = '';
    const startTime = performance.now();
    try {
      const bms = await bocfToBMS(inputValue.value, (cur: string) => {
        const elapsed = ((performance.now() - startTime) / 1000).toFixed(1);
        convertStatus.value = 'iter ' + cur + ' (' + elapsed + 's)';
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
      convertStatus.value = 'Done (' + elapsed + 's)';
    } catch (e) {
      converting.value = false;
      convertStatus.value = String(e);
    }
  }

  function cancelConvert() {
    cancelBocfToBMS();
    converting.value = false;
    convertStatus.value = 'Cancelled';
    bmsHtml.value = '';
  }

  // Expand
  async function doExpand() {
    try {
      const raw = parseInt(expandFs.value);
      const fs = isNaN(raw) ? 3 : raw;

      if (inputMode.value === '1y') {
        if (!current1YSeq) { expandResult.value = '(no sequence)'; return; }
        const expanded = await expand1Y(current1YSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'wy') {
        if (!currentWYSeq) { expandResult.value = '(no sequence)'; return; }
        const expanded = await expandWY(currentWYSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'bocf') {
        if (!currentBocfOrdinal) { expandResult.value = '(no ordinal)'; return; }
        const r = await fundamentalSequence(currentBocfOrdinal, fs);
        expandResult.value = r.term ? katex.renderToString(r.term, { throwOnError: false }) : '0';
      } else if (inputMode.value === '0y') {
        if (!current0YSeq) { expandResult.value = '(no sequence)'; return; }
        const expanded = await zeroYExpand(current0YSeq, fs);
        expandResult.value = expanded.join(',');
      } else if (inputMode.value === 'upms') {
        if (!currentUPMSMatrix) { expandResult.value = '(no matrix)'; return; }
        const expanded = await expandUPMS(currentUPMSMatrix, fs);
        expandResult.value = renderUpms(matrixToDisplayStr(expanded));
      } else if (inputMode.value === 'hprss') {
        if (!currentHPRSSSeq) { expandResult.value = '(no sequence)'; return; }
        const expanded = await expandHPRSS(currentHPRSSSeq, fs);
        if (expanded && expanded.length > 0) {
          expandResult.value = expanded.join(',');
        }
      } else if (inputMode.value === 'lprss') {
        if (!currentLPRSSSeq.value || currentLPRSSSeq.value.length === 0) { expandResult.value = '(no sequence)'; return; }
        const expanded = await expandLPRSS(currentLPRSSSeq.value, fs);
        if (expanded && expanded.length > 0) {
          expandResult.value = expanded.join(',');
        }
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

  watch([veblenMode, sugarEnabled], () => {
    if (lastResult?.veblen && !lastResult.gteEBO) {
      const v = getVeblenOutput(lastResult, veblenMode.value, sugarEnabled.value);
      veblenHtml.value = v ? katex.renderToString(v, { throwOnError: false }) : '';
    }
  });

  return {
    ordinalHtml, veblenHtml, zeroYHtml, dbmsHtml, bmsHtml, triangularHtml, upmsHtml,
    hydraHtml, hprssHtml, lprssHtml,
    showDbmsRow, showBmsRow, showTriangularRow, showUpmsRow, showMountainRow,
    showHydraRow, showHprssRow, showLprssRow,
    bmsRaw, mountainType, mountainData, mountainRowLabels,
    converting, convertStatus, convertBocfToBms, cancelConvert,
    expandResult, expandFs, doExpand,
  };
}
