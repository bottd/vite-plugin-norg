import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateHtmlOutput = ({ html, metadata, toc }: NorgParseResult, css: string) =>
  lines(
    css && `import "virtual:norg-arborium.css";`,
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const html = ${JSON.stringify(html)};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `export default { metadata, html, toc };`
  );
