import { readFile, readdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { createFilter, type FilterPattern, type HmrContext, type Plugin } from 'vite';
import { z } from 'zod';
import { parseNorg, getThemeCss, OutputMode } from '@parser';
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
  components?: Record<string, string>;
}

const optionsSchema = z.object({
  mode: z.enum([OutputMode.html, OutputMode.svelte, OutputMode.react, OutputMode.vue, 'metadata']),
  include: z.any().optional(),
  exclude: z.any().optional(),
  arboriumConfig: z
    .union([
      z.object({ theme: z.string() }),
      z.object({ themes: z.object({ light: z.string(), dark: z.string() }) }),
    ])
    .optional(),
  componentDir: z.string().optional(),
  components: z.record(z.string(), z.string()).optional(),
});

const VIRTUAL_CSS_ID = 'virtual:norg-arborium.css';
const RESOLVED_VIRTUAL_CSS_ID = '\0' + VIRTUAL_CSS_ID;

const VIRTUAL_DOC_CSS_PREFIX = 'virtual:norg-css:';
const RESOLVED_VIRTUAL_DOC_CSS_PREFIX = '\0' + VIRTUAL_DOC_CSS_PREFIX;

const modeExtensions: Record<GeneratorMode, string | null> = {
  [OutputMode.html]: null,
  [OutputMode.svelte]: '.svelte',
  [OutputMode.vue]: '.vue',
  [OutputMode.react]: '.jsx',
  metadata: null,
};

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

async function scanComponentDir(dir: string, mode: GeneratorMode): Promise<Map<string, string>> {
  const ext = modeExtensions[mode];
  if (!ext) return new Map();

  const entries = await readdir(dir).catch(() => [] as string[]);
  const components = new Map<string, string>();
  entries
    .filter(entry => entry.endsWith(ext))
    .forEach(entry => components.set(basename(entry, ext), resolve(dir, entry)));
  return components;
}

function injectAfterTag(code: string, pattern: RegExp, content: string): string | null {
  const match = code.match(pattern);
  if (!match || match.index === undefined) return null;
  const pos = match.index + match[0].length;
  return code.slice(0, pos) + '\n' + content + '\n' + code.slice(pos);
}

function injectComponentImports(
  code: string,
  components: Map<string, string>,
  mode: GeneratorMode
): string {
  if (components.size === 0) return code;

  const imports = [...components]
    .map(([name, path]) => `import ${name} from '${path}';`)
    .join('\n');

  if (mode === OutputMode.svelte) {
    return (
      injectAfterTag(code, /<script(?![^>]*\b(?:module|context\s*=))[^>]*>/, imports) ??
      `<script>\n${imports}\n</script>\n${code}`
    );
  }

  if (mode === OutputMode.vue) {
    const wrapped = /<template[\s>]/.test(code) ? code : `<template>\n${code}\n</template>`;
    return (
      injectAfterTag(wrapped, /<script\s+setup[^>]*>/, imports) ??
      `<script setup>\n${imports}\n</script>\n${wrapped}`
    );
  }

  if (mode === OutputMode.react) {
    return imports + '\n' + code;
  }

  return code;
}

export function norgPlugin(options: NorgPluginOptions): import('vite').Plugin {
  const parsed = optionsSchema.safeParse(options);
  if (!parsed.success) {
    throw new Error(`[vite-plugin-norg] Invalid options: ${parsed.error.message}`);
  }
  const {
    include,
    exclude,
    mode,
    arboriumConfig,
    componentDir,
    components: explicitComponents,
  } = parsed.data;
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig);
  const resolvedComponentDir = componentDir ? resolve(componentDir) : undefined;

  const parseCache = new Map<string, ReturnType<typeof parseNorg>>();
  const embedModuleIds = new Map<string, Set<string>>();
  const embedModules = new Map<string, { basePath: string; index: number }>();
  let components = new Map<string, string>();

  function trackModule(filePath: string, moduleId: string) {
    const ids = embedModuleIds.get(filePath) ?? new Set<string>();
    ids.add(moduleId);
    embedModuleIds.set(filePath, ids);
  }

  async function cachedParse(filePath: string) {
    let result = parseCache.get(filePath);
    if (!result) {
      const content = await readFile(filePath, 'utf-8');
      result = parseNorg(content, mode);
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
      Object.entries(explicitComponents ?? {}).forEach(([name, importPath]) =>
        components.set(name, resolve(importPath))
      );
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
        return '\0' + id + '.css';
      }

      if (id.includes('.norg?embed=') && importer) {
        const ext = modeExtensions[mode];
        if (!ext) return;
        const [relativePath, query] = id.split('?');
        const basePath = resolve(dirname(importer), relativePath);
        const index = parseInt(new URLSearchParams(query).get('embed') ?? '', 10);
        if (Number.isNaN(index)) return;
        const resolvedId = `${basePath}${ext}?${query}`;
        embedModules.set(resolvedId, { basePath, index });
        return resolvedId;
      }
    },

    async load(id: string) {
      if (id === RESOLVED_VIRTUAL_CSS_ID) {
        return css;
      }

      if (id.startsWith(RESOLVED_VIRTUAL_DOC_CSS_PREFIX) && id.endsWith('.css')) {
        const filePath = id.slice(RESOLVED_VIRTUAL_DOC_CSS_PREFIX.length, -4);
        trackModule(filePath, id);
        const result = await cachedParse(filePath);
        return result.embedCss ?? '';
      }

      const embedInfo = embedModules.get(id);
      if (embedInfo) {
        const { basePath, index } = embedInfo;
        trackModule(basePath, id);
        const result = await cachedParse(basePath);

        const embed = result.embedComponents?.[index];
        if (!embed) {
          throw new Error(`Embed component ${index} not found in ${basePath}`);
        }

        let code = embed.code;
        if (mode === OutputMode.react) {
          code = `export default function NorgEmbed() { return <>${code}</>; }`;
        }

        return injectComponentImports(code, components, mode);
      }

      const [basePath, query] = id.split('?', 2);
      if (!basePath.endsWith('.norg') || !filter(basePath)) return;

      const outputMode: GeneratorMode = query === 'metadata' ? 'metadata' : mode;

      try {
        const result = await cachedParse(basePath);
        return generateOutput(outputMode, result, css, basePath);
      } catch (error) {
        this.error(`Failed to parse norg file ${basePath}: ${error}`);
      }
    },

    async handleHotUpdate(ctx: HmrContext) {
      if (resolvedComponentDir && ctx.file.startsWith(resolvedComponentDir + '/')) {
        components = await scanComponentDir(resolvedComponentDir, mode);

        const allIds = [...embedModuleIds.values()].flatMap(ids => [...ids]);
        const invalidated = invalidateModules(ctx, allIds);
        if (invalidated.length > 0) {
          return [...ctx.modules, ...invalidated];
        }
        return;
      }

      if (ctx.file.endsWith('.norg')) {
        parseCache.delete(ctx.file);
      }

      if (!filter(ctx.file) || !ctx.file.endsWith('.norg')) return;

      ctx.modules.forEach(mod => ctx.server.moduleGraph.invalidateModule(mod));

      const trackedIds = embedModuleIds.get(ctx.file);
      if (trackedIds) {
        const invalidated = invalidateModules(ctx, trackedIds);
        if (invalidated.length > 0) {
          return [...ctx.modules, ...invalidated];
        }
      }

      return ctx.modules;
    },
  } satisfies Plugin;
}
