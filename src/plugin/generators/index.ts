import type { NorgParseResult, InlineComponent } from '@parser';

export type GeneratorMode = 'html' | 'svelte' | 'react' | 'vue' | 'metadata';

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
      return generateHtml(result, css, filePath);
    case 'svelte':
      return generateSvelte(result, css, filePath);
    case 'react':
      return generateReact(result, css, filePath);
    case 'vue':
      return generateVue(result, css, filePath);
    case 'metadata':
      return generateMetadata(result);
  }
}

function generateHtml(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const html = embedInlines(htmlParts, inlineComponents);
  const lines: string[] = [];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  if (inlineCss && filePath) lines.push(`import 'virtual:norg-css:${filePath}';`);
  lines.push(
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export const html = ${JSON.stringify(html)};`,
    `export const toc = ${JSON.stringify(toc ?? [])};`,
    `export default { metadata, html, toc };`
  );
  return lines.join('\n');
}

function generateSvelte(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
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
  if (inlineCss && filePath) scriptContent.push(`  import 'virtual:norg-css:${filePath}';`);
  addInlineImports(scriptContent, inlineComponents, filePath, '  ');

  if (scriptContent.length > 0) {
    lines.push('<script lang="ts">', ...scriptContent, '</script>');
  }

  // Interleave HTML parts with inline components
  interleaveHtmlAndInlines(
    lines,
    htmlParts,
    inlineComponents,
    part => `{@html ${JSON.stringify(part)}}`,
    i => `<Inline${i} />`
  );

  return lines.join('\n');
}

function generateReact(
  { htmlParts, metadata, toc, inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const html = htmlParts.join('');
  const lines: string[] = ['import React from "react";'];
  if (css) lines.push('import "virtual:norg-arborium.css";');
  if (inlineCss && filePath) lines.push(`import 'virtual:norg-css:${filePath}';`);

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
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
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
  if (inlineCss && filePath) lines.push(`import 'virtual:norg-css:${filePath}';`);
  addInlineImports(lines, inlineComponents, filePath);

  if (inlineComponents.length > 0) {
    lines.push(`const htmlParts = ${JSON.stringify(htmlParts)};`);
  } else {
    lines.push(`const htmlContent = ${JSON.stringify(htmlParts.join(''))};`);
  }

  lines.push('', 'defineExpose({ metadata, toc });');
  lines.push('</script>', '', '<template>');

  if (inlineComponents.length === 0) {
    lines.push('  <div v-html="htmlContent"></div>');
  } else {
    lines.push('  <div>');
    interleaveHtmlAndInlines(
      lines,
      htmlParts,
      inlineComponents,
      (_part, i) => `    <div v-html="htmlParts[${i}]"></div>`,
      i => `    <Inline${i} />`
    );
    lines.push('  </div>');
  }

  lines.push('</template>');
  return lines.join('\n');
}

function generateMetadata({ metadata }: NorgParseResult): string {
  return [
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export default { metadata };`,
  ].join('\n');
}

function addInlineImports(
  lines: string[],
  inlineComponents: InlineComponent[],
  filePath?: string,
  indent = ''
): void {
  for (let i = 0; i < inlineComponents.length; i++) {
    lines.push(`${indent}import Inline${i} from '${filePath}?inline=${i}';`);
  }
}

function embedInlines(htmlParts: string[], inlineComponents: InlineComponent[]): string {
  const parts: string[] = [];
  for (let i = 0; i < htmlParts.length; i++) {
    parts.push(htmlParts[i]);
    if (i < inlineComponents.length) parts.push(inlineComponents[i].code);
  }
  return parts.join('');
}

function interleaveHtmlAndInlines(
  lines: string[],
  htmlParts: string[],
  inlineComponents: InlineComponent[],
  formatHtml: (part: string, index: number) => string,
  formatInline: (index: number) => string
): void {
  for (let i = 0; i < htmlParts.length; i++) {
    if (htmlParts[i]) lines.push(formatHtml(htmlParts[i], i));
    if (i < inlineComponents.length) lines.push(formatInline(i));
  }
}
