import type { NorgParseResult, OutputMode } from '@parser';
import { generateHtml } from './html';
import { generateSvelte } from './svelte';
import { generateReact } from './react';
import { generateVue } from './vue';
import { generateMetadata } from './metadata';

export type GeneratorMode = `${OutputMode}` | 'metadata';

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
      return generateReact(result, css, filePath);
    case 'vue':
      return generateVue(result, css, filePath);
    case 'metadata':
      return generateMetadata(result);
  }
}
