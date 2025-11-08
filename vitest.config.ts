import { defineConfig } from 'vitest/config';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [tsconfigPaths()],
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    exclude: ['node_modules/**', '.direnv/**'],
    snapshotFormat: {
      printBasicPrototype: false,
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
});
