import type { NorgParseResult } from '@parser';
import { dedent, addEmbedImports } from './helpers';

export function generateVue(
  { htmlParts, metadata, toc, embedComponents = [], embedCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const templateContent =
    embedComponents.length === 0
      ? '<div v-html="htmlContent"></div>'
      : `<div>\n${htmlParts
          .flatMap((part, i) => [
            `  <div v-html="htmlParts[${i}]"></div>`,
            ...(i < embedComponents.length ? [`  <Embed${i} />`] : []),
          ])
          .join('\n')}\n</div>`;

  return dedent`
    <script lang="ts">
    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};
    </script>
    <script setup lang="ts">
    ${css ? 'import "virtual:norg-arborium.css";' : null}
    ${addEmbedImports(embedComponents, filePath)}
    ${
      embedComponents.length > 0
        ? `const htmlParts = ${JSON.stringify(htmlParts)};`
        : `const htmlContent = ${JSON.stringify(htmlParts.join(''))};`
    }

    defineExpose({ metadata, toc });
    </script>

    <template>
      ${templateContent}
    </template>
    ${embedCss ? `<style>${embedCss}</style>` : null}
  `;
}
