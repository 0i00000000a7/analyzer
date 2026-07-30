<script lang="ts" setup vapor>
import { ref, onMounted, onUnmounted } from 'vue';
import katex from 'katex';
import type { VeblenMode, BmsDisplayMode, BmsInputPreference } from '../composables/useAnalysis';

const veblenMode = defineModel<VeblenMode>('veblenMode', { required: true });
const sugarEnabled = defineModel<boolean>('sugarEnabled', { required: true });
const bmsDisplayMode = defineModel<BmsDisplayMode>('bmsDisplayMode', { required: true });
const bmsInputPref = defineModel<BmsInputPreference>('bmsInputPref', { required: true });

const open = ref(false);
const containerRef = ref<HTMLDivElement | null>(null);

function onClickOutside(e: MouseEvent) {
  if (open.value && containerRef.value && !containerRef.value.contains(e.target as Node)) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener('click', onClickOutside));
onUnmounted(() => document.removeEventListener('click', onClickOutside));

const veblenVHtml = katex.renderToString('\\alpha @\\beta', { throwOnError: false });
const veblenMHtml = katex.renderToString('\\begin{smallmatrix}\\alpha\\\\\\beta\\end{smallmatrix}', { throwOnError: false });
</script>

<template>
  <div ref="containerRef" style="position: relative; display: inline-block">
    <button class="mode-btn" style="font-size: 11pt" @click="open = !open">Settings</button>
    <div v-if="open" class="settings-panel">
      <div class="settings-section">Appearance</div>
      <div class="settings-row">
        <span class="label">Veblen</span>
        <button class="mode-btn" :class="{ active: veblenMode === 'v' }" @click="veblenMode = 'v'" v-html="veblenVHtml"></button>
        <button class="mode-btn" :class="{ active: veblenMode === 'm' }" @click="veblenMode = 'm'" v-html="veblenMHtml"></button>
        <label class="settings-label">
          <input type="checkbox" v-model="sugarEnabled" /> Sugar
        </label>
      </div>
      <div class="settings-row">
        <span class="label">BMS</span>
        <button class="mode-btn" :class="{ active: bmsDisplayMode === 'matrix' }" @click="bmsDisplayMode = 'matrix'">Matrix</button>
        <button class="mode-btn" :class="{ active: bmsDisplayMode === 'flat' }" @click="bmsDisplayMode = 'flat'">Flat</button>
      </div>
      <div class="settings-section">Input Preference</div>
      <div class="settings-row">
        <span class="label">BMS</span>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'auto' }" @click="bmsInputPref = 'auto'">Auto</button>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'normal' }" @click="bmsInputPref = 'normal'">Normal</button>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'triangular' }" @click="bmsInputPref = 'triangular'">Triangular</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 4px;
  padding: 10px 14px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 8px;
  white-space: nowrap;
}
.settings-section {
  font-size: 10pt;
  font-weight: bold;
  color: var(--label);
  border-bottom: 1px solid var(--border);
  padding-bottom: 2px;
  margin-top: 2px;
}
.settings-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.settings-row .label {
  width: 50px;
  text-align: right;
  font-size: 11pt;
}
.settings-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11pt;
  cursor: pointer;
}
</style>
