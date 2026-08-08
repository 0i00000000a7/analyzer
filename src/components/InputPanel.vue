<script lang="ts" setup vapor>
import { computed } from 'vue';
import { useI18n } from '../composables/useI18n';

const { t } = useI18n();

type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf' | 'upms' | 'hprss' | 'lprss' | 'hydra' | 'ihss' | 'mbo' | 'sss';

const mode = defineModel<InputMode>('mode', { required: true });
const value = defineModel<string>('value', { required: true });
defineProps<{ enableMBOcf: boolean }>();

const placeholder = computed(() => {
  switch (mode.value) {
    case 'bms': return t('placeholder.bms');
    case '0y': return t('placeholder.0y');
    case '1y': return t('placeholder.1y');
    case 'wy': return t('placeholder.wy');
    case 'bocf': return t('placeholder.bocf');
    case 'upms': return t('placeholder.upms');
    case 'hprss': return t('placeholder.hprss');
    case 'lprss': return t('placeholder.lprss');
    case 'hydra': return t('placeholder.hydra');
    case 'ihss': return t('placeholder.ihss');
    case 'mbo': return t('placeholder.mbo');
    case 'sss': return t('placeholder.sss');
    default: return '';
  }
});

function setMode(m: InputMode) {
  mode.value = m;
}
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 8px; margin-bottom: 4px">
    <div style="display: flex; justify-content: center; align-items: center; gap: 8px; flex-wrap: wrap">
      <button class="mode-btn" :class="{ active: mode === 'bms' }" @click="setMode('bms')">BMS</button>
      <button class="mode-btn" :class="{ active: mode === 'upms' }" @click="setMode('upms')">UPMS</button>
      <button class="mode-btn" :class="{ active: mode === 'hprss' }" @click="setMode('hprss')">HPrSS</button>
      <button class="mode-btn" :class="{ active: mode === 'lprss' }" @click="setMode('lprss')">LPrSS</button>
      <button class="mode-btn" :class="{ active: mode === '0y' }" @click="setMode('0y')">0-Y</button>
      <button class="mode-btn" :class="{ active: mode === '1y' }" @click="setMode('1y')">1-Y</button>
      <button class="mode-btn" :class="{ active: mode === 'wy' }" @click="setMode('wy')">ω-Y</button>
      <button class="mode-btn" :class="{ active: mode === 'bocf' }" @click="setMode('bocf')">BOCF</button>
      <button class="mode-btn" :class="{ active: mode === 'hydra' }" @click="setMode('hydra')">PSS Hydra</button>
      <template v-if="enableMBOcf">
        <button class="mode-btn" :class="{ active: mode === 'ihss' }" @click="setMode('ihss')">IHSS Hydra</button>
        <button class="mode-btn" :class="{ active: mode === 'mbo' }" @click="setMode('mbo')">Mahlo BOCF</button>
      </template>
      <button class="mode-btn" :class="{ active: mode === 'sss' }" @click="setMode('sss')">SSS</button>
    </div>
    <input style="width: 100%" :placeholder="placeholder" v-model="value" />
  </div>
</template>