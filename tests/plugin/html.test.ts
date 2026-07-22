import { join } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, fixtures, loadCode } from './fixtures';

describe('HTML Generator', () => {
  const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });

  it('has correct plugin name', () => {
    expect(plugin.name).toBe('vite-plugin-norg');
  });

  it('enforces pre', () => {
    expect(plugin.enforce).toBe('pre');
  });

  it('ignores non-norg files', async () => {
    const result = await plugin.load('test.js');
    expect(result).toBeUndefined();
  });

  it('forwards parser diagnostics through Vite', async () => {
    const fixturePath = join(fixturesDir, 'diagnostics.norg');
    const warn = vi.fn();
    const load = plugin.load as (this: { warn: typeof warn }, id: string) => Promise<unknown>;

    await load.call({ warn }, fixturePath);

    expect(warn).toHaveBeenCalledOnce();
    expect(warn).toHaveBeenCalledWith({
      id: fixturePath,
      message: expect.stringContaining('unsafe URL scheme'),
    });
  });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const code = await loadCode(plugin, fixturePath);
    if (code == null) throw new Error(`no code returned for ${fixture}`);
    expect(code.replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
