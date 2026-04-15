# vite-plugin-norg

[![npm version](https://img.shields.io/npm/v/vite-plugin-norg.svg)](https://www.npmjs.com/package/vite-plugin-norg)
[![build status](https://img.shields.io/github/actions/workflow/status/bottd/vite-plugin-norg/release.yml)](https://github.com/bottd/vite-plugin-norg/actions)
[![license](https://img.shields.io/npm/l/vite-plugin-norg.svg)](LICENSE)

**Neorg processor for Vite** - Transform `.norg` files into HTML, React, Svelte, or Vue with full TypeScript support.

## Installation

```bash
npm install -D vite-plugin-norg
```

## Setup

### Configuration

```typescript
import type { FilterPattern } from 'vite';

interface NorgPluginOptions {
  mode: 'html' | 'react' | 'svelte' | 'vue' | 'metadata';
  include?: FilterPattern;
  exclude?: FilterPattern;

  arboriumConfig?: {
    // Single theme
    theme?: string;
    // Or light/dark (uses prefers-color-scheme)
    themes?: {
      light: string;
      dark: string;
    };
  };

  // Directory to scan for framework components
  componentDir?: string;

  // (takes precedence over componentDir)
  // { Component: "import-path" }
  components?: Record<string, string>;
}

// vite.config.ts
import { defineConfig } from 'vite';
import { norgPlugin } from 'vite-plugin-norg';

export default defineConfig({
  plugins: [
    norgPlugin({
      mode: 'svelte',
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

// For Vue
/// <reference types="vite-plugin-norg/vue" />

// For HTML
/// <reference types="vite-plugin-norg/html" />

// For Metadata
/// <reference types="vite-plugin-norg/metadata" />
```

This provides type checking for `.norg` modules

### HTML Usage

```javascript
import { metadata, html } from './document.norg';
console.log(metadata.title); // "My Document"
document.body.innerHTML = html;
```

### React Usage

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

### Svelte Usage

```svelte
<script>
  import Document, { metadata } from './document.norg';
</script>

<h1>{metadata.title}</h1>
<Document />
```

### Vue Usage

```vue
<script setup>
import Document, { metadata } from './document.norg';
</script>

<template>
  <h1>{{ metadata.title }}</h1>
  <Document />
</template>
```

### Metadata Usage

```javascript
import { metadata, toc } from './document.norg';
console.log(metadata.title); // "My Document"
console.log(toc); // [{ title: "Section 1", level: 1 }, ...]
```

You can also append `?metadata` to any import to get metadata-only output regardless of mode:

```javascript
import { metadata, toc } from './document.norg?metadata';
```

## Code Syntax Highlighting

Code blocks are highlighted using [arborium](https://arborium.bearcove.eu/), which generates highlights via tree-sitter. Set a theme to include highlights:

```typescript
norgPlugin({
  mode: 'html',
  arboriumConfig: { theme: 'github-dark' },
});
```

See the [arborium themes](https://github.com/bearcove/arborium?tab=readme-ov-file#themes) for available options.

## Embed Components

Embed components can be referenced within `.norg` documents using `@embed`:

```norg
* Example document
With some regular text

@embed svelte
<Chart variant="bar" />
@end
```

To configure components for usage with embeds, set either `componentDir` or map imports directly with `components`.

```typescript
norgPlugin({
  mode: 'svelte',
  componentDir: './src/components',
  components: {
    Chart: './src/lib/Chart.svelte',
  },
});
```

## Embed Styles

Document styles can be embedded as well using `@embed css`. All frameworks will import embedded styles when rendering the document.

```norg
* Example document
With some regular text

@embed css
  h2 {
    color: red;
  }
@end
```

**Requirements:**

- Vite 7.0+
- React 19+ (if using `mode: 'react'`)
- Svelte 5+ (if using `mode: 'svelte'`)
- Vue 3+ (if using `mode: 'vue'`)

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
