import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
describe('vite-plugin-norg', () => {
  describe('HTML', () => {
    const htmlPlugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });

    it.each([
      { fixture: 'basic.norg', description: 'should generate correct output for basic norg file' },
      {
        fixture: 'code-blocks.norg',
        description: 'should generate correct output for code blocks',
      },
      {
        fixture: 'headings.norg',
        description: 'should generate correct output for all heading levels',
      },
      { fixture: 'images.norg', description: 'should generate correct output for image syntax' },
    ])('$description', async ({ fixture }) => {
      const fixturePath = join(__dirname, '../fixtures', fixture);
      const result = await htmlPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should return null for non-norg files', async () => {
      const result = await htmlPlugin.load('console.log("test");', 'test.js');
      expect(result).toBeUndefined();
    });

    it('should have correct plugin name', () => {
      expect(htmlPlugin.name).toBe('vite-plugin-norg');
    });

    it('should enforce pre', () => {
      expect(htmlPlugin.enforce).toBe('pre');
    });
  });
});
