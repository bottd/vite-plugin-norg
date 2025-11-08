import { createFilter, type Plugin, FilterPattern } from 'vite';
import { z } from 'zod';
import { getWasmParser, type NorgParser, type NorgParseResult } from './napi';
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

  const parser = getWasmParser;

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
      /<pre(?:\s+class="lang-(\w+)")?><code(?:\s+class="lang-\w+")?>([^]*?)<\/code><\/pre>/g;

    const matches = html.matchAll(codeBlockRegex);

    let result = html;
    for (const [fullMatch, lang, code] of matches) {
      const decodedCode = decodeHtmlEntities(code);
      const language = lang ? lang.toLowerCase() : 'text';

      try {
        const highlighted = await codeToHtml(decodedCode, {
          ...highlightOptions,
          lang: language,
        });
        result = result.replace(fullMatch, highlighted);
      } catch (error) {
        // Fallback to text if language is not supported
        try {
          const highlighted = await codeToHtml(decodedCode, {
            ...highlightOptions,
            lang: 'text',
          });
          result = result.replace(fullMatch, highlighted);
        } catch (fallbackError) {
          console.warn(`Failed to highlight code block with language "${language}":`, error);
          console.warn(`Fallback to 'text' also failed:`, fallbackError);
        }
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
