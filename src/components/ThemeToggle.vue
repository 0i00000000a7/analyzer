<script lang="ts" setup vapor>
import { useTheme } from '../composables/useTheme';
import { useI18n } from '../composables/useI18n';

const { isDark, toggle } = useTheme();
const { t } = useI18n();
</script>

<template>
  <button id="theme-toggle" :title="t('theme.toggle')" @click="toggle">
    {{ isDark ? '☀' : '☾' }}
  </button>
</template>
