import type { EmbedComponent } from '@parser';
import dedentBase from 'dedent';

const SKIP = '\0';
const baseDedent = dedentBase.withOptions({ alignValues: true, escapeSpecialCharacters: false });

export function dedent(strings: TemplateStringsArray, ...values: unknown[]): string {
  const processed = values.map(v => (v === false || v === null || v === undefined ? SKIP : v));
  const result = baseDedent(strings, ...processed);
  return result
    .split('\n')
    .filter(line => line.trim() !== SKIP)
    .join('\n');
}

export function addEmbedImports(
  embedComponents: EmbedComponent[],
  filePath?: string
): string | null {
  if (embedComponents.length === 0) return null;
  return embedComponents
    .map((_, i) => `import Embed${i} from '${filePath}?embed=${i}';`)
    .join('\n');
}
