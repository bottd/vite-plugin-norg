export { norgPlugin, type ArboriumConfig } from './plugin';
export type { NorgParseResult, NorgMetadataResult, TocEntry } from '@parser';

import { norgPlugin } from './plugin';
export default norgPlugin;

import type { SvelteComponent } from 'svelte';
import type { FC } from 'react';
import type { NorgParseResult } from '@parser';

type NorgMetadata = NorgParseResult['metadata'];

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

/**
 * Module type for .norg files processed by the metadata generator
 */
export interface MetadataModule {
  metadata: NorgMetadata;
}
