import { ref, computed, onMounted, onUnmounted } from 'vue';

const theme = ref<'light' | 'dark' | null>(null);

export function useTheme() {
  const isDark = computed(() => {
    if (theme.value) return theme.value === 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  });

  const toggle = () => {
    theme.value = isDark.value ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', theme.value);
    localStorage.setItem('theme', theme.value);
  };

  onMounted(() => {
    const saved = localStorage.getItem('theme') as 'light' | 'dark' | null;
    if (saved) theme.value = saved;

    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => {
      if (!theme.value) {
        // Auto-detect, no need to update ref
      }
    };
    mq.addEventListener('change', handler);
    onUnmounted(() => mq.removeEventListener('change', handler));
  });

  return { theme, isDark, toggle };
}
