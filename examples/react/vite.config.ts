import react from '@vitejs/plugin-react';
import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'react',
      arboriumConfig: { theme: 'github-dark' },
    }),
    react(),
  ],
});
