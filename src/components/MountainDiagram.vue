<script lang="ts" setup vapor>
import { ref, watchEffect } from 'vue';
import { useI18n } from '../composables/useI18n';
import type { Mountain } from '../ts/types.js';
import { renderMountain0Y, renderMountainWY, renderMountain1Y } from '../ts/mountain-svg.js';
import jetbrainsFontUrl from '../assets/JetBrainsMono-Medium.ttf';
const { t } = useI18n();

const props = defineProps<{
  mountainType: '0y' | '1y' | 'wy' | 'hprss' | 'lprss' | null;
  mountainData: Mountain | null;
  mountainRowLabels: number[][] | null;
}>();

const containerRef = ref<HTMLDivElement | null>(null);
const hasSvg = ref(false);

watchEffect(() => {
  if (!containerRef.value) return;
  if (!props.mountainData || !props.mountainType) {
    containerRef.value.innerHTML = '';
    hasSvg.value = false;
    return;
  }
  let svg = '';
  if (props.mountainType === '0y' || props.mountainType === 'hprss' || props.mountainType === 'lprss') {
    svg = renderMountain0Y(props.mountainData);
  } else if (props.mountainType === 'wy' && props.mountainRowLabels) {
    svg = renderMountainWY(props.mountainData, props.mountainRowLabels);
  } else if (props.mountainType === '1y' && props.mountainRowLabels) {
    svg = renderMountain1Y(props.mountainData, props.mountainRowLabels);
  }
  containerRef.value.innerHTML = svg;
  hasSvg.value = svg.length > 0;
});

let jetbrainsDataUri: string | null = null;

async function loadJetbrainsDataUri(): Promise<string> {
  if (jetbrainsDataUri) return jetbrainsDataUri;
  const res = await fetch(jetbrainsFontUrl);
  const buf = await res.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  jetbrainsDataUri = 'data:font/truetype;base64,' + btoa(binary);
  return jetbrainsDataUri;
}

async function downloadMountain(): Promise<void> {
  const svgEl = containerRef.value?.querySelector('svg');
  if (!svgEl) return;
  const clone = svgEl.cloneNode(true) as SVGSVGElement;
  const rootStyle = getComputedStyle(document.documentElement);
  const resolve = (v: string) =>
    v.replace(/var\((--[\w-]+)\)/g, (_, name: string) => rootStyle.getPropertyValue(name).trim() || '#000');
  for (const el of Array.from(clone.querySelectorAll('*'))) {
    for (const attr of ['fill', 'stroke']) {
      const val = el.getAttribute(attr);
      if (val && val.includes('var(')) el.setAttribute(attr, resolve(val));
    }
  }
  const bg = rootStyle.getPropertyValue('--bg').trim() || '#ffffff';
  const bgRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
  bgRect.setAttribute('width', '100%');
  bgRect.setAttribute('height', '100%');
  bgRect.setAttribute('fill', bg);
  clone.insertBefore(bgRect, clone.firstChild);

  // Embed the JetBrains Mono font so the standalone image renders with it.
  const fontDataUri = await loadJetbrainsDataUri();
  const style = document.createElementNS('http://www.w3.org/2000/svg', 'style');
  style.textContent =
    `@font-face { font-family: jetbrains; src: url('${fontDataUri}') format('truetype'); }\n` +
    `text { font-family: jetbrains, monospace; }\n` +
    `svg { font-family: jetbrains, monospace; }`;
  clone.insertBefore(style, clone.firstChild);

  const serialized = new XMLSerializer().serializeToString(clone);
  const svgUrl = URL.createObjectURL(new Blob([serialized], { type: 'image/svg+xml;charset=utf-8' }));
  const img = new Image();
  img.onload = () => {
    const scale = 2;
    const w = (svgEl.width.baseVal.value || svgEl.clientWidth || 800) * scale;
    const h = (svgEl.height.baseVal.value || svgEl.clientHeight || 600) * scale;
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      URL.revokeObjectURL(svgUrl);
      return;
    }
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, h);
    ctx.drawImage(img, 0, 0, w, h);
    URL.revokeObjectURL(svgUrl);
    canvas.toBlob((blob) => {
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'mountain.png';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }, 'image/png');
  };
  img.onerror = () => URL.revokeObjectURL(svgUrl);
  img.src = svgUrl;
}
</script>

<template>
  <div class="section-divider" style="margin-top: 12px; padding-top: 8px">
    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px">
      <span class="label" style="font-size: 12pt; width: 68px; text-align: right">{{ t('mountain.label') }}</span>
      <button v-if="hasSvg" class="mode-btn" style="font-size: 10pt; margin-left: 8px" :title="t('mountain.download')" @click="downloadMountain">{{ t('mountain.download') }}</button>
    </div>
    <div ref="containerRef" style="overflow-x: auto; overflow-y: visible; padding: 8px 0"></div>
  </div>
</template>
