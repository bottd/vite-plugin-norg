import { vi } from 'vitest';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
describe('vite-plugin-norg', () => {
  describe('HTML', () => {
    const htmlPlugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });

    it('should generate correct output for basic norg file', async () => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const result = await htmlPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct output for code blocks', async () => {
      const fixturePath = join(__dirname, '../fixtures/code-blocks.norg');
      const result = await htmlPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct output for all heading levels', async () => {
      const fixturePath = join(__dirname, '../fixtures/headings.norg');
      const result = await htmlPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct output for image syntax', async () => {
      const fixturePath = join(__dirname, '../fixtures/images.norg');
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

    it('should resolve .norg files', async () => {
      // Mock the plugin context
      const mockContext = {
        resolve: vi.fn().mockResolvedValue({ id: 'test.norg' }),
      };

      expect(await htmlPlugin.resolveId.call(mockContext, 'test.norg')).toBe('test.norg');
      expect(await htmlPlugin.resolveId.call(mockContext, 'test.js')).toBeNull();
    });
  });

  describe('Svelte Mode', () => {
    const sveltePlugin = norgPlugin({ mode: 'svelte', include: ['**/*.norg'] });

    it('should generate correct component for basic norg file', async () => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const result = await sveltePlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct component for code blocks', async () => {
      const fixturePath = join(__dirname, '../fixtures/code-blocks.norg');
      const result = await sveltePlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct component for all heading levels', async () => {
      const fixturePath = join(__dirname, '../fixtures/headings.norg');
      const result = await sveltePlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });
  });

  describe('React Mode', () => {
    const reactPlugin = norgPlugin({ mode: 'react', include: ['**/*.norg'] });

    it('should generate correct component for basic norg file', async () => {
      const fixturePath = join(__dirname, '../fixtures/basic.norg');
      const result = await reactPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct component for code blocks', async () => {
      const fixturePath = join(__dirname, '../fixtures/code-blocks.norg');
      const result = await reactPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });

    it('should generate correct component for all heading levels', async () => {
      const fixturePath = join(__dirname, '../fixtures/headings.norg');
      const result = await reactPlugin.load(fixturePath);
      expect(result).toMatchSnapshot();
    });
  });
});
