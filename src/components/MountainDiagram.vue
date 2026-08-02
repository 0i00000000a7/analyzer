<script lang="ts" setup vapor>
import { ref, watchEffect } from 'vue';
import type { Mountain } from '../ts/types.js';
import { renderMountain0Y, renderMountainWY, renderMountain1Y } from '../ts/mountain-svg.js';

const props = defineProps<{
  mountainType: '0y' | '1y' | 'wy' | 'hprss' | 'lprss' | null;
  mountainData: Mountain | null;
  mountainRowLabels: number[][] | null;
}>();

const containerRef = ref<HTMLDivElement | null>(null);

watchEffect(() => {
  if (!containerRef.value) return;
  if (!props.mountainData || !props.mountainType) {
    containerRef.value.innerHTML = '';
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
});
</script>

<template>
  <div class="section-divider" style="margin-top: 12px; padding-top: 8px">
    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px">
      <span class="label" style="font-size: 12pt; width: 68px; text-align: right">Mountain</span>
    </div>
    <div ref="containerRef" style="overflow-x: auto; overflow-y: visible; padding: 8px 0"></div>
  </div>
</template>
