import { defineConfig } from 'vitest/config';
import wasmPlugin from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasmPlugin(), topLevelAwait()],
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    exclude: ['node_modules/**', '.direnv/**'],
    snapshotFormat: {
      printBasicPrototype: false, // Cleaner output (Vitest default)
      escapeString: false,
      printFunctionName: false,
    },
    setupFiles: ['./tests/setup.ts'],
  },
});
