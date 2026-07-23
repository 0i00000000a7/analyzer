/**
 * Structural comparison test: verify C++/WASM matches JS baseline.
 *
 * Usage: node src/ts/run-tests.js
 */

import { readFileSync, appendFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = join(__dirname, '../..');
const LOG_FILE = join(PROJECT_ROOT, 'test-results.log');

function serializeStructure(arr) {
  if (arr == null || (Array.isArray(arr) && arr.length === 0)) return '0';
  if (!Array.isArray(arr)) return String(arr);
  return '[' + arr.map(serializeStructure).join(',') + ']';
}

const mod = await import('../wasm/bms-core.js');
const wasm = await (mod.default || mod.BmsModule)();
const testCases = JSON.parse(readFileSync(join(PROJECT_ROOT, 'test-cases.json'), 'utf-8'));

let passed = 0,
  failed = 0;
let mismatches = [];
const total = testCases.length;

// Initialize log file
writeFileSync(LOG_FILE, 'BMS Test Results — ψ form | Veblen form\n\n');

process.stdout.write(`  0/${total}\n`);
for (let i = 0; i < total; i++) {
  const tc = testCases[i];

  // Milestone at named entries
  if (tc.name) {
    const pct = Math.floor((i / total) * 100);
    process.stdout.write(`  ${i}/${total} (${pct}%) — ${tc.name}\n`);
  }

  try {
    const r = wasm.bmsAnalyze(tc.matrix);
    appendFileSync(LOG_FILE, `${tc.name || tc.input}\n  ψ:  ${r.ordinal}\n  V:  ${r.veblen || '≥BHO'}\n\n`);
    // Verify Veblen output
    if (tc.formattedVeblen != null) {
      if (String(r.veblen || '') !== tc.formattedVeblen) {
        mismatches.push({
          i,
          name: tc.name || tc.input,
          field: 'Veblen',
          expected: tc.formattedVeblen,
          got: String(r.veblen || ''),
        });
      }
    }

    if (tc.isEBO) {
      if (r.ordinal !== tc.formattedOrdinal) {
        mismatches.push({ i, name: tc.name || tc.input, expected: tc.formattedOrdinal, got: r.ordinal });
      }
    } else {
      const s = serializeStructure(r.ordinalJS);
      if (s !== tc.serializedOCF) {
        mismatches.push({ i, name: tc.name || tc.input, expected: tc.serializedOCF, got: s });
      }
    }
  } catch (e) {
    mismatches.push({ i, name: tc.name || tc.input, expected: '(ok)', got: e.message });
  }
}
process.stdout.write(`  ${total}/${total} (100%)\n`);

for (const m of mismatches.slice(0, 10)) {
  const field = m.field ? ` [${m.field}]` : '';
  process.stdout.write(`  ✕ #${m.i} ${m.name}${field}\n    expected: ${m.expected}\n    got:      ${m.got}\n`);
}

if (mismatches.length > 10) {
  process.stdout.write(`  ... and ${mismatches.length - 10} more\n`);
}

if (mismatches.length === 0) {
  passed++;
  process.stdout.write(`  ✓ ${testCases.length} cases, 0 mismatches\n`);
} else {
  failed++;
}
process.stdout.write(`\n${passed} passed, ${failed} failed\n`);
process.exit(failed > 0 ? 1 : 0);
