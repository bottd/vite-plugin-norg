import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const fixtures = ['basic.norg', 'code-blocks.norg', 'headings.norg', 'images.norg', 'links.norg'];

describe('Metadata Generator', () => {
  describe('mode: metadata', () => {
    it.each(fixtures)('should generate correct metadata module for %s', async fixture => {
      const fixturePath = join(__dirname, '../fixtures', fixture);
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const result = await plugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should return undefined for non-norg files', async () => {
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const result = await plugin.load('test.js');
      expect(result).toBeUndefined();
    });
  });

  describe('?metadata query', () => {
    it.each(fixtures)(
      'should generate correct metadata module for %s via ?metadata query',
      async fixture => {
        const fixturePath = join(__dirname, '../fixtures', fixture);
        const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
        const result = await plugin.load(`${fixturePath}?metadata`);
        expect(result).toMatchSnapshot();
      }
    );

    it('should return undefined for non-norg files with ?metadata', async () => {
      const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
      const result = await plugin.load('test.js?metadata');
      expect(result).toBeUndefined();
    });
  });

  describe('equivalence', () => {
    it.each(fixtures)(
      'mode: metadata and ?metadata produce identical output for %s',
      async fixture => {
        const fixturePath = join(__dirname, '../fixtures', fixture);
        const metadataPlugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
        const htmlPlugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });

        const modeResult = await metadataPlugin.load(fixturePath);
        const queryResult = await htmlPlugin.load(`${fixturePath}?metadata`);

        expect(modeResult).toBe(queryResult);
      }
    );
  });

  describe('output format', () => {
    it('should not contain html, toc, or CSS imports', async () => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const result = await plugin.load(fixturePath);

      expect(result).not.toContain('export const html');
      expect(result).not.toContain('export const toc');
      expect(result).not.toContain('virtual:norg-arborium.css');
    });

    it('should contain metadata export and default export', async () => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const result = await plugin.load(fixturePath);

      expect(result).toContain('export const metadata');
      expect(result).toContain('export default');
    });
  });

  describe('?metadata on different modes', () => {
    it.each(['html', 'svelte', 'react'] as const)('?metadata works on %s mode', async mode => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const plugin = norgPlugin({ mode, include: ['**/*.norg'] });
      const result = await plugin.load(`${fixturePath}?metadata`);

      expect(result).toContain('export const metadata');
      expect(result).not.toContain('export const html');
      expect(result).not.toContain('export const toc');
    });
  });
});
