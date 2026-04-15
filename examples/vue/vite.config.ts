import vue from '@vitejs/plugin-vue';
import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'vue',
      arboriumConfig: { theme: 'github-dark' },
    }),
    vue(),
  ],
});
