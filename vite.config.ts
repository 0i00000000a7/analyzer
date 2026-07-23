import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  root: '.',
  build: {
    outDir: 'dist',
  },
  server: {
    open: true,
  },
  worker: {
    format: 'es',
  },
});
