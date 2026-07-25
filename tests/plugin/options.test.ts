import { norgPlugin } from '../../src/plugin/index.js';
import type { GeneratorMode } from '../../src/plugin/generators';

const MODES: GeneratorMode[] = ['html', 'svelte', 'react', 'vue', 'metadata'];

it.each(MODES)('accepts the %s mode', mode => {
  expect(norgPlugin({ mode }).name).toBe('vite-plugin-norg');
});

it('rejects an unknown mode instead of emitting an undefined module', () => {
  // Without the guard the plugin loads and silently produces nothing.
  expect(() => norgPlugin({ mode: 'nope' as GeneratorMode })).toThrow(
    /Invalid mode "nope".*Expected one of: html, svelte, vue, react, metadata/s
  );
});
