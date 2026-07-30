import katex from 'katex';
import { watchEffect } from 'vue';

export const vKatex = (el: HTMLElement, source: () => string) => {
  watchEffect(() => {
    const latex = source();
    if (latex) {
      el.innerHTML = katex.renderToString(latex, { throwOnError: false });
    } else {
      el.textContent = '';
    }
  });
  return () => {};
};
