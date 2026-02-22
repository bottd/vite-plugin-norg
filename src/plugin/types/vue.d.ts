import type { TocEntry } from '@parser';
import type { DefineComponent } from 'vue';

export interface VueModule {
  metadata: Record<string, unknown>;
  toc: TocEntry[];
  default: DefineComponent<object, object, unknown>;
}

declare module '*.norg' {
  export const metadata: Record<string, unknown>;
  export const toc: TocEntry[];
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
