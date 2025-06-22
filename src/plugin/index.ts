export { norgPlugin } from './plugin';
export type { NorgMetadata, NorgParseResult, TocEntry } from './wasm';

import { norgPlugin } from './plugin';
export default norgPlugin;

// Module types for different generators
import type { NorgMetadata } from './wasm';

/**
 * Module type for .norg files processed by the HTML generator
 */
export interface HtmlModule {
  metadata: NorgMetadata;
  html: string;
}

/**
 * Module type for .norg files processed by the React generator
 */
export interface ReactModule {
  metadata: NorgMetadata;
  Component: () => unknown;
}

/**
 * Module type for .norg files processed by the Svelte generator
 */
export interface SvelteModule {
  metadata: NorgMetadata;
  default: new (options: { target: unknown; props?: Record<string, unknown> }) => {
    $destroy(): void;
    $set(props: Record<string, unknown>): void;
  };
}
