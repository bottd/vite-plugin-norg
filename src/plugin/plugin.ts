import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { createFilter, type FilterPattern, type HmrContext } from 'vite';
import { z } from 'zod';
import { parseNorgWithFramework, getThemeCss } from '@parser';
import { generateOutput, type GeneratorMode } from './generators';

export type ArboriumConfig =
  | { theme: string; themes?: never }
  | { themes: { light: string; dark: string }; theme?: never };

export interface NorgPluginOptions {
  mode: GeneratorMode;
  include?: FilterPattern;
  exclude?: FilterPattern;
  arboriumConfig?: ArboriumConfig;
}

const optionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react', 'vue']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  arboriumConfig: z
    .union([
      z.object({ theme: z.string() }),
      z.object({ themes: z.object({ light: z.string(), dark: z.string() }) }),
    ])
    .optional(),
});

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

/**
 * Get the framework file extension for inline component module IDs.
 * Framework plugins (vite-plugin-svelte, @vitejs/plugin-vue) identify files
 * to compile by extension, so inline modules need a matching extension.
 */
function frameworkExtension(mode: GeneratorMode): '.svelte' | '.vue' | null {
  switch (mode) {
    case 'svelte':
      return '.svelte';
    case 'vue':
      return '.vue';
    default:
      return null;
  }
}

/**
 * Parse the ?inline=N query parameter from a module ID.
 * Handles format: /path/file.norg.svelte?inline=0 or /path/file.norg.vue?inline=0
 * Tolerates extra Vite query params (e.g., ?inline=0&t=123456)
 */
function parseInlineQuery(id: string): { basePath: string; index: number } | null {
  const match = id.match(/^(.+\.norg)\.(?:svelte|vue)\?inline=(\d+)(?:&|$)/);
  if (!match) return null;
  return {
    basePath: match[1],
    index: parseInt(match[2], 10),
  };
}

function parse(content: string, mode: GeneratorMode) {
  return parseNorgWithFramework(content, mode);
}

export function norgPlugin(options: NorgPluginOptions): import('vite').Plugin {
  const parsed = optionsSchema.safeParse(options);
  if (!parsed.success) {
    throw new Error(`[vite-plugin-norg] Invalid options: ${parsed.error.message}`);
  }
  const { include, exclude, mode, arboriumConfig } = parsed.data;
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig);

  // Cache parsed results to avoid re-parsing for inline component requests
  const parseCache = new Map<string, ReturnType<typeof parse>>();

  return {
    name: 'vite-plugin-norg',
    enforce: 'pre' as const,

    resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_CSS_ID) {
        return RESOLVED_VIRTUAL_CSS_ID;
      }

      // Handle relative inline imports (e.g., './foo.norg?inline=0')
      if (id.includes('.norg?inline=') && importer) {
        const ext = frameworkExtension(mode);
        if (!ext) return;
        const [relativePath, query] = id.split('?');
        const absolutePath = resolve(dirname(importer), relativePath);
        return `${absolutePath}${ext}?${query}`;
      }
    },

    async load(id: string) {
      if (id === RESOLVED_VIRTUAL_CSS_ID) {
        return css;
      }

      // Check for inline component request
      const inlineQuery = parseInlineQuery(id);
      if (inlineQuery) {
        const { basePath, index } = inlineQuery;

        let result = parseCache.get(basePath);
        if (!result) {
          const content = await readFile(basePath, 'utf-8');
          result = parse(content, mode);
          parseCache.set(basePath, result);
        }

        const inline = result.inlines?.[index];
        if (!inline) {
          throw new Error(`Inline component ${index} not found in ${basePath}`);
        }

        return inline.code;
      }

      // Handle main .norg file
      if (!id.endsWith('.norg') || !filter(id)) return;

      const content = await readFile(id, 'utf-8');
      const result = parse(content, mode);

      // Cache for potential inline requests
      parseCache.set(id, result);

      return generateOutput(mode, result, css, id);
    },

    async handleHotUpdate(ctx: HmrContext) {
      // Invalidate cache on file change
      if (ctx.file.endsWith('.norg')) {
        parseCache.delete(ctx.file);
      }

      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      const defaultRead = ctx.read;
      ctx.read = async function () {
        const content = await defaultRead();
        const result = parse(content, mode);
        parseCache.set(ctx.file, result);
        return generateOutput(mode, result, css, ctx.file);
      };

      // Invalidate inline component modules derived from this .norg file
      const ext = frameworkExtension(mode);
      if (ext) {
        const inlinePrefix = ctx.file + ext + '?inline=';
        const inlineModules = Array.from(ctx.server.moduleGraph.idToModuleMap.entries())
          .filter(([id]) => id.startsWith(inlinePrefix))
          .map(([, mod]) => mod);

        for (const mod of inlineModules) {
          ctx.server.moduleGraph.invalidateModule(mod);
        }

        if (inlineModules.length > 0) {
          return [...ctx.modules, ...inlineModules];
        }
      }
    },
  };
}
