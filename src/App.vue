<script lang="ts" setup vapor>
import { ref, watch } from 'vue';
import 'katex/dist/katex.min.css';
import './assets/default.css';
import ThemeToggle from './components/ThemeToggle.vue';
import SettingsPanel from './components/SettingsPanel.vue';
import InputPanel from './components/InputPanel.vue';
import OutputPanel from './components/OutputPanel.vue';
import ExpandPanel from './components/ExpandPanel.vue';
import MountainDiagram from './components/MountainDiagram.vue';
import { useAnalysis, type InputMode, type VeblenMode, type BocfDisplayMode, type LmnDisplayMode, type BmsDisplayMode, type UpmsDisplayMode, type BmsCompactStyle, type BmsInputPreference, type MboDisplayMode, type MboCompactStyle } from './composables/useAnalysis';
import { useI18n } from './composables/useI18n';

declare const __APP_VERSION__: string;
const version = __APP_VERSION__;

const { t } = useI18n();

const SETTINGS_KEY = 'bms-analyzer-settings';

interface Settings {
  veblenMode: VeblenMode;
  sugarEnabled: boolean;
  bocfDisplayMode: BocfDisplayMode;
  lmnDisplayMode: LmnDisplayMode;
  bmsDisplayMode: BmsDisplayMode;
  upmsDisplayMode: UpmsDisplayMode;
  bmsCompactStyle: BmsCompactStyle;
  upmsCompactStyle: BmsCompactStyle;
  bmsInputPref: BmsInputPreference;
  mboDisplayMode: MboDisplayMode;
  mboCompactStyle: MboCompactStyle;
  mboSugar: boolean;
  nocfSugar: boolean;
}

const defaults: Settings = {
  veblenMode: 'v',
  sugarEnabled: true,
  bocfDisplayMode: 'normal',
  lmnDisplayMode: 'full',
  bmsDisplayMode: 'flat',
  upmsDisplayMode: 'flat',
  bmsCompactStyle: 'alpha',
  upmsCompactStyle: 'alpha',
  bmsInputPref: 'auto',
  mboDisplayMode: 'flat',
  mboCompactStyle: 'alpha',
  mboSugar: true,
  nocfSugar: true,
};

function loadSettings(): Settings {
  try {
    const raw = JSON.parse(localStorage.getItem(SETTINGS_KEY) || '{}');
    const s = { ...defaults, ...raw };
    if (!['v', 'm'].includes(s.veblenMode)) s.veblenMode = defaults.veblenMode;
    if (!['normal', 'psi'].includes(s.bocfDisplayMode)) s.bocfDisplayMode = defaults.bocfDisplayMode;
    if (!['full', 'simple'].includes(s.lmnDisplayMode)) s.lmnDisplayMode = defaults.lmnDisplayMode;
    if (!['matrix', 'flat', 'compact'].includes(s.bmsDisplayMode)) s.bmsDisplayMode = defaults.bmsDisplayMode;
    if (!['matrix', 'flat', 'compact'].includes(s.upmsDisplayMode)) s.upmsDisplayMode = defaults.upmsDisplayMode;
    if (!['brace', 'alpha'].includes(s.bmsCompactStyle)) s.bmsCompactStyle = defaults.bmsCompactStyle;
    if (!['brace', 'alpha'].includes(s.upmsCompactStyle)) s.upmsCompactStyle = defaults.upmsCompactStyle;
    if (!['auto', 'normal', 'triangular'].includes(s.bmsInputPref)) s.bmsInputPref = defaults.bmsInputPref;
    if (!['matrix', 'flat', 'compact'].includes(s.mboDisplayMode)) s.mboDisplayMode = defaults.mboDisplayMode;
    if (!['brace', 'alpha'].includes(s.mboCompactStyle)) s.mboCompactStyle = defaults.mboCompactStyle;
    if (typeof s.mboSugar !== 'boolean') s.mboSugar = defaults.mboSugar;
    return s;
  } catch {
    return { ...defaults };
  }
}

const saved = loadSettings();
const inputMode = ref<InputMode>('bms');
const inputValue = ref('');
const veblenMode = ref<VeblenMode>(saved.veblenMode);
const sugarEnabled = ref<boolean>(saved.sugarEnabled);
const bocfDisplayMode = ref<BocfDisplayMode>(saved.bocfDisplayMode);
const lmnDisplayMode = ref<LmnDisplayMode>(saved.lmnDisplayMode);
const bmsDisplayMode = ref<BmsDisplayMode>(saved.bmsDisplayMode);
const upmsDisplayMode = ref<UpmsDisplayMode>(saved.upmsDisplayMode);
const bmsCompactStyle = ref<BmsCompactStyle>(saved.bmsCompactStyle);
const upmsCompactStyle = ref<BmsCompactStyle>(saved.upmsCompactStyle);
const bmsInputPref = ref<BmsInputPreference>(saved.bmsInputPref);
const mboDisplayMode = ref<MboDisplayMode>(saved.mboDisplayMode);
const mboCompactStyle = ref<MboCompactStyle>(saved.mboCompactStyle);
const mboSugar = ref<boolean>(saved.mboSugar);
const nocfSugar = ref<boolean>(saved.nocfSugar);

