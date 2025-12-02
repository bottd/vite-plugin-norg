import type { NorgParseResult } from '@parser';

export function generateSvelteOutput({ html, metadata, toc }: NorgParseResult) {
  return [
    '<script lang="ts" module>',
    `  export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `  export const toc = ${JSON.stringify(toc ?? [])};`,
    '</script>',
    '<script lang="ts">',
    `  const htmlContent = ${JSON.stringify(html)};`,
    '</script>',
    '{@html htmlContent}',
  ].join('\n');
}
