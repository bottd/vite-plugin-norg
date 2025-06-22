import { createFilter, type Plugin, FilterPattern } from 'vite';
import { z } from 'zod';
import { getWasmParser, NorgParser, NorgParseResult } from './wasm';
import { generateHtmlOutput } from './generators/html';
import { generateSvelteOutput } from './generators/svelte';
import { generateReactOutput } from './generators/react';

const NorgPluginOptionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react']),
  include: z.any().optional(),
  exclude: z.any().optional(),
});

export interface NorgPluginOptions {
  mode: 'html' | 'svelte' | 'react';
  include?: FilterPattern;
  exclude?: FilterPattern;
}

export type NorgGenerator = (result: NorgParseResult) => string;
const generators = {
  html: generateHtmlOutput,
  svelte: generateSvelteOutput,
  react: generateReactOutput,
} as const satisfies Record<NorgPluginOptions['mode'], NorgGenerator>;

export function norgPlugin(options: NorgPluginOptions) {
  const validatedOptions = NorgPluginOptionsSchema.parse(options);
  const { include, exclude, mode } = validatedOptions;
  const filter = createFilter(include, exclude);

  let parser: NorgParser | null = null;

  const getParser = async () => {
    if (!parser) {
      try {
        parser = await getWasmParser();
      } catch (error) {
        throw new Error(`Failed to load norg parser: ${error}`);
      }
    }
    return parser;
  };

  return {
    name: 'vite-plugin-norg',
    enforce: 'pre',

    async resolveId(id, importer) {
      if (!id.endsWith('.norg')) return null;

      const resolved = await this.resolve(id, importer, { skipSelf: true });
      if (resolved) {
        return resolved.id;
      }

      return null;
    },

    async load(id) {
      if (!filter(id)) return;

      try {
        const { readFile } = await import('node:fs/promises');
        const currentParser = await getParser();

        const content = await readFile(id, 'utf-8');
        const result = currentParser(content);
        return generators[mode](result);
      } catch (error) {
        this.error(new Error(`Failed to parse norg file ${id}: ${error}`));
      }
    },

    async handleHotUpdate(ctx) {
      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      const defaultRead = ctx.read;
      ctx.read = async function () {
        try {
          const content = await defaultRead();
          const currentParser = await getParser();
          return generators[mode](currentParser(content));
        } catch (error) {
          throw new Error(`Failed to parse norg file ${ctx.file}: ${error}`);
        }
      };
    },
  } satisfies Plugin;
}
