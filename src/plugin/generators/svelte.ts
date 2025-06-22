import { NorgParseResult } from '../wasm';

export function generateSvelteOutput({ html, metadata }: NorgParseResult) {
  const metadataJson = JSON.stringify(metadata ?? {});

  return [
    '<script>',
    `  const htmlContent = ${JSON.stringify(html)};`,
    '</script>',
    '<script context="module">',
    `  export const metadata = ${metadataJson};`,
    '</script>',
    '{@html htmlContent}',
  ].join('\n');
}
