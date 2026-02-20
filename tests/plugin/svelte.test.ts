import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, '../fixtures');

const fixtures = [
  'basic.norg',
  'code-blocks.norg',
  'headings.norg',
  'images.norg',
  'links.norg',
  'inline-css.norg',
];

describe('Svelte Generator', () => {
  const plugin = norgPlugin({ mode: 'svelte', include: ['**/*.norg'] });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(fixturesDir, fixture);
    const result = await plugin.load(fixturePath);
    expect((result as string).replaceAll(fixturesDir, '<fixtures>')).toMatchSnapshot();
  });
});
