<script lang="ts" setup vapor>
import { computed } from 'vue';

type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf' | 'upms' | 'hprss' | 'lprss' | 'hydra';

const mode = defineModel<InputMode>('mode', { required: true });
const value = defineModel<string>('value', { required: true });

const placeholder = computed(() => {
  switch (mode.value) {
    case '0y': return 'e.g. 1,4,8,11';
    case '1y': return 'e.g. 1,2,3,4';
    case 'wy': return 'e.g. 1,2,3,4';
    case 'bocf': return 'e.g. ψ(Ω) or \\psi(\\Omega)';
    case 'upms': return 'e.g. 0 111 211';
    case 'hprss': return 'e.g. 1,4,6,6';
    case 'lprss': return 'e.g. 1,4,6,6';
    case 'hydra': return 'e.g. p1(p2(0)) or p1(p2(0)+p2(0))';
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
    </div>
    <input style="width: 100%" :placeholder="placeholder" v-model="value" />
  </div>
</template>
