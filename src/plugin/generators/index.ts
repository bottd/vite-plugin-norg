import type { NorgParseResult, InlineComponent } from '@parser';

export type GeneratorMode = 'html' | 'svelte' | 'react' | 'vue';

/**
 * Generate output for the specified framework mode
 */
export function generateOutput(
  mode: GeneratorMode,
  result: NorgParseResult,
  css: string,
  filePath?: string
): string {
  switch (mode) {
    case 'html':
      return generateHtml(result, css);
    case 'svelte':
      return generateSvelte(result, css, filePath);
    case 'react':
      return generateReact(result, css);
    case 'vue':
      return generateVue(result, css, filePath);
  }
}

function generateHtml({ htmlParts, metadata, toc }: NorgParseResult, css: string): string {
  const html = htmlParts.join('');
  const lines: string[] = [];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  lines.push(
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const html = ${JSON.stringify(html)};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `export default { metadata, html, toc };`
  );
  return lines.join('\n');
}

function generateSvelte(
  { htmlParts, metadata, toc, inlines = [] }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const lines: string[] = [
    '<script lang="ts" module>',
    `  export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `  export const toc = ${JSON.stringify(toc ?? [])};`,
    '</script>',
  ];

  // Only add script block if it has content
  const scriptContent: string[] = [];
  if (css) scriptContent.push('  import "virtual:norg-arborium.css";');
  addInlineImports(scriptContent, inlines, filePath, '  ');

  if (scriptContent.length > 0) {
    lines.push('<script lang="ts">', ...scriptContent, '</script>');
  }

  // Interleave HTML parts with inline components
  interleaveHtmlAndInlines(
    lines,
    htmlParts,
    inlines,
    part => `{@html ${JSON.stringify(part)}}`,
    i => `<Inline${i} />`
  );

  return lines.join('\n');
}

function generateReact(
  { htmlParts, metadata, toc }: NorgParseResult,
  css: string,
): string {
  const html = htmlParts.join('');
  const lines: string[] = ['import React from "react";'];
  if (css) lines.push('import "virtual:norg-arborium.css";');

  lines.push(
    '',
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    '',
    `const htmlContent = ${JSON.stringify(html)};`,
    '',
    'export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });',
    'export default Component;'
  );

  return lines.join('\n');
}

function generateVue(
  { htmlParts, metadata, toc, inlines = [] }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  // Separate <script> block for module exports (script setup vars aren't exports)
  const lines: string[] = [
    '<script lang="ts">',
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    '</script>',
    '<script setup lang="ts">',
  ];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  addInlineImports(lines, inlines, filePath);

  if (inlines.length > 0) {
    lines.push(`const htmlParts = ${JSON.stringify(htmlParts)};`);
  } else {
    lines.push(`const htmlContent = ${JSON.stringify(htmlParts.join(''))};`);
  }

  lines.push('', 'defineExpose({ metadata, toc });');
  lines.push('</script>', '', '<template>');

  if (inlines.length === 0) {
    lines.push('  <div v-html="htmlContent"></div>');
  } else {
    lines.push('  <div>');
    interleaveHtmlAndInlines(
      lines,
      htmlParts,
      inlines,
      (_part, i) => `    <span v-html="htmlParts[${i}]"></span>`,
      i => `    <Inline${i} />`
    );
    lines.push('  </div>');
  }

  lines.push('</template>');
  return lines.join('\n');
}

function addInlineImports(
  lines: string[],
  inlines: InlineComponent[],
  filePath?: string,
  indent = ''
): void {
  for (let i = 0; i < inlines.length; i++) {
    lines.push(`${indent}import Inline${i} from '${filePath}?inline=${i}';`);
  }
}

function interleaveHtmlAndInlines(
  lines: string[],
  htmlParts: string[],
  inlines: InlineComponent[],
  formatHtml: (part: string, index: number) => string,
  formatInline: (index: number) => string
): void {
  for (let i = 0; i < htmlParts.length; i++) {
    if (htmlParts[i]) lines.push(formatHtml(htmlParts[i], i));
    if (i < inlines.length) lines.push(formatInline(i));
  }
}
