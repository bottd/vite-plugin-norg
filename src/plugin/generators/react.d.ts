import type { TocEntry } from '@parser';
import type { ComponentType } from 'react';

export interface ReactModule {
  metadata: Record<string, unknown>;
  toc: TocEntry[];
  Component: ComponentType;
  default: ComponentType;
}

declare module '*.norg' {
  export const metadata: Record<string, unknown>;
  export const toc: TocEntry[];
  export const Component: ComponentType;
  const _default: ComponentType;
  export default _default;
}
