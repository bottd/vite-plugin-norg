import { describe, expect, it } from 'bun:test';
import { parseNorgWithFramework } from '@parser';

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

      expect(result.inlines).toHaveLength(1);
      expect(result.inlines[0].index).toBe(0);
      expect(result.inlines[0].framework).toBe('svelte');
      expect(result.inlines[0].code).toContain('let count');
      // With N inlines, there should be N+1 parts
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

      expect(result.inlines).toHaveLength(2);
      expect(result.inlines[0].index).toBe(0);
      expect(result.inlines[1].index).toBe(1);
      // With N inlines, there should be N+1 parts
      expect(result.htmlParts).toHaveLength(3);
    });

    it('should infer framework from config when not specified in tag', async () => {
      const content = `
@inline
let x = 1;
@end
`;
      const result = parseNorgWithFramework(content, 'svelte');

      expect(result.inlines).toHaveLength(1);
      expect(result.inlines[0].framework).toBe('svelte');
    });

    it('should error on invalid framework', async () => {
      const content = `
@inline invalid
some code
@end
`;
      const result = parseNorgWithFramework(content, 'svelte');

      expect(result.htmlParts.join('')).toContain('invalid framework');
      expect(result.inlines).toHaveLength(0);
    });
  });

  describe('without framework mode', () => {
    it('should error on @inline tags when no framework specified', async () => {
      const content = `
@inline
<script>
  let count = 0;
</script>
@end
`;
      const result = parseNorgWithFramework(content, null);

      // Without a framework, @inline tags should produce an error
      expect(result.inlines).toHaveLength(0);
      expect(result.htmlParts.join('')).toContain('invalid framework');
    });
  });
});
