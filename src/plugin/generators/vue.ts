import type { NorgParseResult } from '@parser';
import { dedent, addInlineImports } from './helpers';

export function generateVue(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const templateContent =
    inlineComponents.length === 0
      ? '<div v-html="htmlContent"></div>'
      : `<div>\n${htmlParts
          .flatMap((part, i) => [
            ...(part ? [`  <div v-html="htmlParts[${i}]"></div>`] : []),
            ...(i < inlineComponents.length ? [`  <Inline${i} />`] : []),
          ])
          .join('\n')}\n</div>`;

  return dedent`
    <script lang="ts">
    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};
    </script>
    <script setup lang="ts">
    ${css ? 'import "virtual:norg-arborium.css";' : null}
    ${addInlineImports(inlineComponents, filePath)}
    ${
      inlineComponents.length > 0
        ? `const htmlParts = ${JSON.stringify(htmlParts)};`
        : `const htmlContent = ${JSON.stringify(htmlParts.join(''))};`
    }

    defineExpose({ metadata, toc });
    </script>

    <template>
      ${templateContent}
    </template>
    ${inlineCss ? '' : null}
    ${inlineCss ? `<style>${inlineCss}</style>` : null}
  `;
}
