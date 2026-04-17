import { join } from 'node:path';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, loadCode } from './fixtures';

const fixtures = ['basic.norg', 'code-blocks.norg', 'headings.norg', 'images.norg', 'links.norg'];

describe('Metadata Generator', () => {
  describe('mode: metadata', () => {
    it.each(fixtures)('should generate correct metadata module for %s', async fixture => {
      const fixturePath = join(fixturesDir, fixture);
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      expect(await loadCode(plugin, fixturePath)).toMatchSnapshot();
    });

    it('should return undefined for non-norg files', async () => {
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      expect(await loadCode(plugin, 'test.js')).toBeUndefined();
    });
  });

  describe('?metadata query', () => {
    it.each(fixtures)(
      'should generate correct metadata module for %s via ?metadata query',
      async fixture => {
        const fixturePath = join(fixturesDir, fixture);
        const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
        expect(await loadCode(plugin, `${fixturePath}?metadata`)).toMatchSnapshot();
      }
    );

    it('should return undefined for non-norg files with ?metadata', async () => {
      const plugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });
      expect(await loadCode(plugin, 'test.js?metadata')).toBeUndefined();
    });
  });

  describe('equivalence', () => {
    it.each(fixtures)(
      'mode: metadata and ?metadata produce identical output for %s',
      async fixture => {
        const fixturePath = join(fixturesDir, fixture);
        const metadataPlugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
        const htmlPlugin = norgPlugin({ mode: 'html', include: ['**/*.norg'] });

        const [modeCode, queryCode] = await Promise.all([
          loadCode(metadataPlugin, fixturePath),
          loadCode(htmlPlugin, `${fixturePath}?metadata`),
        ]);

        expect(modeCode).toBe(queryCode);
      }
    );
  });

  describe('output format', () => {
    it('should not contain html or CSS imports, but should contain toc', async () => {
      const fixturePath = join(fixturesDir, 'basic.norg');
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const code = await loadCode(plugin, fixturePath);

      expect(code).not.toContain('export const html');
      expect(code).toContain('export const toc');
      expect(code).not.toContain('virtual:norg-arborium.css');
    });

    it('should contain metadata export and default export', async () => {
      const fixturePath = join(fixturesDir, 'basic.norg');
      const plugin = norgPlugin({ mode: 'metadata', include: ['**/*.norg'] });
      const code = await loadCode(plugin, fixturePath);

      expect(code).toContain('export const metadata');
      expect(code).toContain('export default');
    });
  });

  describe('?metadata on different modes', () => {
    it.each(['html', 'svelte', 'react'] as const)('?metadata works on %s mode', async mode => {
      const fixturePath = join(fixturesDir, 'basic.norg');
      const plugin = norgPlugin({ mode, include: ['**/*.norg'] });
      const code = await loadCode(plugin, `${fixturePath}?metadata`);

      expect(code).toContain('export const metadata');
      expect(code).not.toContain('export const html');
      expect(code).toContain('export const toc');
    });
  });
});
