import { createFilter, type Plugin, FilterPattern } from 'vite';
import { z } from 'zod';
import { getWasmParser, NorgParser, NorgParseResult } from './wasm';
import { generateHtmlOutput } from './generators/html';
import { generateSvelteOutput } from './generators/svelte';
import { generateReactOutput } from './generators/react';
import { codeToHtml, BundledHighlighterOptions, BundledLanguage, BundledTheme } from 'shiki';

const NorgPluginOptionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  shikiOptions: z.any().optional(),
});

export interface NorgPluginOptions {
  mode: 'html' | 'svelte' | 'react';
  include?: FilterPattern;
  exclude?: FilterPattern;
  shikiOptions?: BundledHighlighterOptions<BundledLanguage, BundledTheme>;
}

export type NorgGenerator = (result: NorgParseResult) => string;
const generators = {
  html: generateHtmlOutput,
  svelte: generateSvelteOutput,
  react: generateReactOutput,
} as const satisfies Record<NorgPluginOptions['mode'], NorgGenerator>;

export function norgPlugin(options: NorgPluginOptions) {
  const validatedOptions = NorgPluginOptionsSchema.parse(options);
  const { include, exclude, mode, shikiOptions } = validatedOptions;
  const filter = createFilter(include, exclude);

  const highlightOptions = {
    themes: { light: 'github-light', dark: 'github-dark' },
    ...shikiOptions,
  };

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

  const decodeHtmlEntities = (html: string): string =>
    html
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&amp;/g, '&')
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
      .replace(/&#x27;/g, "'")
      .replace(/&#x2F;/g, '/')
      .replace(/&#x60;/g, '`')
      .replace(/&#(\d+);/g, (match, dec) => String.fromCharCode(dec))
      .replace(/&#x([0-9a-fA-F]+);/g, (match, hex) => String.fromCharCode(parseInt(hex, 16)));

  const highlightCodeBlocks = async (html: string): Promise<string> => {
    const codeBlockRegex =
      /<pre class="lang-(\w+)"><code class="lang-\w+">([\s\S]*?)<\/code><\/pre>/g;

    const matches = html.matchAll(codeBlockRegex);

    let result = html;
    for (const [fullMatch, lang, code] of matches) {
      const decodedCode = decodeHtmlEntities(code);

      try {
        const highlighted = await codeToHtml(decodedCode, {
          ...highlightOptions,
          lang: lang.toLowerCase(),
        });
        result = result.replace(fullMatch, highlighted);
      } catch {
        // Fallback to plaintext if language is not supported
        const highlighted = await codeToHtml(decodedCode, {
          ...highlightOptions,
          lang: 'plaintext',
        });
        result = result.replace(fullMatch, highlighted);
      }
    }

    return result;
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
        const parser = await getParser();

        const content = await readFile(id, 'utf-8');
        const result = parser(content);

        // highlightCodeBlocks adds syntax highlighting to code embedded in documents
        const processedHtml = decodeHtmlEntities(await highlightCodeBlocks(result.html));
        return generators[mode]({ ...result, html: processedHtml });
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
          const parser = await getParser();
          const result = parser(content);

          // highlightCodeBlocks adds syntax highlighting to code embedded in documents
          const processedHtml = decodeHtmlEntities(await highlightCodeBlocks(result.html));
          return generators[mode]({ ...result, html: processedHtml });
        } catch (error) {
          throw new Error(`Failed to parse norg file ${ctx.file}: ${error}`);
        }
      };
    },
  } satisfies Plugin;
}
