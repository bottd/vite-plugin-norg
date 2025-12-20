import type { NorgParseResult } from '@parser';

export const generateHtmlOutput = ({ html, metadata, toc }: NorgParseResult, css: string) => {
  const lines: string[] = [];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  lines.push(
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const html = ${JSON.stringify(html)};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `export default { metadata, html, toc };`
  );
  return lines.join('\n');
};
