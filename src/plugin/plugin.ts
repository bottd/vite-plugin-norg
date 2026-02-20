import { readFile, readdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { createFilter, type FilterPattern, type HmrContext, type Plugin } from 'vite';
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
  componentDir?: string;
}

const optionsSchema = z.object({
  mode: z.enum(['html', 'svelte', 'react', 'vue', 'metadata']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  arboriumConfig: z
    .union([
      z.object({ theme: z.string() }),
      z.object({ themes: z.object({ light: z.string(), dark: z.string() }) }),
    ])
    .optional(),
  componentDir: z.string().optional(),
});

const VIRTUAL_CSS_ID = 'virtual:norg-arborium.css';
const RESOLVED_VIRTUAL_CSS_ID = '\0' + VIRTUAL_CSS_ID;

const VIRTUAL_DOC_CSS_PREFIX = 'virtual:norg-css:';
const RESOLVED_VIRTUAL_DOC_CSS_PREFIX = '\0' + VIRTUAL_DOC_CSS_PREFIX;

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

/**
 * Scan a directory for framework component files and return a name→path map.
 * e.g., Counter.svelte → Map { "Counter" => "/abs/path/Counter.svelte" }
 */
async function scanComponentDir(dir: string, mode: GeneratorMode): Promise<Map<string, string>> {
  const ext = frameworkExtension(mode);
  if (!ext) return new Map();

  const entries = await readdir(dir);
  const components = new Map<string, string>();
  for (const entry of entries) {
    if (entry.endsWith(ext)) {
      const name = basename(entry, ext);
      components.set(name, resolve(dir, entry));
    }
  }
  return components;
}

function injectAfterTag(code: string, pattern: RegExp, content: string): string | null {
  const match = code.match(pattern);
  if (!match || match.index === undefined) return null;
  const pos = match.index + match[0].length;
  return code.slice(0, pos) + '\n' + content + '\n' + code.slice(pos);
}

/**
 * Inject component import statements into inline module code.
 * For Svelte: injects into existing <script> or prepends a new one.
 * For Vue: injects into existing <script setup> or prepends a new one.
 */
function injectComponentImports(
  code: string,
  components: Map<string, string>,
  mode: GeneratorMode
): string {
  if (components.size === 0) return code;

  const imports = Array.from(components.entries())
    .map(([name, path]) => `import ${name} from '${path}';`)
    .join('\n');

  if (mode === 'svelte') {
    return (
      injectAfterTag(code, /<script(?![^>]*\b(?:module|context\s*=))[^>]*>/, imports) ??
      `<script>\n${imports}\n</script>\n${code}`
    );
  }

  if (mode === 'vue') {
    return (
      injectAfterTag(code, /<script\s+setup[^>]*>/, imports) ??
      `<script setup>\n${imports}\n</script>\n${code}`
    );
  }

  return code;
}

function parse(content: string, mode: GeneratorMode) {
  return parseNorgWithFramework(content, mode);
}

export function norgPlugin(options: NorgPluginOptions): import('vite').Plugin {
  const parsed = optionsSchema.safeParse(options);
  if (!parsed.success) {
    throw new Error(`[vite-plugin-norg] Invalid options: ${parsed.error.message}`);
  }
  const { include, exclude, mode, arboriumConfig, componentDir } = parsed.data;
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig);
  const resolvedComponentDir = componentDir ? resolve(componentDir) : undefined;

  const parseCache = new Map<string, ReturnType<typeof parse>>();
  const inlineModuleIds = new Map<string, Set<string>>();
  let components = new Map<string, string>();

  function trackModule(filePath: string, moduleId: string) {
    let ids = inlineModuleIds.get(filePath);
    if (!ids) {
      ids = new Set();
      inlineModuleIds.set(filePath, ids);
    }
    ids.add(moduleId);
  }

  async function cachedParse(filePath: string) {
    let result = parseCache.get(filePath);
    if (!result) {
      const content = await readFile(filePath, 'utf-8');
      result = parse(content, mode);
      parseCache.set(filePath, result);
    }
    return result;
  }

  function invalidateModules(
    ctx: HmrContext,
    moduleIds: Iterable<string>
  ): import('vite').ModuleNode[] {
    const modules: import('vite').ModuleNode[] = [];
    for (const id of moduleIds) {
      const mod = ctx.server.moduleGraph.getModuleById(id);
      if (mod) {
        ctx.server.moduleGraph.invalidateModule(mod);
        modules.push(mod);
      }
    }
    return modules;
  }

  return {
    name: 'vite-plugin-norg',
    enforce: 'pre',

    async buildStart() {
      if (resolvedComponentDir) {
        components = await scanComponentDir(resolvedComponentDir, mode);
      }
    },

    configureServer(server) {
      if (resolvedComponentDir) {
        server.watcher.add(resolvedComponentDir);
      }
    },

    resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_CSS_ID) {
        return RESOLVED_VIRTUAL_CSS_ID;
      }

      if (id.startsWith(VIRTUAL_DOC_CSS_PREFIX)) {
        return '\0' + id;
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

      // Handle per-document CSS virtual module
      if (id.startsWith(RESOLVED_VIRTUAL_DOC_CSS_PREFIX)) {
        const filePath = id.slice(RESOLVED_VIRTUAL_DOC_CSS_PREFIX.length);
        trackModule(filePath, id);
        const result = await cachedParse(filePath);
        return result.inlineCss ?? '';
      }

      // Check for inline component request
      const inlineQuery = parseInlineQuery(id);
      if (inlineQuery) {
        const { basePath, index } = inlineQuery;
        trackModule(basePath, id);
        const result = await cachedParse(basePath);

        const inline = result.inlineComponents?.[index];
        if (!inline) {
          throw new Error(`Inline component ${index} not found in ${basePath}`);
        }

        return injectComponentImports(inline.code, components, mode);
      }

      // Handle ?metadata query on any mode
      const [filepath, query] = id.split('?', 2);
      if (query === 'metadata' && filepath.endsWith('.norg') && filter(filepath)) {
        const result = await cachedParse(filepath);
        return generateOutput('metadata', result, css, filepath);
      }

      // Handle main .norg file
      if (!id.endsWith('.norg') || !filter(id)) return;

      const content = await readFile(id, 'utf-8');
      const result = parse(content, mode);
      parseCache.set(id, result);

      return generateOutput(mode, result, css, id);
    },

    async handleHotUpdate(ctx: HmrContext) {
      // If a file in componentDir changed, re-scan and invalidate all inline modules
      if (resolvedComponentDir && ctx.file.startsWith(resolvedComponentDir + '/')) {
        components = await scanComponentDir(resolvedComponentDir, mode);

        const allIds = [...inlineModuleIds.values()].flatMap(ids => [...ids]);
        const invalidated = invalidateModules(ctx, allIds);
        if (invalidated.length > 0) {
          return [...ctx.modules, ...invalidated];
        }
        return;
      }

      // Invalidate cache on file change
      if (ctx.file.endsWith('.norg')) {
        parseCache.delete(ctx.file);
      }

      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      // Invalidate inline component modules derived from this .norg file
      const trackedIds = inlineModuleIds.get(ctx.file);
      if (trackedIds) {
        const invalidated = invalidateModules(ctx, trackedIds);
        if (invalidated.length > 0) {
          return [...ctx.modules, ...invalidated];
        }
      }
    },
  } satisfies Plugin;
}
