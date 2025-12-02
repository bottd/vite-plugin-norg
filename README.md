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

## Setup

```typescript
import { defineConfig } from "vite";
import { norgPlugin } from "vite-plugin-norg";

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: "html",
    }),
  ],
});
```

### TypeScript

Add a type reference to `app.d.ts` based on your output target:

```typescript
// For Svelte
/// <reference types="vite-plugin-norg/svelte" />

// For React
/// <reference types="vite-plugin-norg/react" />

// For HTML
/// <reference types="vite-plugin-norg/html" />
```

This provides type checking for `.norg` modules

### HTML Output

```javascript
import { metadata, html } from "./document.norg";
console.log(metadata.title); // "My Document"
document.body.innerHTML = html;
```

### React Output

```jsx
import { metadata, Component } from "./document.norg";

export default function App() {
  return (
    <div>
      <h1>{metadata.title}</h1>
      <Component />
    </div>
  );
}
```

### Svelte Output

```svelte
<script>
  import Document, { metadata } from './document.norg';
</script>

<h1>{metadata.title}</h1>
<Document />
```

## Configuration Reference

```typescript
import type { FilterPattern } from "vite";

interface NorgPluginOptions {
  mode: "html" | "react" | "svelte";
  include?: FilterPattern;
  exclude?: FilterPattern;
  // See https://shiki.style/guide for all options
  shikiOptions?: {
    themes?: {
      // optional: defaults to 'github-light'
      light: string;
      // optional: defaults to 'github-dark'
      dark: string;
    };
  };
}
```

**Requirements:**

- Vite 7.0+
- React 19+ (if using `mode: 'react'`)
- Svelte 5+ (if using `mode: 'svelte'`)

## Development

This project uses Nix flakes and direnv for reproducible development environments.

### Setup

```bash
# Enable direnv
direnv allow

bun install
```

### Development Commands

```bash
# Run tests
npm test
cargo test

# Lint and format
nix fmt
```

## License

MIT © [Drake Bott](https://github.com/bottd)
