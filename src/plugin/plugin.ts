import { createFilter, type FilterPattern, type Plugin } from 'vite';
import { z } from 'zod';
import { parseNorg, getThemeCss } from '@parser';
import type { NorgParseResult } from '@parser';
import { generateHtmlOutput } from './generators/html';
import { generateSvelteOutput } from './generators/svelte';
import { generateReactOutput } from './generators/react';
import { generateMetadataOutput } from './generators/metadata';

export type ArboriumConfig =
  | { theme: string; themes?: never }
  | { themes: { light: string; dark: string }; theme?: never };

const NorgPluginOptionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react', 'metadata']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  arboriumConfig: z.any().optional(),
});

export interface NorgPluginOptions {
  mode: 'html' | 'svelte' | 'react' | 'metadata';
  include?: FilterPattern;
  exclude?: FilterPattern;
  arboriumConfig?: ArboriumConfig;
}

export type NorgGenerator = (result: NorgParseResult, css: string) => string;
const generators = {
  html: generateHtmlOutput,
  svelte: generateSvelteOutput,
  react: generateReactOutput,
  metadata: generateMetadataOutput,
} as const satisfies Record<NorgPluginOptions['mode'], NorgGenerator>;

const VIRTUAL_CSS_ID = 'virtual:norg-arborium.css';
const RESOLVED_VIRTUAL_CSS_ID = '\0' + VIRTUAL_CSS_ID;

function buildCss(config?: ArboriumConfig): string {
  if (!config) return '';

  if (config.theme) {
    return getThemeCss(config.theme);
  }

  if (config.themes) {
    return `
      @media (prefers-color-scheme: light) {\n${getThemeCss(config.themes.light)}\n}
      @media (prefers-color-scheme: dark) {\n${getThemeCss(config.themes.dark)}\n}
    `;
  }

  return '';
}

export function norgPlugin(options: NorgPluginOptions) {
  const validatedOptions = NorgPluginOptionsSchema.parse(options);
  const { include, exclude, mode, arboriumConfig } = validatedOptions;
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig as ArboriumConfig);

  return {
    name: 'vite-plugin-norg',
    enforce: 'pre',

    resolveId(id) {
      if (id === VIRTUAL_CSS_ID) {
        return RESOLVED_VIRTUAL_CSS_ID;
      }
    },

    async load(id) {
      if (id === RESOLVED_VIRTUAL_CSS_ID) {
        return css;
      }

      const [filepath, query = ''] = id.split('?', 2);
      if (!filepath.endsWith('.norg') || !filter(filepath)) return;

      try {
        const { readFile } = await import('node:fs/promises');

        const content = await readFile(filepath, 'utf-8');
        const generator = query === 'metadata' ? generators.metadata : generators[mode];
        const result = parseNorg(content);
        return generator(result, css);
      } catch (error) {
        this.error(new Error(`Failed to parse norg file ${filepath}: ${error}`));
      }
    },

    async handleHotUpdate(ctx) {
      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      const defaultRead = ctx.read;
      ctx.read = async function () {
        try {
          const content = await defaultRead();
          const result = parseNorg(content);
          return generators[mode](result, css);
        } catch (error) {
          throw new Error(`Failed to parse norg file ${ctx.file}: ${error}`);
        }
      };
    },
  } satisfies Plugin;
}
