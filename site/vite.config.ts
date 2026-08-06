import { sveltekit } from '@sveltejs/kit/vite';
import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'svelte',
      arboriumConfig: { theme: 'github-dark' },
    }),
    sveltekit(),
  ],
});
