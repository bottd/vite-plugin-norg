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

describe('@embed feature', () => {
  describe('with mode', () => {
    it('should parse @embed tags and collect embeds', async () => {
      const content = `
* Test

@embed svelte
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

      expect(result.embedComponents).toHaveLength(1);
      expect(result.embedComponents[0].index).toBe(0);
      expect(result.embedComponents[0].mode).toBe('svelte');
      expect(result.embedComponents[0].code).toContain('let count');
      expect(result.htmlParts).toHaveLength(2);
    });

    it('should parse multiple @embed tags', async () => {
      const content = `
* Test

@embed svelte
<div>First</div>
@end

@embed svelte
<div>Second</div>
@end
`;
      const result = parseNorg(content, 'svelte');

      expect(result.embedComponents).toHaveLength(2);
      expect(result.embedComponents[0].index).toBe(0);
      expect(result.embedComponents[1].index).toBe(1);
      expect(result.htmlParts).toHaveLength(3);
    });

    it('should error when mode not specified in tag', async () => {
      const content = `
@embed
let x = 1;
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/missing language/i);
    });

    it('should error on invalid language', async () => {
      const content = `
@embed invalid
some code
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/invalid language/i);
    });

    it('should error when embed language mismatches mode', async () => {
      const content = `
@embed vue
<template><div>Hi</div></template>
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/cannot be used in svelte mode/i);
    });

    it('should parse @embed react tags in react mode', async () => {
      const content = `
* Test

@embed react
<button onClick={() => setCount(c => c + 1)}>Click me</button>
@end

* Another heading
`;
      const result = parseNorg(content, 'react');

      expect(result.embedComponents).toHaveLength(1);
      expect(result.embedComponents[0].index).toBe(0);
      expect(result.embedComponents[0].mode).toBe('react');
      expect(result.embedComponents[0].code).toContain('onClick');
      expect(result.htmlParts).toHaveLength(2);
    });

    it('should error when @embed react is used in svelte mode', async () => {
      const content = `
@embed react
<button>Click</button>
@end
`;
      expect(() => parseNorg(content, 'svelte')).toThrow(/cannot be used in svelte mode/i);
    });
  });

  describe('without mode', () => {
    it('should error on @embed tags when no language specified', async () => {
      const content = `
@embed
<script>
  let count = 0;
</script>
@end
`;
      expect(() => parseNorg(content, null)).toThrow(/missing language/i);
    });
  });

  describe('react embed modules', () => {
    const reactFixture = join(fixturesDir, 'embed-react.norg');

    it('should wrap react embed code as JSX component', async () => {
      const plugin = await createPlugin({ mode: 'react', include: ['**/*.norg'] });
      const resolved = plugin.resolveId(`${reactFixture}?embed=0`, reactFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain('export default function NorgEmbed() { return <>');
      expect(result).toContain('onClick');
      expect(result).toContain('</>; }');
    });

    it('should inject component imports into react embed modules', async () => {
      const plugin = await createPlugin({
        mode: 'react',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const resolved = plugin.resolveId(`${reactFixture}?embed=0`, reactFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toContain('export default function NorgEmbed() { return <>');
    });
  });

  describe('componentDir', () => {
    const embedFixture = join(fixturesDir, 'embed.norg');

    it('should inject component imports into embed modules with existing <script>', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const resolved = plugin.resolveId(`${embedFixture}?embed=0`, embedFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toMatch(/<script>\nimport /);
    });

    it('should inject component imports into embed modules without <script>', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const resolved = plugin.resolveId(`${embedFixture}?embed=1`, embedFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).toContain("import Badge from '");
      expect(result).toContain("import Counter from '");
      expect(result).toMatch(/^<script>\nimport /);
      expect(result).toContain('</script>\n<div>Hello from Svelte!</div>');
    });

    it('should not inject imports when no componentDir is set', async () => {
      const plugin = await createPlugin({ mode: 'svelte', include: ['**/*.norg'] });
      const resolved = plugin.resolveId(`${embedFixture}?embed=1`, embedFixture) as string;
      const result = await plugin.load(resolved);

      expect(result).not.toContain('import Badge');
      expect(result).not.toContain('import Counter');
    });

    it('should not inject imports for non-embed modules', async () => {
      const plugin = await createPlugin({
        mode: 'svelte',
        include: ['**/*.norg'],
        componentDir: componentsDir,
      });
      const result = await plugin.load(embedFixture);

      expect(result).not.toContain(`import Badge from '${componentsDir}`);
    });
  });
});
