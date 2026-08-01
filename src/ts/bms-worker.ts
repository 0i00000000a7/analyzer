/**
 * Web Worker for running BMS WASM computation off the main thread.
 */
import initWasm, * as wasm from '../wasm/pkg/bms_wasm.js';

async function init() {
  await initWasm();
}

self.onmessage = async (e: MessageEvent) => {
  const { type, id, input } = e.data;

  if (type === 'init') {
    try {
      await init();
      self.postMessage({ type: 'init', id });
    } catch (e: any) {
      self.postMessage({ type: 'init_error', id, error: String(e) });
    }
    return;
  }

  if (type === 'bocfToBMS') {
    try {
      const t0 = performance.now();
      const result = wasm.bocfToBMS(input, (progress: string) => {
        self.postMessage({ type: 'progress', id, data: progress });
      });
      console.log('worker bocfToBMS total: ' + (performance.now() - t0).toFixed(0) + 'ms');
      self.postMessage({
        type: 'result',
        id,
        result: result.result,
        error: result.error,
      });
    } catch (e: any) {
      self.postMessage({ type: 'error', id, error: String(e) });
    }
    return;
  }
};
