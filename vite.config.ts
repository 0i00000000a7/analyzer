import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import vue from '@vitejs/plugin-vue';

const version = readFileSync('version.txt', 'utf-8').trim();

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
  },
  worker: {
    format: 'es',
  },
  plugins: [
    vue(),
  ],
});
