import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';

const version = readFileSync('version.txt', 'utf-8').trim();

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
  plugins: [
    {
      name: 'inject-version',
      transformIndexHtml(html) {
        return html.replace(
          '<span id="version" style="font-size: 11pt; color: #888; align-self: flex-end; margin-bottom: 2px"></span>',
          `<span id="version" style="font-size: 11pt; color: #888; align-self: flex-end; margin-bottom: 2px">v${version}</span>`,
        );
      },
    },
  ],
});
