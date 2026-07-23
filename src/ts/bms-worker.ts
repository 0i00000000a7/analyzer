/**
 * Web Worker for running BMS WASM computation off the main thread.
 * WASM module URL is passed from the main thread via init message.
 */
let wasmModule: any = null;

async function init(wasmModuleUrl: string, wasmBaseUrl: string) {
  const mod = await import(/* @vite-ignore */ wasmModuleUrl);
  wasmModule = await (mod.default as Function)({
    locateFile: (path: string) => wasmBaseUrl + path,
  });
}

self.onmessage = async (e: MessageEvent) => {
  const { type, id, input, wasmModuleUrl, wasmBaseUrl } = e.data;

  if (type === 'init') {
    try {
      await init(wasmModuleUrl, wasmBaseUrl);
      self.postMessage({ type: 'init', id });
    } catch (e: any) {
      self.postMessage({ type: 'init_error', id, error: String(e) });
    }
    return;
  }

  if (type === 'bocfToBMS') {
    if (!wasmModule) {
      self.postMessage({ type: 'error', id, error: 'WASM not initialized' });
      return;
    }
    try {
      const t0 = performance.now();
      const result = wasmModule.bocfToBMS(input, (progress: string) => {
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
