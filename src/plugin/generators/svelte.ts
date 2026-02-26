import type { NorgParseResult } from '@parser';
import { dedent, addEmbedImports } from './helpers';

export function generateSvelte(
  { htmlParts, metadata, toc, embedComponents = [], embedCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const hasImports = !!(css || embedComponents.length);
  const body = htmlParts
    .flatMap((part, i) => [
      `{@html ${JSON.stringify(part)}}`,
      ...(i < embedComponents.length ? [`<Embed${i} />`] : []),
    ])
    .join('\n');

  return dedent`
    <script lang="ts" module>
      export const metadata = ${JSON.stringify(metadata ?? {})};
      export const toc = ${JSON.stringify(toc ?? [])};
    </script>
    ${hasImports ? '<script lang="ts">' : null}
      ${css ? 'import "virtual:norg-arborium.css";' : null}
      ${addEmbedImports(embedComponents, filePath)}
    ${hasImports ? '</script>' : null}
    ${embedCss ? `{@html ${JSON.stringify(`<style>${embedCss}</style>`)}}` : null}
    ${body}
  `;
}
