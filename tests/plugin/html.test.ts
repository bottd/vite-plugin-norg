import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('HTML Generator', () => {
  it.each(['basic.norg', 'code-blocks.norg', 'headings.norg', 'images.norg', 'links.norg'])(
    'should generate correct HTML module for %s',
    async fixture => {
      const fixturePath = join(__dirname, '../fixtures', fixture);
      const htmlPlugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
      const result = await htmlPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    }
  );
});
