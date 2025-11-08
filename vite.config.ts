import { defineConfig } from 'vite';
import { resolve } from 'path';
import dts from 'vite-plugin-dts';

export default defineConfig({
  plugins: [
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
      external: ['vite', 'fs', 'path', 'url', 'react', 'node:fs/promises', 'node:path', /\.node$/],
    },
    copyPublicDir: false,
    outDir: 'dist',
  },
});
