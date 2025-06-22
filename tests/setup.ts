import { vi } from 'vitest';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

// Setup WASM module for tests
vi.mock('../pkg/vite_plugin_norg_parser.js', async () => {
  const actualModule = (await vi.importActual('../pkg/vite_plugin_norg_parser.js')) as Record<
    string,
    unknown
  >;

  // Initialize the WASM module synchronously for tests
  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const wasmPath = join(__dirname, '../pkg/vite_plugin_norg_parser_bg.wasm');
  const wasmBuffer = readFileSync(wasmPath);

  if (actualModule.initSync) {
    actualModule.initSync({ module: wasmBuffer });
  }

  return actualModule;
});
