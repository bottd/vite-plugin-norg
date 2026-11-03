import { readFile, readdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import {
  createFilter,
  transformWithOxc,
  type FilterPattern,
  type HmrContext,
  type ModuleNode,
  type Plugin,
} from 'vite';
import { parseNorg, getThemeCss, type NorgParseResult } from '@parser';
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

const VIRTUAL_CSS_ID = 'virtual:norg-arborium.css';
const RESOLVED_VIRTUAL_CSS_ID = `\0${VIRTUAL_CSS_ID}`;

const VIRTUAL_DOC_CSS_PREFIX = 'virtual:norg-css:';
const RESOLVED_VIRTUAL_DOC_CSS_PREFIX = `\0${VIRTUAL_DOC_CSS_PREFIX}`;

const modeExtensions: Record<GeneratorMode, string | null> = {
  html: null,
  svelte: '.svelte',
  vue: '.vue',
  react: '.jsx',
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

  if (mode === 'svelte') {
    return (
      injectAfterTag(code, /<script(?![^>]*\b(?:module|context\s*=))[^>]*>/, imports) ??
      `<script>\n${imports}\n</script>\n${code}`
    );
  }

  if (mode === 'vue') {
    const wrapped = /<template[\s>]/.test(code) ? code : `<template>\n${code}\n</template>`;
    return (
      injectAfterTag(wrapped, /<script\s+setup[^>]*>/, imports) ??
      `<script setup>\n${imports}\n</script>\n${wrapped}`
    );
  }

  if (mode === 'react') {
    return imports + '\n' + code;
  }

  return code;
}

export function norgPlugin(options: NorgPluginOptions): Plugin {
  const {
    include,
    exclude,
    mode,
    arboriumConfig,
    componentDir,
    components: explicitComponents,
  } = options;

  // The one option that fails silently — an unknown mode matches no generator
  // branch and yields an undefined module. Others fail where they're used.
  if (!(mode in modeExtensions)) {
    throw new Error(
      `[vite-plugin-norg] Invalid mode ${JSON.stringify(mode)}. ` +
        `Expected one of: ${Object.keys(modeExtensions).join(', ')}.`
    );
  }
  const filter = createFilter(include, exclude);
  const css = buildCss(arboriumConfig);
  const resolvedComponentDir = componentDir ? resolve(componentDir) : undefined;
  const ext = modeExtensions[mode];
  const norgWithExt = ext ? `.norg${ext}` : null;
  const parseMode = mode === 'metadata' ? null : mode;

  const parseCache = new Map<string, Promise<NorgParseResult>>();
  const embedModuleIds = new Map<string, Set<string>>();
  const embedModules = new Map<string, { basePath: string; index: number }>();
  let components = new Map<string, string>();

  function trackModule(filePath: string, moduleId: string) {
    const ids = embedModuleIds.get(filePath) ?? new Set<string>();
    ids.add(moduleId);
    embedModuleIds.set(filePath, ids);
  }

  function cachedParse(
    filePath: string,
    warn: (message: string) => void
  ): Promise<NorgParseResult> {
    let pending = parseCache.get(filePath);
    if (!pending) {
      const fresh = readFile(filePath, 'utf-8')
        .then(content => parseNorg(content, parseMode))
        .then(result => {
          if (parseCache.get(filePath) !== fresh) return cachedParse(filePath, warn);
          result.diagnostics.forEach(warn);
          return result;
        })
        .catch((error: unknown) => {
          if (parseCache.get(filePath) !== fresh) return cachedParse(filePath, warn);
          throw error;
        });
      pending = fresh;
      parseCache.set(filePath, fresh);
      void fresh.catch(() => {
        if (parseCache.get(filePath) === fresh) parseCache.delete(filePath);
      });
    }
    return pending;
  }

  function invalidateModules(ctx: HmrContext, moduleIds: Iterable<string>): ModuleNode[] {
    const modules: ModuleNode[] = [];
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

    async resolveId(id: string, importer?: string) {
      if (id === VIRTUAL_CSS_ID) {
        return RESOLVED_VIRTUAL_CSS_ID;
      }

      if (id.startsWith(VIRTUAL_DOC_CSS_PREFIX)) {
        return `\0${id}.css`;
      }

      if (ext && id.endsWith('.norg') && importer) {
        // Let Vite resolve root-relative ("/src/x.norg"), aliased and package ids.
        const resolved = await this.resolve(id, importer, { skipSelf: true });
        const basePath = resolved?.id ?? resolve(dirname(importer), id);
        if (filter(basePath)) {
          return `\0${basePath}${ext}`;
        }
      }

      if (ext && id.includes('.norg?embed=') && importer) {
        const [relativePath, query] = id.split('?', 2);
        const basePath = resolve(dirname(importer), relativePath);
        const index = parseInt(new URLSearchParams(query).get('embed') ?? '', 10);
        if (Number.isNaN(index)) return;
        const resolvedId = `\0${basePath}${ext}?${query}`;
        embedModules.set(resolvedId, { basePath, index });
        return resolvedId;
      }
    },

    async load(id: string) {
      const parse = (filePath: string) =>
        cachedParse(filePath, message => {
          this.warn({ id: filePath, message });
        });

      if (id === RESOLVED_VIRTUAL_CSS_ID) {
        return css;
      }

      if (id.startsWith(RESOLVED_VIRTUAL_DOC_CSS_PREFIX) && id.endsWith('.css')) {
        const filePath = id.slice(RESOLVED_VIRTUAL_DOC_CSS_PREFIX.length, -4);
        trackModule(filePath, id);
        const result = await parse(filePath);
        return result.embedCss ?? '';
      }

      const embedInfo = embedModules.get(id);
      if (embedInfo) {
        const { basePath, index } = embedInfo;
        trackModule(basePath, id);
        const result = await parse(basePath);

        const embed = result.embedComponents?.[index];
        if (!embed) {
          throw new Error(`Embed component ${index} not found in ${basePath}`);
        }

        let code = embed.code;
        if (mode === 'react') {
          code = `export default function NorgEmbed() { return <>${code}</>; }`;
        }

        return injectComponentImports(code, components, mode);
      }

      const rawId = id.startsWith('\0') ? id.slice(1) : id;
      const [idWithoutQuery, query] = rawId.split('?', 2);
      if (query && query !== 'metadata') return;
      let basePath = idWithoutQuery;
      if (ext && norgWithExt && idWithoutQuery.endsWith(norgWithExt)) {
        basePath = idWithoutQuery.slice(0, -ext.length);
      }
      if (!basePath.endsWith('.norg') || !filter(basePath)) return;

      const outputMode: GeneratorMode = query === 'metadata' ? 'metadata' : mode;
      // Virtual ids aren't mapped to their source file, so HMR must find them here.
      if (id.startsWith('\0')) trackModule(basePath, id);

      try {
        const result = await parse(basePath);
        const code = generateOutput(outputMode, result, css, basePath);
        return {
          code,
          map: { version: 3, sources: [basePath], names: [], mappings: '' },
        };
      } catch (error) {
        this.error(`Failed to parse norg file ${basePath}: ${String(error)}`);
      }
    },

    async transform(code, id) {
      if (mode !== 'react') return;
      if (!ext || !norgWithExt || !id.startsWith('\0') || !id.includes(norgWithExt)) return;
      return transformWithOxc(code, id, {
        lang: ext.slice(1) as 'jsx',
        jsx: { runtime: 'automatic' },
      });
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

      if (!ctx.file.endsWith('.norg')) return;
      parseCache.delete(ctx.file);

      if (!filter(ctx.file)) return;

      ctx.modules.forEach(mod => {
        ctx.server.moduleGraph.invalidateModule(mod);
      });

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
