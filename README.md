# vite-plugin-norg

[![npm version](https://img.shields.io/npm/v/vite-plugin-norg.svg)](https://www.npmjs.com/package/vite-plugin-norg)
[![build status](https://img.shields.io/github/actions/workflow/status/bottd/vite-plugin-norg/publish.yml)](https://github.com/bottd/vite-plugin-norg/actions)
[![license](https://img.shields.io/npm/l/vite-plugin-norg.svg)](LICENSE)

**Neorg processor for Vite** - Transform `.norg` files into HTML, React, or Svelte with full TypeScript support.

> **Built for [Neorg](https://github.com/nvim-neorg/neorg) users, powered by [rust-norg](https://github.com/nvim-neorg/rust-norg)**

## Installation

```bash
npm install -D vite-plugin-norg
```

## Quick Setup

### HTML Output

```javascript
import { norgPlugin } from 'vite-plugin-norg';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'html',
      include: ['**/*.norg'],
    }),
  ],
});
```

```javascript
import { metadata, html } from './document.norg';
console.log(metadata.title); // "My Document"
document.body.innerHTML = html;
```

### React Components

```javascript
export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'react',
      include: ['**/*.norg'],
    }),
  ],
});
```

```jsx
import { metadata, Component } from './document.norg';

export default function App() {
  return (
    <div>
      <h1>{metadata.title}</h1>
      <Component />
    </div>
  );
}
```

### Svelte Components

```javascript
export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'svelte',
      include: ['**/*.norg'],
    }),
  ],
});
```

```svelte
<script>
  import Document, { metadata } from './document.norg';
</script>

<h1>{metadata.title}</h1>
<Document />
```

## Architecture

```mermaid
graph LR
    A(📝 .norg files) ==> B(⚡ Rust Parser)
    B ==> C(🔧 WASM Module)
    C ==> D(🚀 Vite Plugin)
    D ==> E{🎨 Generator}
    E ==> F(📄 HTML)
    E ==> G(⚛️ React)
    E ==> H(🔥 Svelte)
    F ==> I(💎 TypeScript Module)
    G ==> I
    H ==> I

    linkStyle default stroke-width:3px
```

Built with:

- **[rust-norg](https://github.com/nvim-neorg/rust-norg)** compiled to WASM for robust parsing
- **[vite-plugin-wasm](https://github.com/Menci/vite-plugin-wasm)** to load Rust HTML transformer
- **TypeScript** for the sanity of all involved
- **Vite integration** with HMR support

## Plugin API Reference

```typescript
import type { FilterPattern } from 'vite';
import type { BundledHighlighterOptions, BundledLanguage, BundledTheme } from 'shiki';

interface NorgPluginOptions {
  mode: 'html' | 'react' | 'svelte';
  include?: FilterPattern;
  exclude?: FilterPattern;
  shikiOptions?: BundledHighlighterOptions<BundledLanguage, BundledTheme>;
}
```

### Syntax Highlighting

The plugin automatically applies syntax highlighting to code blocks using [Shiki](https://shiki.style/). You can customize the highlighting themes and languages:

```javascript
norgPlugin({
  mode: 'html',
  shikiOptions: {
    themes: {
      // Optional, github theme applied if none specified
      light: 'github-light',
      dark: 'github-dark',
    },
    // Optional, all langs enabled by default
    langs: ['javascript', 'typescript', 'python'],
  },
});
```

Review the [Shiki documentation](https://shiki.style/guide) for theme and configuration options.

**Requirements:**

- Vite 7.0+
- Node.js 20+

## Development

This project uses Nix flakes and direnv for reproducible development environments.

### Setup

```bash
# Install direnv (if not already installed)
curl -sfL https://direnv.net/install.sh | bash

# Enable direnv for this project
direnv allow

# Build the project
npm run build
```

### Development Workflow

```bash
npm run build

# Run tests
npm test

# Lint code
npm run lint

# Format code
nix fmt
```

### Nix Flake

The `flake.nix` provides:

- Rust toolchain with [wasm-pack](https://github.com/rustwasm/wasm-pack)
- Node.js and npm
- Development tools and dependencies

To enter the development shell if not using direnv:

```bash
nix develop
```

## Contributing

PRs and issues welcome! To open a PR please fork and ensure tests and linters pass.
