# vite-plugin-norg

A Vite plugin that enables importing and processing `.norg` files (Neorg markup format) in your Vite projects with support for HTML, Svelte, and React output targets.

- Built on top of the [rust-norg](https://github.com/nvim-neorg/rust-norg) parser

## Installation

```bash
npm install vite-plugin-norg
```

## Usage

### Basic Setup

Add the plugin to your `vite.config.js`:

```js
import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [norgPlugin({ mode: 'html', include: ['/**/*.norg'] })],
});
```
