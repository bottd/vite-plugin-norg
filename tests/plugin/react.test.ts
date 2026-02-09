import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const fixtures = [
  'basic.norg',
  'code-blocks.norg',
  'headings.norg',
  'images.norg',
  'links.norg',
  'inline-css.norg',
];

describe('React Generator', () => {
  const plugin = norgPlugin({ mode: 'react', include: ['**/*.norg'] });

  it.each(fixtures)('generates correct output for %s', async fixture => {
    const fixturePath = join(__dirname, '../fixtures', fixture);
    const result = await plugin.load(fixturePath);
    expect(result).toMatchSnapshot();
  });
});
