import { defineConfig } from 'vitest/config';

export default defineConfig({
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
  server: {
    fs: {
      allow: ['.'],
    },
  },
  assetsInclude: ['**/*.node'],
});
