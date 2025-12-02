import type { NorgParseResult } from '@parser';

export const generateHtmlOutput = ({ html, metadata, toc }: NorgParseResult) => {
  return [
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const html = ${JSON.stringify(html)};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `export default { metadata, html, toc };`,
  ].join('\n');
};
