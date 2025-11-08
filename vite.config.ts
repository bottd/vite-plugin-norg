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
    }),
  ],
  build: {
    lib: {
      entry: resolve(__dirname, 'src/plugin/index.ts'),
      name: 'VitePluginNorg',
      fileName: 'plugin/index',
      formats: ['es'],
    },
    rollupOptions: {
      external: ['vite', 'fs', 'path', 'url', 'react', 'node:fs/promises', 'node:path', '@parser'],
    },
    copyPublicDir: false,
    outDir: 'dist',
  },
});
