import { join } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, fixtures, loadCode } from './fixtures';

describe('React Generator', () => {
  const plugin = norgPlugin({ mode: 'react', include: ['**/*.norg'] });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const code = await loadCode(plugin, fixturePath);
    if (code == null) throw new Error(`no code returned for ${fixture}`);
    expect(code.replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
