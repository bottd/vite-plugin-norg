import assert from 'node:assert/strict';
import { parseNorg, getThemeCss } from '../dist/napi/index.js';

const doc = `@document.meta
title: Smoke
@end

* Heading
  - item
  -- nested item

@code js
const answer = 42;
@end
`;

const result = await parseNorg(doc, 'html');
assert.equal(result.metadata.title, 'Smoke');
assert.equal(result.toc[0]?.title, 'Heading');
assert.match(result.htmlParts.join(''), /<a-k>const<\/a-k>/);

await assert.rejects(parseNorg('@embed\nx\n@end\n', 'svelte'), /missing language/i);

assert.ok(getThemeCss('GitHub Dark').length > 0, 'theme css is empty');

console.log(`native binding OK (${process.platform}-${process.arch})`);
