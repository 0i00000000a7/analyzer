<script lang="ts" setup vapor>
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from '../composables/useI18n';
import katex from 'katex';
import type { VeblenMode, BocfDisplayMode, BmsDisplayMode, UpmsDisplayMode, BmsCompactStyle, BmsInputPreference, MboDisplayMode, MboCompactStyle } from '../composables/useAnalysis';

const { t, locale, setLocale } = useI18n();

const veblenMode = defineModel<VeblenMode>('veblenMode', { required: true });
const sugarEnabled = defineModel<boolean>('sugarEnabled', { required: true });
const bocfDisplayMode = defineModel<BocfDisplayMode>('bocfDisplayMode', { required: true });
const bmsDisplayMode = defineModel<BmsDisplayMode>('bmsDisplayMode', { required: true });
const upmsDisplayMode = defineModel<UpmsDisplayMode>('upmsDisplayMode', { required: true });
const bmsCompactStyle = defineModel<BmsCompactStyle>('bmsCompactStyle', { required: true });
const upmsCompactStyle = defineModel<BmsCompactStyle>('upmsCompactStyle', { required: true });
const bmsInputPref = defineModel<BmsInputPreference>('bmsInputPref', { required: true });
const enableMBOcf = defineModel<boolean>('enableMBOcf', { required: true });
const mboDisplayMode = defineModel<MboDisplayMode>('mboDisplayMode', { required: true });
const mboCompactStyle = defineModel<MboCompactStyle>('mboCompactStyle', { required: true });
const mboSugar = defineModel<boolean>('mboSugar', { required: true });

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
    <button class="mode-btn" style="font-size: 11pt" @click="open = !open">{{ t('settings.toggle') }}</button>
    <div v-if="open" class="settings-panel">
      <div class="settings-section">{{ t('settings.section.appearance') }}</div>
      <div class="settings-row">
        <span class="label">Veblen</span>
        <button class="mode-btn" :class="{ active: veblenMode === 'v' }" @click="veblenMode = 'v'" v-html="veblenVHtml"></button>
        <button class="mode-btn" :class="{ active: veblenMode === 'm' }" @click="veblenMode = 'm'" v-html="veblenMHtml"></button>
        <label class="settings-label">
          <input type="checkbox" v-model="sugarEnabled" /> {{ t('settings.sugar') }}
        </label>
      </div>
      <div class="settings-row">
        <span class="label">BOCF</span>
        <button class="mode-btn" :class="{ active: bocfDisplayMode === 'normal' }" @click="bocfDisplayMode = 'normal'">{{ t('settings.bocf.normal') }}</button>
        <button class="mode-btn" :class="{ active: bocfDisplayMode === 'psi' }" @click="bocfDisplayMode = 'psi'">{{ t('settings.bocf.psiRaw') }}</button>
      </div>
      <div class="settings-row">
        <span class="label">BMS</span>
        <button class="mode-btn" :class="{ active: bmsDisplayMode === 'matrix' }" @click="bmsDisplayMode = 'matrix'">{{ t('settings.display.matrix') }}</button>
        <button class="mode-btn" :class="{ active: bmsDisplayMode === 'flat' }" @click="bmsDisplayMode = 'flat'">{{ t('settings.display.flat') }}</button>
        <button class="mode-btn" :class="{ active: bmsDisplayMode === 'compact' }" @click="bmsDisplayMode = 'compact'; bmsCompactStyle = 'alpha'">{{ t('settings.display.compact') }}</button>
        <template v-if="bmsDisplayMode === 'compact'">
          <button class="mode-btn sub" :class="{ active: bmsCompactStyle === 'brace' }" @click="bmsCompactStyle = 'brace'">{{ t('settings.compact.brace') }}</button>
          <button class="mode-btn sub" :class="{ active: bmsCompactStyle === 'alpha' }" @click="bmsCompactStyle = 'alpha'">{{ t('settings.compact.alpha') }}</button>
        </template>
      </div>
      <div class="settings-row">
        <span class="label">UPMS</span>
        <button class="mode-btn" :class="{ active: upmsDisplayMode === 'matrix' }" @click="upmsDisplayMode = 'matrix'">{{ t('settings.display.matrix') }}</button>
        <button class="mode-btn" :class="{ active: upmsDisplayMode === 'flat' }" @click="upmsDisplayMode = 'flat'">{{ t('settings.display.flat') }}</button>
        <button class="mode-btn" :class="{ active: upmsDisplayMode === 'compact' }" @click="upmsDisplayMode = 'compact'; upmsCompactStyle = 'alpha'">{{ t('settings.display.compact') }}</button>
        <template v-if="upmsDisplayMode === 'compact'">
          <button class="mode-btn sub" :class="{ active: upmsCompactStyle === 'brace' }" @click="upmsCompactStyle = 'brace'">{{ t('settings.compact.brace') }}</button>
          <button class="mode-btn sub" :class="{ active: upmsCompactStyle === 'alpha' }" @click="upmsCompactStyle = 'alpha'">{{ t('settings.compact.alpha') }}</button>
        </template>
      </div>
      <div class="settings-section">{{ t('settings.section.inputPreference') }}</div>
      <div class="settings-row">
        <span class="label">BMS</span>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'auto' }" @click="bmsInputPref = 'auto'">{{ t('settings.inputPref.auto') }}</button>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'normal' }" @click="bmsInputPref = 'normal'">{{ t('settings.inputPref.normal') }}</button>
        <button class="mode-btn" :class="{ active: bmsInputPref === 'triangular' }" @click="bmsInputPref = 'triangular'">{{ t('settings.inputPref.triangular') }}</button>
      </div>
      <div class="settings-section">{{ t('settings.section.experimental') }}</div>
      <div class="settings-row">
        <label class="settings-label">
          <input type="checkbox" v-model="enableMBOcf" /> {{ t('settings.mboEnable') }}
        </label>
      </div>
      <div v-if="enableMBOcf" class="settings-row">
        <span class="label">IHSS</span>
        <button class="mode-btn" :class="{ active: mboDisplayMode === 'matrix' }" @click="mboDisplayMode = 'matrix'">{{ t('settings.display.matrix') }}</button>
        <button class="mode-btn" :class="{ active: mboDisplayMode === 'flat' }" @click="mboDisplayMode = 'flat'">{{ t('settings.display.flat') }}</button>
        <button class="mode-btn" :class="{ active: mboDisplayMode === 'compact' }" @click="mboDisplayMode = 'compact'; mboCompactStyle = 'alpha'">{{ t('settings.display.compact') }}</button>
        <template v-if="mboDisplayMode === 'compact'">
          <button class="mode-btn sub" :class="{ active: mboCompactStyle === 'brace' }" @click="mboCompactStyle = 'brace'">{{ t('settings.compact.brace') }}</button>
          <button class="mode-btn sub" :class="{ active: mboCompactStyle === 'alpha' }" @click="mboCompactStyle = 'alpha'">{{ t('settings.compact.alpha') }}</button>
        </template>
        <label class="settings-label">
          <input type="checkbox" v-model="mboSugar" /> {{ t('settings.mboSugar') }}
        </label>
      </div>
      <div class="settings-section">{{ t('lang') }}</div>
      <div class="settings-row">
        <button class="mode-btn" :class="{ active: locale === 'en' }" @click="setLocale('en')">{{ t('lang.en') }}</button>
        <button class="mode-btn" :class="{ active: locale === 'zh' }" @click="setLocale('zh')">{{ t("lang.zh") }}</button>
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