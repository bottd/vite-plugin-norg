import type { NorgParseResult } from '@parser';

export function generateReactOutput({ html, metadata, toc }: NorgParseResult) {
  return [
    'import React from "react";',
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `const htmlContent = ${JSON.stringify(html)};`,
    'export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });',
    'export default Component;',
  ].join('\n');
}
