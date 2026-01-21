import type { TocEntry } from '@parser';

export interface HtmlModule {
  metadata: Record<string, unknown>;
  html: string;
  toc: TocEntry[];
}

declare module '*.norg' {
  export const metadata: Record<string, unknown>;
  export const html: string;
  export const toc: TocEntry[];
  const _default: HtmlModule;
  export default _default;
}
