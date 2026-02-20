import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { describe, expect, it } from 'bun:test';
import { parseNorgWithFramework } from '@parser';
import { norgPlugin } from '../../src/plugin/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, '../fixtures');
const componentsDir = join(fixturesDir, 'components');

describe('@inline feature', () => {
  describe('with framework mode', () => {
    it('should parse @inline tags and collect inlines', async () => {
      const content = `
* Test

@inline svelte
<script>
  let count = 0;
</script>

<button on:click={() => count++}>
  Count: {count}
</button>
@end

* Another heading
`;
      const result = parseNorgWithFramework(content, 'svelte');

      expect(result.inlineComponents).toHaveLength(1);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[0].framework).toBe('svelte');
      expect(result.inlineComponents[0].code).toContain('let count');
      expect(result.htmlParts).toHaveLength(2);
    });

    it('should parse multiple @inline tags', async () => {
      const content = `
* Test

@inline svelte
<div>First</div>
@end

@inline svelte
<div>Second</div>
@end
`;
      const result = parseNorgWithFramework(content, 'svelte');

      expect(result.inlineComponents).toHaveLength(2);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[1].index).toBe(1);
      expect(result.htmlParts).toHaveLength(3);
    });

    it('should error when framework not specified in tag', async () => {
      const content = `
@inline
let x = 1;
@end
`;
      expect(() => parseNorgWithFramework(content, 'svelte')).toThrow(/missing language/i);
    });

    it('should error on invalid language', async () => {
      const content = `
@inline invalid
some code
@end
`;
      expect(() => parseNorgWithFramework(content, 'svelte')).toThrow(/invalid language/i);
    });

    it('should error when inline language mismatches target', async () => {
      const content = `
@inline vue
<template><div>Hi</div></template>
@end
`;
      expect(() => parseNorgWithFramework(content, 'svelte')).toThrow(
        /cannot be used in a svelte project/i
      );
    });
  });

  describe('without framework mode', () => {
    it('should error on @inline tags when no language specified', async () => {
      const content = `
@inline
<script>
  let count = 0;
</script>
@end
`;
      expect(() => parseNorgWithFramework(content, null)).toThrow(/missing language/i);
    });
  });

  describe('componentDir', () => {
    const inlineFixture = join(fixturesDir, 'inline.norg');

    async function createPlugin(opts: Parameters<typeof norgPlugin>[0]) {
      const plugin = norgPlugin(opts);
      const hook = plugin.buildStart;
      if (typeof hook === 'function') {
        await (hook as () => Promise<void>)();
      }
      return plugin;
    }

    it('should inject component imports into inline modules with existing <script>', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });

      // Load inline=0 which has a <script> tag
      const result = await plugin.load(`${inlineFixture}.svelte?inline=0`);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toMatch(/<script>\nimport /);
    });

    it('should inject component imports into inline modules without <script>', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });

      // Load inline=1 which is just <div>Hello from Svelte!</div> (no script tag)
      const result = await plugin.load(`${inlineFixture}.svelte?inline=1`);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toMatch(/^<script>\nimport /);
      expect(result).toContain('</script>\n<div>Hello from Svelte!</div>');
    });

    it('should not inject imports when no componentDir is set', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
      });

      const result = await plugin.load(`${inlineFixture}.svelte?inline=1`);

      expect(result).not.toContain('import Badge');
      expect(result).not.toContain('import Counter');
    });

    it('should not inject imports for non-inline modules', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });

      const result = await plugin.load(inlineFixture);

      expect(result).not.toContain(`import Badge from '${componentsDir}`);
    });
  });
});
