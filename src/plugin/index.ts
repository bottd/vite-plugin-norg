export { norgPlugin, type ArboriumConfig } from './plugin';
export type { NorgParseResult, TocEntry, InlineComponent } from '@parser';
export type { HtmlModule } from './generators/html';
export type { SvelteModule } from './generators/svelte';
export type { ReactModule } from './generators/react';
export type { VueModule } from './generators/vue';

import { norgPlugin } from './plugin';
export default norgPlugin;
