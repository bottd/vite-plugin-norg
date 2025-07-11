import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

describe('React Generator', () => {
  it.each(['basic.norg', 'code-blocks.norg', 'headings.norg', 'images.norg', 'links.norg'])(
    'should generate correct React component for %s',
    async fixture => {
      const fixturePath = join(__dirname, '../fixtures', fixture);
      const reactPlugin = norgPlugin({ mode: 'react', include: ['**/*.norg'] });
      const mockContext = {
        error: (err: Error) => {
          throw err;
        },
        resolve: () => Promise.resolve({ id: fixturePath }),
      };
      const result = await reactPlugin.load.call(mockContext, fixturePath);
      expect(result).toMatchSnapshot();
    }
  );
});
