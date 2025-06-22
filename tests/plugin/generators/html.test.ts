import { describe, it, expect } from 'vitest';
import { generateHtmlOutput } from '../../../src/plugin/generators/html.js';

describe('HTML Generator', () => {
  it('should generate correct HTML module with metadata', () => {
    const result = generateHtmlOutput({
      html: '<h1>Test</h1>',
      metadata: { title: 'Test Title', author: 'Test Author' },
      ast: null,
      toc: [],
    });

    expect(result).toBe(
      'export const metadata = {"title":"Test Title","author":"Test Author"};\n' +
        'export const html = "<h1>Test</h1>";\n' +
        'export default { metadata, html };'
    );
  });

  it('should handle empty metadata', () => {
    const result = generateHtmlOutput({
      html: '<p>Content</p>',
      metadata: {},
      ast: null,
      toc: [],
    });

    expect(result).toBe(
      'export const metadata = {};\n' +
        'export const html = "<p>Content</p>";\n' +
        'export default { metadata, html };'
    );
  });

  it('should properly escape JSON in output', () => {
    const result = generateHtmlOutput({
      html: '<p>Quote: "Hello"</p>',
      metadata: { title: 'Test "quoted" title' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('export const metadata = {"title":"Test \\"quoted\\" title"}');
    expect(result).toContain('export const html = "<p>Quote: \\"Hello\\"</p>"');
  });
});
