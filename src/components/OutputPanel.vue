<script lang="ts" setup vapor>
import { ref } from 'vue';
import { useI18n } from '../composables/useI18n';
import type { InputMode } from '../composables/useAnalysis';
const { t } = useI18n();

const props = defineProps<{
  inputMode: InputMode;
  ordinalHtml: string;
  veblenHtml: string;
  zeroYHtml: string;
  dbmsHtml: string;
  bmsHtml: string;
  triangularHtml: string;
  upmsHtml: string;
  hydraHtml: string;
  hprssHtml: string;
  lprssHtml: string;
  mboHtml: string;
  mboMatrix: string;
  mboAstHtml: string;
  sssNocfHtml: string;
  sssTprssHtml: string;
  showDbmsRow: boolean;
  showBmsRow: boolean;
  showTriangularRow: boolean;
  showUpmsRow: boolean;
  showHydraRow: boolean;
  showHprssRow: boolean;
  showLprssRow: boolean;
  showMboRow: boolean;
  showMboAstRow: boolean;
  showSssNocfRow: boolean;
  showSssTprssRow: boolean;
  converting: boolean;
  convertStatus: string;
}>();

const emit = defineEmits<{
  convert: [];
  cancel: [];
}>();

const copiedRow = ref('');

// Remove a single top-level \text{...} wrapper (used by KaTeX for plain-text rows).
function unwrapText(latex: string): string {
  const prefix = '\\text{';
  if (!latex.startsWith(prefix)) return latex;
  let depth = 0;
  for (let i = prefix.length; i < latex.length; i++) {
    if (latex[i] === '{') depth++;
    else if (latex[i] === '}') {
      if (depth === 0) {
        return i === latex.length - 1 ? latex.slice(prefix.length, i) : latex;
      }
      depth--;
    }
  }
  return latex;
}

function copyText(e: MouseEvent, label: string) {
  const target = e.currentTarget as HTMLElement;
  const row = target.closest('[data-row]') as HTMLElement;
  if (!row) return;
  const content = row.querySelector('[data-content]');
  if (!content) return;
  // KaTeX embeds the original LaTeX source in a MathML annotation; use that so
  // we copy the actual expression, not the garbled rendered glyphs.
  const annotation = content.querySelector('annotation[encoding="application/x-tex"]');
  const raw = annotation ? (annotation.textContent || '') : (content.textContent || '');
  const text = unwrapText(raw);
  navigator.clipboard.writeText(text).catch(() => {});
  copiedRow.value = label;
  setTimeout(() => { copiedRow.value = ''; }, 1500);
}
</script>

<template>
  <div style="display: inline-flex; flex-direction: column; align-items: flex-start; margin: 0 auto">
    <!-- BOCF / Ordinal output -->
    <div data-row="ordinal" style="display: flex; align-items: baseline; gap: 8px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'ordinal')" :title="t('output.copy')">BOCF</span>
      <span data-content style="font-size: 16pt" v-html="props.ordinalHtml"></span>
      <span v-if="copiedRow === 'ordinal'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- Veblen output -->
    <div data-row="veblen" style="display: flex; align-items: baseline; gap: 8px; margin-top: 8px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'veblen')" :title="t('output.copy')">Veblen</span>
      <span data-content style="font-size: 14pt" v-html="props.veblenHtml"></span>
      <span v-if="copiedRow === 'veblen'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- 0-Y output -->
    <div data-row="0y" style="display: flex; align-items: baseline; gap: 8px; margin-top: 8px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, '0y')" :title="t('output.copy')">0-Y</span>
      <span data-content style="font-size: 14pt" v-html="props.zeroYHtml"></span>
      <span v-if="copiedRow === '0y'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- UPMS output -->
    <div v-if="props.showUpmsRow" data-row="upms" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'upms')" :title="t('output.copy')">UPMS</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.upmsHtml"></span>
      <span v-if="copiedRow === 'upms'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- DBMS output -->
    <div v-if="props.showDbmsRow" data-row="dbms" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'dbms')" :title="t('output.copy')">DBMS</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.dbmsHtml"></span>
      <span v-if="copiedRow === 'dbms'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- Triangular BMS output -->
    <div v-if="props.showTriangularRow" data-row="tri" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'tri')" :title="t('output.copy')">Tri BMS</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.triangularHtml"></span>
      <span v-if="copiedRow === 'tri'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- PSS Hydra output -->
    <div v-if="props.showHydraRow" data-row="hydra" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 9pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'hydra')" :title="t('output.copy')">PSS Hydra</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.hydraHtml"></span>
      <span v-if="copiedRow === 'hydra'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- HPrSS output -->
    <div v-if="props.showHprssRow" data-row="hprss" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'hprss')" :title="t('output.copy')">HPrSS</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.hprssHtml"></span>
      <span v-if="copiedRow === 'hprss'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- LPrSS output -->
    <div v-if="props.showLprssRow" data-row="lprss" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'lprss')" :title="t('output.copy')">LPrSS</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.lprssHtml"></span>
      <span v-if="copiedRow === 'lprss'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- Mahlo BOCF output (experimental) -->
    <div v-if="props.showMboRow" data-row="mbocf" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'mbocf')" :title="t('output.copy')">Mahlo BOCF</span>
      <span data-content style="font-size: 14pt" v-html="props.mboHtml"></span>
      <span v-if="copiedRow === 'mbocf'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- IHSS Hydra output -->
    <div v-if="props.showMboRow && props.mboMatrix" data-row="ihss" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'ihss')" :title="t('output.copy')">IHSS Hydra</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.mboMatrix"></span>
      <span v-if="copiedRow === 'ihss'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- Mahlo BOCF AST (experimental) -->
    <div v-if="props.showMboAstRow" data-row="mboast" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'mboast')" :title="t('output.copy')">Mahlo BOCF AST</span>
      <pre data-content class="mono" style="font-size: 11pt; font-family: monospace; margin: 0" v-text="props.mboAstHtml"></pre>
      <span v-if="copiedRow === 'mboast'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <div v-if="props.showSssNocfRow" data-row="nocf" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'nocf')" :title="t('output.copy')">NOCF</span>
      <span data-content style="font-size: 14pt" v-html="props.sssNocfHtml"></span>
      <span v-if="copiedRow === 'nocf'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <div v-if="props.showSssTprssRow" data-row="tprss" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'tprss')" :title="t('output.copy')">TPrSS</span>
      <span data-content style="font-size: 14pt" v-html="props.sssTprssHtml"></span>
      <span v-if="copiedRow === 'tprss'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>

    <!-- BMS output row -->
    <div v-if="props.showBmsRow" data-row="bms" style="display: flex; align-items: center; gap: 8px; margin-top: 6px; flex-wrap: wrap">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyText($event, 'bms')" :title="t('output.copy')">BMS</span>
      <button v-if="props.inputMode === 'bocf'" class="mode-btn" style="font-size: 10pt" @click="emit('convert')">{{ t('output.convert') }}</button>
      <button v-if="props.inputMode === 'bocf' && props.converting" class="mode-btn" style="font-size: 10pt; color: var(--error)" @click="emit('cancel')">{{ t('output.cancel') }}</button>
      <span v-if="props.inputMode === 'bocf'" class="muted" style="font-size: 10pt; max-width: 400px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ props.convertStatus }}</span>
      <span data-content class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.bmsHtml"></span>
      <span v-if="copiedRow === 'bms'" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>
  </div>
</template>
