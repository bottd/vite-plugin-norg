import { describe, expect, it } from 'bun:test';
import { parseNorgWithFramework } from '@parser';
import { injectComponentImports } from '../../src/plugin/plugin';

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

      expect(result.inlineComponents).toHaveLength(2);
      expect(result.inlineComponents[0].index).toBe(0);
      expect(result.inlineComponents[1].index).toBe(1);
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

      expect(result.inlineComponents).toHaveLength(1);
      expect(result.inlineComponents[0].framework).toBe('svelte');
    });

    it('should error on invalid framework', async () => {
      const content = `
@inline invalid
some code
@end
`;
      expect(() => parseNorgWithFramework(content, 'svelte')).toThrow(/invalid framework/i);
    });

    it('should error when inline framework mismatches target', async () => {
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

  describe('injectComponentImports', () => {
    const components = {
      EntityHeading: '$lib/components/changelog/EntityHeading.svelte',
      AbilityHeading: '$lib/components/changelog/AbilityHeading.svelte',
      SectionPreview: '$lib/components/changelog/SectionPreview.svelte',
    };

    it('should inject imports when no <script> tag exists', () => {
      const code = '<EntityHeading name="Abrams" type="hero" />';
      const result = injectComponentImports(code, components);

      expect(result).toContain('<script>');
      expect(result).toContain(
        "import EntityHeading from '$lib/components/changelog/EntityHeading.svelte';"
      );
      expect(result).toContain('</script>');
      expect(result).toContain('<EntityHeading name="Abrams" type="hero" />');
    });

    it('should inject imports into existing <script> tag', () => {
      const code = `<script>
  let x = 1;
</script>
<EntityHeading name="Abrams" type="hero" />`;
      const result = injectComponentImports(code, components);

      expect(result).toContain(
        "import EntityHeading from '$lib/components/changelog/EntityHeading.svelte';"
      );
      // Should not add a second <script> tag
      expect(result.match(/<script>/g)).toHaveLength(1);
      expect(result).toContain('let x = 1;');
    });

    it('should not duplicate already-imported components', () => {
      const code = `<script>
  import EntityHeading from '$lib/components/changelog/EntityHeading.svelte';
</script>
<EntityHeading name="Abrams" type="hero" />`;
      const result = injectComponentImports(code, components);

      // Count occurrences of the import — should be exactly 1
      const matches = result.match(/import EntityHeading/g);
      expect(matches).toHaveLength(1);
    });

    it('should leave unregistered component names alone', () => {
      const code = '<UnknownComponent foo="bar" />';
      const result = injectComponentImports(code, components);

      // No imports injected, code unchanged
      expect(result).toBe(code);
    });

    it('should inject multiple components in one block', () => {
      const code = `<EntityHeading name="Abrams" type="hero" />
<AbilityHeading name="Charge" />
<SectionPreview title="Overview" />`;
      const result = injectComponentImports(code, components);

      expect(result).toContain(
        "import EntityHeading from '$lib/components/changelog/EntityHeading.svelte';"
      );
      expect(result).toContain(
        "import AbilityHeading from '$lib/components/changelog/AbilityHeading.svelte';"
      );
      expect(result).toContain(
        "import SectionPreview from '$lib/components/changelog/SectionPreview.svelte';"
      );
    });

    it('should handle <script lang="ts"> tags', () => {
      const code = `<script lang="ts">
  let x: number = 1;
</script>
<EntityHeading name="Abrams" type="hero" />`;
      const result = injectComponentImports(code, components);

      expect(result).toContain(
        "import EntityHeading from '$lib/components/changelog/EntityHeading.svelte';"
      );
      // Should still have only one script tag
      expect(result.match(/<script/g)).toHaveLength(1);
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
      // Without a framework, @inline tags should produce an error
      expect(() => parseNorgWithFramework(content, null)).toThrow(/missing framework/i);
    });
  });
});
