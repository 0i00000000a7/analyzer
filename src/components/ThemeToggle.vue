<script lang="ts" setup vapor>
import { useTheme } from '../composables/useTheme';

const { isDark, toggle } = useTheme();
</script>

<template>
  <button id="theme-toggle" title="Toggle theme" @click="toggle">
    {{ isDark ? '☀' : '☾' }}
  </button>
</template>
