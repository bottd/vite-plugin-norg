import { defineConfig } from 'vite';
import { resolve } from 'path';
import dts from 'vite-plugin-dts';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [
    tsconfigPaths(),
    dts({
      include: ['src/**/*'],
      exclude: ['src/parser/**/*'],
      outDir: 'dist',
      copyDtsFiles: true,
    }),
  ],
  build: {
    lib: {
      entry: resolve(__dirname, 'src/plugin/index.ts'),
      name: 'VitePluginNorg',
      fileName: format => {
        if (format === 'es') {
          return 'plugin/index.js';
        }
        return 'plugin/index.js';
      },
      formats: ['es'],
    },
    rollupOptions: {
      external: ['vite', 'fs', 'path', 'url', 'react', 'node:fs/promises', 'node:path', '@parser'],
      output: {
        paths: {
          '@parser': '../napi/index.js',
        },
        // Preserve the runtime module structure
        preserveModules: true,
        preserveModulesRoot: resolve(__dirname, 'src'),
      },
    },
    copyPublicDir: false,
    outDir: 'dist',
    // needed to preserve build:napi output
    emptyOutDir: false,
  },
});
