export { norgPlugin, type ArboriumConfig, type NorgPluginOptions } from './plugin';
export type { NorgParseResult, TocEntry, InlineComponent } from '@parser';
export type { HtmlModule } from './types/html';
export type { SvelteModule } from './types/svelte';
export type { ReactModule } from './types/react';
export type { VueModule } from './types/vue';
export type { MetadataModule } from './types/metadata';

import { norgPlugin } from './plugin';
export default norgPlugin;
