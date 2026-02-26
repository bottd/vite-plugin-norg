import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'html',
      arboriumConfig: { theme: 'github-dark' }
    })
  ],
  build: {
    rollupOptions: {
      input: {
        main: 'index.html',
        'getting-started': 'getting-started.html',
        configuration: 'configuration.html',
        'embed-components': 'embed-components.html'
      }
    }
  }
});
