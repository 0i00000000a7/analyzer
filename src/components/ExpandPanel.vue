<script lang="ts" setup vapor>
import { ref } from 'vue';
import { useI18n } from '../composables/useI18n';
const { t } = useI18n();

defineProps<{
  expandResult: string;
}>();

const fs = defineModel<string>('fs', { required: true });
const emit = defineEmits<{
  expand: [];
}>();

const copied = ref(false);

function copyExp() {
  const el = document.querySelector('[data-expand-content]');
  if (!el) return;
  navigator.clipboard.writeText(el.textContent || '').catch(() => {});
  copied.value = true;
  setTimeout(() => { copied.value = false; }, 1500);
}
</script>

<template>
  <div class="section-divider" style="display: flex; flex-direction: column; gap: 4px; margin-top: 12px; padding-top: 8px">
    <div style="display: flex; align-items: center; gap: 8px">
      <span class="label" style="font-size: 12pt; width: 70px; text-align: right; cursor: pointer" @click="copyExp()" :title="t('output.copy')">{{ t('expand.label') }}</span>
      <button class="mode-btn" style="font-size: 11pt" @click="emit('expand')">{{ t('expand.button') }}</button>
      <span class="muted" style="font-size: 10pt">{{ t('expand.fs') }}</span>
      <input type="number" min="1" max="20" style="width: 50px; font-size: 11pt" v-model="fs" />
      <span v-if="copied" class="muted" style="font-size: 9pt">{{ t('output.copied') }}</span>
    </div>
    <div data-expand-content style="font-size: 14pt; margin-left: 63px; word-break: break-all" v-html="expandResult"></div>
  </div>
</template>
