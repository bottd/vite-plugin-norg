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

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const code = await loadCode(plugin, fixturePath);
    if (code == null) throw new Error(`no code returned for ${fixture}`);
    expect(code.replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
