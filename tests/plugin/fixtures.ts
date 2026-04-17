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
  'embed-css.norg',
];

export async function loadCode(
  plugin: { load?: (id: string) => unknown | Promise<unknown> },
  id: string
): Promise<string | undefined> {
  const result = await plugin.load?.(id);
  if (result == null) return undefined;
  return typeof result === 'string' ? result : (result as { code: string }).code;
}
