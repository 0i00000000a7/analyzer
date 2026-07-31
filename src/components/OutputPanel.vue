<script lang="ts" setup vapor>
import type { InputMode } from '../composables/useAnalysis';

const props = defineProps<{
  inputMode: InputMode;
  ordinalHtml: string;
  veblenHtml: string;
  zeroYHtml: string;
  dbmsHtml: string;
  bmsHtml: string;
  triangularHtml: string;
  upmsHtml: string;
  showDbmsRow: boolean;
  showBmsRow: boolean;
  showTriangularRow: boolean;
  showUpmsRow: boolean;
  converting: boolean;
  convertStatus: string;
}>();

const emit = defineEmits<{
  convert: [];
  cancel: [];
}>();
</script>

<template>
  <div style="display: inline-flex; flex-direction: column; align-items: flex-start; margin: 0 auto">
    <!-- BOCF / Ordinal output -->
    <div style="display: flex; align-items: baseline; gap: 8px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">BOCF</span>
      <span style="font-size: 16pt" v-html="props.ordinalHtml"></span>
    </div>

    <!-- Veblen output -->
    <div style="display: flex; align-items: baseline; gap: 8px; margin-top: 8px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">Veblen</span>
      <span style="font-size: 14pt" v-html="props.veblenHtml"></span>
    </div>

    <!-- 0-Y output -->
    <div style="display: flex; align-items: baseline; gap: 8px; margin-top: 8px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">0-Y</span>
      <span style="font-size: 14pt" v-html="props.zeroYHtml"></span>
    </div>

    <!-- UPMS output -->
    <div v-if="props.showUpmsRow" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">UPMS</span>
      <span class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.upmsHtml"></span>
    </div>

    <!-- DBMS output -->
    <div v-if="props.showDbmsRow" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">DBMS</span>
      <span class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.dbmsHtml"></span>
    </div>

    <!-- Triangular BMS output -->
    <div v-if="props.showTriangularRow" style="display: flex; align-items: baseline; gap: 8px; margin-top: 6px">
      <span class="label" style="font-size: 12pt; width: 68px; text-align: right">Tri BMS</span>
      <span class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.triangularHtml"></span>
    </div>

    <!-- BMS output row -->
    <div v-if="props.showBmsRow" style="display: flex; align-items: center; gap: 8px; margin-top: 6px; flex-wrap: wrap">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">BMS</span>
      <button v-if="props.inputMode === 'bocf'" class="mode-btn" style="font-size: 10pt" @click="emit('convert')">Convert</button>
      <button v-if="props.inputMode === 'bocf' && props.converting" class="mode-btn" style="font-size: 10pt; color: var(--error)" @click="emit('cancel')">Cancel</button>
      <span v-if="props.inputMode === 'bocf'" class="muted" style="font-size: 10pt; max-width: 400px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ props.convertStatus }}</span>
      <span class="mono" style="font-size: 11pt; font-family: monospace; word-break: break-all" v-html="props.bmsHtml"></span>
    </div>
  </div>
</template>
