<script lang="ts" setup vapor>
defineProps<{
  expandResult: string;
}>();

const fs = defineModel<string>('fs', { required: true });
const emit = defineEmits<{
  expand: [];
}>();
</script>

<template>
  <div class="section-divider" style="display: flex; flex-direction: column; gap: 4px; margin-top: 12px; padding-top: 8px">
    <div style="display: flex; align-items: center; gap: 8px">
      <span class="label" style="font-size: 12pt; width: 55px; text-align: right">Expand</span>
      <button class="mode-btn" style="font-size: 11pt" @click="emit('expand')">Expand</button>
      <span class="muted" style="font-size: 10pt">fs:</span>
      <input type="number" value="3" min="1" max="20" style="width: 50px; font-size: 11pt" v-model="fs" />
    </div>
    <div style="font-size: 14pt; margin-left: 63px; word-break: break-all" v-html="expandResult"></div>
  </div>
</template>
