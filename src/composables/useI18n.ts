import { ref } from 'vue';
import { en } from '../i18n/en';
import { zh } from '../i18n/zh';

export type Locale = 'en' | 'zh';

const messages: Record<Locale, Record<string, string>> = { en, zh };

const locale = ref<Locale>('en');
const saved = localStorage.getItem('locale');
if (saved === 'en' || saved === 'zh') {
  locale.value = saved;
}

export function useI18n() {
  function t(key: string): string {
    return messages[locale.value]?.[key] ?? key;
  }
  function setLocale(l: Locale) {
    locale.value = l;
    localStorage.setItem('locale', l);
  }
  return { locale, setLocale, t };
}