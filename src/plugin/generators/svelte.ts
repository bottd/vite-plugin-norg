import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateSvelteOutput = ({ html, metadata, toc }: NorgParseResult, css: string) =>
  lines(
    `<script lang="ts" module>`,
    `  export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `  export const toc = ${JSON.stringify(toc ?? [])};`,
    `</script>`,
    `<script lang="ts">`,
    css && `  import "virtual:norg-arborium.css";`,
    `  const htmlContent = ${JSON.stringify(html)};`,
    `</script>`,
    `{@html htmlContent}`
  );
