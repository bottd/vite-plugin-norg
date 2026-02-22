import type { NorgParseResult } from '@parser';
import { dedent, addInlineImports } from './helpers';

export function generateSvelte(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const hasImports = !!(css || inlineComponents.length);
  const body = htmlParts
    .flatMap((part, i) => [
      ...(part ? [`{@html ${JSON.stringify(part)}}`] : []),
      ...(i < inlineComponents.length ? [`<Inline${i} />`] : []),
    ])
    .join('\n');

  return dedent`
    <script lang="ts" module>
      export const metadata = ${JSON.stringify(metadata ?? {})};
      export const toc = ${JSON.stringify(toc ?? [])};
    </script>
    ${hasImports ? '<script lang="ts">' : null}
      ${css ? 'import "virtual:norg-arborium.css";' : null}
      ${addInlineImports(inlineComponents, filePath)}
    ${hasImports ? '</script>' : null}
    ${inlineCss ? `{@html ${JSON.stringify(`<style>${inlineCss}</style>`)}}` : null}
    ${body}
  `;
}
