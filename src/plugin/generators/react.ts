import type { NorgParseResult } from '@parser';

export function generateReactOutput({ html, metadata, toc }: NorgParseResult, css: string) {
  const lines = ['import React from "react";'];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  lines.push(
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `const htmlContent = ${JSON.stringify(html)};`,
    'export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });',
    'export default Component;'
  );
  return lines.join('\n');
}
