import { join } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, fixtures } from './fixtures';

describe('Svelte Generator', () => {
  const plugin = norgPlugin({ mode: 'svelte', include: ['**/*.norg'] });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const result = await plugin.load(fixturePath);
    expect((result as string).replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
