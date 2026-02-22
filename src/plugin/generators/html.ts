import type { NorgParseResult, InlineComponent } from '@parser';
import { dedent } from './helpers';

function embedInlines(htmlParts: string[], inlineComponents: InlineComponent[]): string {
  return htmlParts
    .flatMap((part, i) => (i < inlineComponents.length ? [part, inlineComponents[i].code] : [part]))
    .join('');
}

export function generateHtml(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string
): string {
  const raw = embedInlines(htmlParts, inlineComponents);
  const html = inlineCss ? `<style>${inlineCss}</style>${raw}` : raw;
  return dedent`
    ${css ? 'import "virtual:norg-arborium.css";' : null}

    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const html = ${JSON.stringify(html)};
    export const toc = ${JSON.stringify(toc ?? [])};

    export default { metadata, html, toc };
  `;
}
