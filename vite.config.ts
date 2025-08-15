import { defineConfig } from 'vite';
import { resolve } from 'path';
import dts from 'vite-plugin-dts';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
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
      external: ['vite', 'fs', 'path', 'url', 'react', 'node:fs/promises', 'node:path'],
    },
    copyPublicDir: false,
    outDir: 'dist',
  },
});
