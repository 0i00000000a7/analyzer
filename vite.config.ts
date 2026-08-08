import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import vue from '@vitejs/plugin-vue';

const { version } = JSON.parse(readFileSync('package.json', 'utf-8'));

export default defineConfig({
  base: './',
  root: '.',
  define: {
    __APP_VERSION__: JSON.stringify(version),
  },
  build: {
    outDir: 'dist',
  },
  server: {
    open: true,
    watch: {
      // Avoid watching the Rust build tree (huge; exhausts inotify watchers).
      ignored: ['**/src/rust/target/**', '**/target/**'],
    },
  },
  worker: {
    format: 'es',
  },
  plugins: [
    vue(),
  ],
});
