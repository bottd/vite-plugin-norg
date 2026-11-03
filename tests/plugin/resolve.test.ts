import { join } from 'node:path';
import { createServer, type HmrContext, type ModuleNode } from 'vite';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir } from './fixtures';

it('resolves root-relative imports in framework modes', async () => {
  const server = await createServer({
    root: fixturesDir,
    configFile: false,
    logLevel: 'silent',
    server: { middlewareMode: true, hmr: false, watch: null },
    plugins: [norgPlugin({ mode: 'react' })],
  });
  try {
    const mod = await server.ssrLoadModule('/root-relative.js');
    expect(mod.toc.length).toBeGreaterThan(0);
  } finally {
    await server.close();
  }
});

it('hot-updates the virtual main module when its .norg file changes', async () => {
  const file = join(fixturesDir, 'basic.norg');
  const virtualId = `\0${file}.jsx`;
  const plugin = norgPlugin({ mode: 'react' });
  const load = plugin.load as (this: { warn: () => void }, id: string) => Promise<unknown>;
  await load.call({ warn: vi.fn() }, virtualId);

  const mod = { id: virtualId } as ModuleNode;
  const invalidateModule = vi.fn();
  const hotUpdate = plugin.handleHotUpdate as (ctx: HmrContext) => Promise<ModuleNode[]>;
  const modules = await hotUpdate({
    file,
    modules: [],
    server: {
      moduleGraph: {
        getModuleById: (id: string) => (id === virtualId ? mod : undefined),
        invalidateModule,
      },
    },
  } as unknown as HmrContext);

  expect(modules).toEqual([mod]);
  expect(invalidateModule).toHaveBeenCalledWith(mod);
});
