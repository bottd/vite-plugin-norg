import { generateSvelteOutput } from '../../../src/plugin/generators/svelte.js';

describe('Svelte Generator', () => {
  it('should generate Svelte component with title', () => {
    const result = generateSvelteOutput({
      html: '<h1>Test</h1>',
      metadata: { title: 'Test Title', author: 'Test Author' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('<script>');
    expect(result).toContain('const htmlContent = "<h1>Test</h1>";');
    expect(result).toContain('<script context="module">');
    expect(result).toContain(
      'export const metadata = {"title":"Test Title","author":"Test Author"};'
    );
    expect(result).toContain('{@html htmlContent}');
  });

  it('should handle metadata without title', () => {
    const result = generateSvelteOutput({
      html: '<p>Content</p>',
      metadata: { author: 'Test Author' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('{@html htmlContent}');
  });

  it('should escape HTML in title', () => {
    const result = generateSvelteOutput({
      html: '<p>Content</p>',
      metadata: { title: 'Test <script>alert("xss")</script> Title' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('{@html htmlContent}');
  });

  it('should handle special characters in title', () => {
    const result = generateSvelteOutput({
      html: '<p>Content</p>',
      metadata: { title: 'Test & "Quoted" Title' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('{@html htmlContent}');
  });

  it('should generate valid Svelte component structure', () => {
    const result = generateSvelteOutput({
      html: '<h1>Test</h1>',
      metadata: { title: 'Test' },
      ast: null,
      toc: [],
    });
    const lines = result.split('\n');

    expect(lines[0]).toBe('<script>');
    expect(lines[2]).toBe('</script>');
    expect(lines[3]).toBe('<script context="module">');
    expect(lines[5]).toBe('</script>');
    expect(lines[6]).toBe('{@html htmlContent}');
  });
});
