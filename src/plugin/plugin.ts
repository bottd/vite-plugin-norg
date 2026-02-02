import { resolve, dirname } from 'node:path';
import { createFilter, type FilterPattern, type HmrContext } from 'vite';
import { z } from 'zod';
import { parseNorg, parseNorgWithFramework, getThemeCss } from '@parser';
import type { NorgParseResult } from '@parser';
import { generateOutput, type GeneratorMode } from './generators';

export type ArboriumConfig =
  | { theme: string; themes?: never }
  | { themes: { light: string; dark: string }; theme?: never };

const NorgPluginOptionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react', 'vue']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  arboriumConfig: z.any().optional(),
});

export interface NorgPluginOptions {
  mode: GeneratorMode;
  include?: FilterPattern;
  exclude?: FilterPattern;
  arboriumConfig?: ArboriumConfig;
}

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
    case 'svelte': return '.svelte';
    case 'vue': return '.vue';
    default: return null;
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

export function norgPlugin(options: NorgPluginOptions): import('vite').Plugin {
  const validatedOptions = NorgPluginOptionsSchema.parse(options);
  const { include, exclude, mode, arboriumConfig } = validatedOptions;
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig as ArboriumConfig);

  // For framework modes, pass framework to parser for @inline support
  const framework = mode === 'html' ? null : mode;

  // Cache parsed results to avoid re-parsing for inline component requests
  const parseCache = new Map<string, NorgParseResult>();

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
        if (!ext) return; // html/react modes don't support inline
        // Remove the query part to get the relative path
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

        try {
          // Check cache first
          let result = parseCache.get(basePath);
          if (!result) {
            const { readFile } = await import('node:fs/promises');
            const content = await readFile(basePath, 'utf-8');
            result = framework ? parseNorgWithFramework(content, framework) : parseNorg(content);
            parseCache.set(basePath, result);
          }

          const inline = result.inlines?.[index];
          if (!inline) {
            throw new Error(`Inline component ${index} not found in ${basePath}`);
          }

          // Return the raw component code - Vite will process it based on the extension
          return inline.code;
        } catch (error) {
          throw new Error(`Failed to load inline component ${index} from ${basePath}: ${error}`);
        }
      }

      // Handle main .norg file
      if (!id.endsWith('.norg') || !filter(id)) return;

      try {
        const { readFile } = await import('node:fs/promises');

        const content = await readFile(id, 'utf-8');
        const result = framework ? parseNorgWithFramework(content, framework) : parseNorg(content);

        // Cache for potential inline requests
        parseCache.set(id, result);

        return generateOutput(mode, result, css, id);
      } catch (error) {
        throw new Error(`Failed to parse norg file ${id}: ${error}`);
      }
    },

    async handleHotUpdate(ctx: HmrContext) {
      // Invalidate cache on file change
      if (ctx.file.endsWith('.norg')) {
        parseCache.delete(ctx.file);
      }

      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      const defaultRead = ctx.read;
      ctx.read = async function () {
        try {
          const content = await defaultRead();
          const result = framework
            ? parseNorgWithFramework(content, framework)
            : parseNorg(content);

          // Update cache
          parseCache.set(ctx.file, result);

          return generateOutput(mode, result, css, ctx.file);
        } catch (error) {
          throw new Error(`Failed to parse norg file ${ctx.file}: ${error}`);
        }
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
