import type { NorgParseResult } from '@parser';

export function generateReactOutput({ html, metadata }: NorgParseResult) {
  const metadataJson = JSON.stringify(metadata ?? {});
  const htmlJson = JSON.stringify(html);

  return [
    'import React from "react";',
    `export const metadata = ${metadataJson};`,
    `const htmlContent = ${htmlJson};`,
    'export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });',
    'export default Component;',
  ].join('\n');
}
