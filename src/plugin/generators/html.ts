import type { NorgParseResult, EmbedComponent } from '@parser';
import { dedent } from './helpers';

function mergeEmbeds(htmlParts: string[], embeds: EmbedComponent[]): string {
  return htmlParts
    .flatMap((part, i) => (i < embeds.length ? [part, embeds[i].code] : [part]))
    .join('');
}

export function generateHtml(
  { htmlParts, metadata, toc, embedComponents = [], embedCss = '' }: NorgParseResult,
  css: string
): string {
  const raw = mergeEmbeds(htmlParts, embedComponents);
  const html = embedCss ? `<style>${embedCss}</style>${raw}` : raw;
  return dedent`
    ${css ? 'import "virtual:norg-arborium.css";' : null}

    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const html = ${JSON.stringify(html)};
    export const toc = ${JSON.stringify(toc ?? [])};

    export default { metadata, html, toc };
  `;
}