watch([veblenMode, sugarEnabled, bocfDisplayMode, lmnDisplayMode, bmsDisplayMode, upmsDisplayMode, bmsCompactStyle, upmsCompactStyle, bmsInputPref, mboDisplayMode, mboCompactStyle, mboSugar, nocfSugar], () => {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify({
    veblenMode: veblenMode.value,
    sugarEnabled: sugarEnabled.value,
    bocfDisplayMode: bocfDisplayMode.value,
    lmnDisplayMode: lmnDisplayMode.value,
    bmsDisplayMode: bmsDisplayMode.value,
    upmsDisplayMode: upmsDisplayMode.value,
    bmsCompactStyle: bmsCompactStyle.value,
    upmsCompactStyle: upmsCompactStyle.value,
    bmsInputPref: bmsInputPref.value,
    mboDisplayMode: mboDisplayMode.value,
    mboCompactStyle: mboCompactStyle.value,
    mboSugar: mboSugar.value,
    nocfSugar: nocfSugar.value,
  }));
});

const analysis = useAnalysis(inputMode, inputValue, veblenMode, sugarEnabled, bocfDisplayMode, bmsDisplayMode, upmsDisplayMode, bmsCompactStyle, upmsCompactStyle, bmsInputPref, mboDisplayMode, mboCompactStyle, mboSugar, nocfSugar, lmnDisplayMode);
</script>

<template>
  <ThemeToggle />
  <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 4px">
    <h1 style="margin: 0">{{ t('app.title') }}</h1>
    <span class="muted" style="font-size: 11pt; align-self: flex-end; margin-bottom: 2px">v{{ version }}</span>
    <SettingsPanel v-model:veblenMode="veblenMode" v-model:sugarEnabled="sugarEnabled" v-model:bocfDisplayMode="bocfDisplayMode" v-model:lmnDisplayMode="lmnDisplayMode" v-model:bmsDisplayMode="bmsDisplayMode" v-model:upmsDisplayMode="upmsDisplayMode" v-model:bmsCompactStyle="bmsCompactStyle" v-model:upmsCompactStyle="upmsCompactStyle" v-model:bmsInputPref="bmsInputPref" v-model:mboDisplayMode="mboDisplayMode" v-model:mboCompactStyle="mboCompactStyle" v-model:mboSugar="mboSugar" v-model:nocfSugar="nocfSugar" />
  </div>
  <InputPanel v-model:mode="inputMode" v-model:value="inputValue" />
  <div v-if="analysis.nonStandard.value" style="display: flex; justify-content: center; align-items: center; gap: 12px; margin-top: 6px">
    <span style="color: #d97706; font-size: 11pt">{{ t('warning.nonStandardCritical') }}</span>
    <button class="mode-btn" @click="analysis.forceNonStandardConvert()">{{ t('warning.convertAnyway') }}</button>
  </div>
  <div v-if="analysis.bocfNonStandardWarning.value || analysis.hydraNonStandardWarning.value || analysis.ihssNonStandardWarning.value || analysis.sssNonStandardWarning.value" style="display: flex; justify-content: center; margin-top: 6px">
    <span style="color: #ca8a04; font-size: 10pt">{{ t('warning.nonStandardMild') }}</span>
  </div>
  <hr />
  <OutputPanel
    :inputMode="inputMode"
    :ordinalHtml="analysis.ordinalHtml.value"
    :veblenHtml="analysis.veblenHtml.value"
    :lmnHtml="analysis.lmnHtml.value"
    :zeroYHtml="analysis.zeroYHtml.value"
    :dbmsHtml="analysis.dbmsHtml.value"
    :bmsHtml="analysis.bmsHtml.value"
    :triangularHtml="analysis.triangularHtml.value"
    :upmsHtml="analysis.upmsHtml.value"
    :hydraHtml="analysis.hydraHtml.value"
    :hprssHtml="analysis.hprssHtml.value"
    :lprssHtml="analysis.lprssHtml.value"
    :mboHtml="analysis.mboHtml.value"
    :mboMatrix="analysis.mboMatrix.value"
    :mboAstHtml="analysis.mboAstHtml.value"
    :showDbmsRow="analysis.showDbmsRow.value"
    :showBmsRow="analysis.showBmsRow.value"
    :showTriangularRow="analysis.showTriangularRow.value"
    :showUpmsRow="analysis.showUpmsRow.value"
    :showHydraRow="analysis.showHydraRow.value"
    :showHprssRow="analysis.showHprssRow.value"
    :showLprssRow="analysis.showLprssRow.value"
    :showMboRow="analysis.showMboRow.value"
    :showMboAstRow="analysis.showMboAstRow.value"
    :sssNocfHtml="analysis.sssNocfHtml.value"
    :showSssNocfRow="analysis.showSssNocfRow.value"
    :sssTprssHtml="analysis.sssTprssHtml.value"
    :showSssTprssRow="analysis.showSssTprssRow.value"
    :nocfHtml="analysis.nocfHtml.value"
    :mocfHtml="analysis.mocfHtml.value"
    :bocfMocfHtml="analysis.bocfMocfHtml.value"
    :converting="analysis.converting.value"
    :convertStatus="analysis.convertStatus.value"
    @convert="analysis.convertBocfToBms"
    @cancel="analysis.cancelConvert"
  />
  <ExpandPanel
    :expandResult="analysis.expandResult.value"
    v-model:fs="analysis.expandFs.value"
    @expand="analysis.doExpand"
  />
  <MountainDiagram
    v-if="analysis.showMountainRow.value"
    :mountainType="analysis.mountainType.value"
    :mountainData="analysis.mountainData.value"
    :mountainRowLabels="analysis.mountainRowLabels.value"
  />
  <div v-if="inputMode === 'sss'" style="display: flex; justify-content: center; margin-top: 10px">
    <span style="color: #ca8a04; font-size: 10pt; max-width: 720px; text-align: center">{{ t('warning.sssBocfUnstable') }}</span>
  </div>
</template>
