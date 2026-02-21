import { join } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, fixtures } from './fixtures';

describe('React Generator', () => {
  const plugin = norgPlugin({ mode: 'react', include: ['**/*.norg'] });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const result = await plugin.load(fixturePath);
    expect((result as string).replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
