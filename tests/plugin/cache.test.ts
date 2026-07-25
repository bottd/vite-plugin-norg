const readFile = vi.hoisted(() => vi.fn());

vi.mock('node:fs/promises', async importOriginal => ({
  ...(await importOriginal<typeof import('node:fs/promises')>()),
  readFile,
}));

import { norgPlugin } from '../../src/plugin/index.js';

beforeEach(() => readFile.mockReset());

it('retries an in-flight parse invalidated by HMR', async () => {
  let resolveStale!: (content: string) => void;
  readFile
    .mockImplementationOnce(() => new Promise<string>(resolve => (resolveStale = resolve)))
    .mockResolvedValueOnce('* Fresh\n');

  const file = '/tmp/cache-race.norg';
  const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
  const context = {
    warn: vi.fn(),
    error(message: string): never {
      throw new Error(message);
    },
  };
  const load = plugin.load as (
    this: typeof context,
    id: string
  ) => Promise<{ code: string } | undefined>;
  const staleLoad = load.call(context, file);

  const hotUpdate = plugin.handleHotUpdate as (ctx: {
    file: string;
    modules: never[];
  }) => Promise<unknown>;
  await hotUpdate({ file, modules: [] });
  resolveStale('* Stale\n');

  const result = await staleLoad;
  expect(result?.code).toContain('Fresh');
  expect(result?.code).not.toContain('Stale');
  expect(readFile).toHaveBeenCalledTimes(2);
});

it('retries an in-flight read failure invalidated by HMR', async () => {
  let rejectStale!: (error: Error) => void;
  readFile
    .mockImplementationOnce(() => new Promise<string>((_, reject) => (rejectStale = reject)))
    .mockResolvedValueOnce('* Fresh\n');

  const file = '/tmp/cache-race.norg';
  const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
  const context = {
    warn: vi.fn(),
    error(message: string): never {
      throw new Error(message);
    },
  };
  const load = plugin.load as (
    this: typeof context,
    id: string
  ) => Promise<{ code: string } | undefined>;
  const staleLoad = load.call(context, file);

  const hotUpdate = plugin.handleHotUpdate as (ctx: {
    file: string;
    modules: never[];
  }) => Promise<unknown>;
  await hotUpdate({ file, modules: [] });
  rejectStale(new Error('stale read'));

  const result = await staleLoad;
  expect(result?.code).toContain('Fresh');
  expect(readFile).toHaveBeenCalledTimes(2);
});
