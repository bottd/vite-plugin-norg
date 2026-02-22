import type { InlineComponent } from '@parser';
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

export function addInlineImports(
  inlineComponents: InlineComponent[],
  filePath?: string
): string | null {
  if (inlineComponents.length === 0) return null;
  return inlineComponents
    .map((_, i) => `import Inline${i} from '${filePath}?inline=${i}';`)
    .join('\n');
}
