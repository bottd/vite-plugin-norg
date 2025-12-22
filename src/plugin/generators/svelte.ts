import type { NorgParseResult } from '@parser';

export function generateSvelteOutput({ html, metadata, toc }: NorgParseResult, css: string) {
  const lines = [
    '<script lang="ts" module>',
    `  export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `  export const toc = ${JSON.stringify(toc ?? [])};`,
    '</script>',
    '<script lang="ts">',
  ];
  if (css) lines.push('  import "virtual:norg-arborium.css";');
  lines.push(`  const htmlContent = ${JSON.stringify(html)};`, '</script>', '{@html htmlContent}');
  return lines.join('\n');
}
