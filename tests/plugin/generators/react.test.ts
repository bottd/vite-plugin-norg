import { describe, it, expect } from 'vitest';
import { generateReactOutput } from '../../../src/plugin/generators/react.js';

describe('React Generator', () => {
  it('should generate React component with metadata', () => {
    const result = generateReactOutput({
      html: '<h1>Test</h1>',
      metadata: { title: 'Test Title', author: 'Test Author' },
      ast: null,
      toc: [],
    });

    expect(result).toBe(
      'import React from "react";\n' +
        'export const metadata = {"title":"Test Title","author":"Test Author"};\n' +
        'const htmlContent = "<h1>Test</h1>";\n' +
        'export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });\n' +
        'export default Component;'
    );
  });

  it('should handle empty metadata', () => {
    const result = generateReactOutput({
      html: '<p>Content</p>',
      metadata: {},
      ast: null,
      toc: [],
    });

    expect(result).toContain('export const metadata = {};');
    expect(result).toContain('const htmlContent = "<p>Content</p>";');
    expect(result).toContain('import React from "react";');
    expect(result).toContain('export default Component;');
  });

  it('should properly escape JSON in output', () => {
    const result = generateReactOutput({
      html: '<p>Quote: "Hello"</p>',
      metadata: { title: 'Test "quoted" title' },
      ast: null,
      toc: [],
    });

    expect(result).toContain('export const metadata = {"title":"Test \\"quoted\\" title"}');
    expect(result).toContain('const htmlContent = "<p>Quote: \\"Hello\\"</p>"');
  });

  it('should use dangerouslySetInnerHTML', () => {
    const html = '<h1>Test</h1>';
    const metadata = { title: 'Test' };

    const result = generateReactOutput({ html, metadata, ast: null, toc: [] });

    expect(result).toContain('dangerouslySetInnerHTML: { __html: htmlContent }');
    expect(result).toContain('React.createElement("div"');
  });

  it('should export Component and default', () => {
    const html = '<h1>Test</h1>';
    const metadata = { title: 'Test' };

    const result = generateReactOutput({ html, metadata, ast: null, toc: [] });

    expect(result).toContain('export const Component = ');
    expect(result).toContain('export default Component;');
  });
});
