import { join } from 'node:path';
import { describe, expect, it } from 'bun:test';
import { parseNorg } from '@parser';
import { norgPlugin } from '../../src/plugin/index.js';
import { fixturesDir, componentsDir } from './fixtures';

async function createPlugin(opts: Parameters<typeof norgPlugin>[0]) {
  const plugin = norgPlugin(opts);
  const hook = plugin.buildStart;
  if (typeof hook === 'function') {
    await (hook as () => Promise<void>)();
  }
  return plugin;
}

describe('@inline feature', () => {
  describe('with mode', () => {
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
      const result = parseNorg(content, 'svelte');

      expect(result.inlineComponents).toHaveLength(1);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[0].mode).toBe('svelte');
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
      const result = parseNorg(content, 'svelte');

      expect(result.inlineComponents).toHaveLength(2);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[1].index).toBe(1);
      expect(result.htmlParts).toHaveLength(3);
    });

    it('should error when mode not specified in tag', async () => {
      const content = `
@inline
let x = 1;
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/missing language/i);
    });

    it('should error on invalid language', async () => {
      const content = `
@inline invalid
some code
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/invalid language/i);
    });

    it('should error when inline language mismatches mode', async () => {
      const content = `
@inline vue
<template><div>Hi</div></template>
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/cannot be used in svelte mode/i);
    });

    it('should parse @inline react tags in react mode', async () => {
      const content = `
* Test

@inline react
<button onClick={() => setCount(c => c + 1)}>Click me</button>
@end

* Another heading
`;
      const result = parseNorg(content, 'react');

      expect(result.inlineComponents).toHaveLength(1);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[0].mode).toBe('react');
      expect(result.inlineComponents[0].code).toContain('onClick');
      expect(result.htmlParts).toHaveLength(2);
    });

    it('should error when @inline react is used in svelte mode', async () => {
      const content = `
@inline react
<button>Click</button>
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/cannot be used in svelte mode/i);
    });
  });

  describe('without mode', () => {
    it('should error on @inline tags when no language specified', async () => {
      const content = `
@inline
<script>
  let count = 0;
</script>
@end
`;
      expect(() => parseNorg(content, null)).toThrow(/missing language/i);
    });
  });

  describe('react inline modules', () => {
    const reactFixture = join(fixturesDir, 'inline-react.norg');

    it('should wrap react inline code as JSX component', async () => {
      const plugin = await createPlugin({ mode: 'react', include: ['**/*.norg'] });
      const resolved = plugin.resolveId(`${reactFixture}?inline=0`, reactFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain('export default function NorgInline() { return <>');
      expect(result).toContain('onClick');
      expect(result).toContain('</>; }');
    });

    it('should inject component imports into react inline modules', async () => {
      const plugin = await createPlugin({
        mode: 'react',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const resolved = plugin.resolveId(`${reactFixture}?inline=0`, reactFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toContain('export default function NorgInline() { return <>');
    });
  });

  describe('componentDir', () => {
    const inlineFixture = join(fixturesDir, 'inline.norg');

    it('should inject component imports into inline modules with existing <script>', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const resolved = plugin.resolveId(`${inlineFixture}?inline=0`, inlineFixture) as string;
      const result = await plugin.load(resolved);

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
      const resolved = plugin.resolveId(`${inlineFixture}?inline=1`, inlineFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toMatch(/^<script>\nimport /);
      expect(result).toContain('</script>\n<div>Hello from Svelte!</div>');
    });

    it('should not inject imports when no componentDir is set', async () => {
      const plugin = await createPlugin({ mode: 'svelte', include: ['**/*.norg'] });
      const resolved = plugin.resolveId(`${inlineFixture}?inline=1`, inlineFixture) as string;
      const result = await plugin.load(resolved);

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
