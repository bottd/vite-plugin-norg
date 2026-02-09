import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateHtmlOutput = ({ html, metadata, toc }: NorgParseResult, css: string) =>
  lines`
    ${css && `import "virtual:norg-arborium.css";`}
    export const metadata = ${metadata ?? {}};
    export const html = ${html};
    export const toc = ${toc ?? []};
    export default { metadata, html, toc };
  `;
