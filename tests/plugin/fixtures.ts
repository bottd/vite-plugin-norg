import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const fixturesDir = join(__dirname, '../fixtures');
export const componentsDir = join(fixturesDir, 'components');

export const fixtures = [
  'basic.norg',
  'code-blocks.norg',
  'headings.norg',
  'images.norg',
  'links.norg',
  'inline-css.norg',
];
