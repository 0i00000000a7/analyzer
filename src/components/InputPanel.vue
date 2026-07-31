<script lang="ts" setup vapor>
import { computed } from 'vue';

type InputMode = 'bms' | '0y' | '1y' | 'wy' | 'bocf' | 'upms';

const mode = defineModel<InputMode>('mode', { required: true });
const value = defineModel<string>('value', { required: true });

const placeholder = computed(() => {
  switch (mode.value) {
    case '0y': return 'e.g. 1,4,8,11';
    case '1y': return 'e.g. 1,2,3,4';
    case 'wy': return 'e.g. 1,2,3,4';
    case 'bocf': return 'e.g. ψ(Ω) or \\psi(\\Omega)';
    case 'upms': return 'e.g. 0 111 211';
    default: return '';
  }
});

function setMode(m: InputMode) {
  mode.value = m;
}
</script>

<template>
  <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px; flex-wrap: wrap">
    <button class="mode-btn" :class="{ active: mode === 'bms' }" @click="setMode('bms')">BMS</button>
    <button class="mode-btn" :class="{ active: mode === '0y' }" @click="setMode('0y')">0-Y</button>
    <button class="mode-btn" :class="{ active: mode === '1y' }" @click="setMode('1y')">1-Y</button>
    <button class="mode-btn" :class="{ active: mode === 'wy' }" @click="setMode('wy')">ω-Y</button>
    <button class="mode-btn" :class="{ active: mode === 'bocf' }" @click="setMode('bocf')">BOCF</button>
    <button class="mode-btn" :class="{ active: mode === 'upms' }" @click="setMode('upms')">UPMS</button>
    <input style="flex: 1" :placeholder="placeholder" v-model="value" />
  </div>
</template>
