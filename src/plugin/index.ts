export { norgPlugin } from './plugin';
export type { NorgMetadata, NorgParseResult, TocEntry } from './napi';

import { norgPlugin } from './plugin';
export default norgPlugin;

// Module types for different generators
import type { NorgMetadata } from './napi';
import type { SvelteComponent } from 'svelte';
import type { FC } from 'react';

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
  Component: FC;
}

/**
 * Module type for .norg files processed by the Svelte generator
 */
export interface SvelteModule {
  metadata: NorgMetadata;
  default: typeof SvelteComponent;
}
