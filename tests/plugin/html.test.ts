import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const fixtures = ['basic.norg', 'code-blocks.norg', 'headings.norg', 'images.norg', 'links.norg'];

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
    const fixturePath = join(__dirname, '../fixtures', fixture);
    const result = await plugin.load(fixturePath);
    expect(result).toMatchSnapshot();
  });
});
