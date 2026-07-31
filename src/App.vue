<script lang="ts" setup vapor>
import { ref } from 'vue';
import 'katex/dist/katex.min.css';
import './assets/default.css';
import ThemeToggle from './components/ThemeToggle.vue';
import SettingsPanel from './components/SettingsPanel.vue';
import InputPanel from './components/InputPanel.vue';
import OutputPanel from './components/OutputPanel.vue';
import ExpandPanel from './components/ExpandPanel.vue';
import MountainDiagram from './components/MountainDiagram.vue';
import { useAnalysis, type InputMode, type VeblenMode, type BmsDisplayMode, type UpmsDisplayMode, type BmsInputPreference } from './composables/useAnalysis';

declare const __APP_VERSION__: string;
const version = __APP_VERSION__;

const inputMode = ref<InputMode>('bms');
const inputValue = ref('(0,0,0)(1,1,1)(2,1,0)(1,1,1)');
const veblenMode = ref<VeblenMode>('v');
const sugarEnabled = ref(true);
const bmsDisplayMode = ref<BmsDisplayMode>('flat');
const upmsDisplayMode = ref<UpmsDisplayMode>('flat');
const bmsInputPref = ref<BmsInputPreference>('auto');

const analysis = useAnalysis(inputMode, inputValue, veblenMode, sugarEnabled, bmsDisplayMode, upmsDisplayMode, bmsInputPref);
</script>

<template>
  <ThemeToggle />
  <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 4px">
    <h1 style="margin: 0">BMS analyzer</h1>
    <span class="muted" style="font-size: 11pt; align-self: flex-end; margin-bottom: 2px">v{{ version }}</span>
    <SettingsPanel v-model:veblenMode="veblenMode" v-model:sugarEnabled="sugarEnabled" v-model:bmsDisplayMode="bmsDisplayMode" v-model:upmsDisplayMode="upmsDisplayMode" v-model:bmsInputPref="bmsInputPref" />
  </div>
  <InputPanel v-model:mode="inputMode" v-model:value="inputValue" />
  <hr />
  <OutputPanel
    :inputMode="inputMode"
    :ordinalHtml="analysis.ordinalHtml.value"
    :veblenHtml="analysis.veblenHtml.value"
    :zeroYHtml="analysis.zeroYHtml.value"
    :dbmsHtml="analysis.dbmsHtml.value"
    :bmsHtml="analysis.bmsHtml.value"
    :triangularHtml="analysis.triangularHtml.value"
    :upmsHtml="analysis.upmsHtml.value"
    :showDbmsRow="analysis.showDbmsRow.value"
    :showBmsRow="analysis.showBmsRow.value"
    :showTriangularRow="analysis.showTriangularRow.value"
    :showUpmsRow="analysis.showUpmsRow.value"
    :converting="analysis.converting.value"
    :convertStatus="analysis.convertStatus.value"
    @convert="analysis.convertBocfToBms"
    @cancel="analysis.cancelConvert"
  />
  <ExpandPanel
    :expandResult="analysis.expandResult.value"
    v-model:fs="analysis.expandFs"
    @expand="analysis.doExpand"
  />
  <MountainDiagram
    v-if="analysis.showMountainRow.value"
    :mountainType="analysis.mountainType.value"
    :mountainData="analysis.mountainData.value"
    :mountainRowLabels="analysis.mountainRowLabels.value"
  />
</template>
