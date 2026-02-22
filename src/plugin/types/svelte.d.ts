import type { TocEntry } from '@parser';
import type { Component } from 'svelte';

export interface SvelteModule {
  metadata: Record<string, unknown>;
  toc: TocEntry[];
  default: Component;
}

declare module '*.norg' {
  export const metadata: Record<string, unknown>;
  export const toc: TocEntry[];
  const component: Component;
  export default component;
}
