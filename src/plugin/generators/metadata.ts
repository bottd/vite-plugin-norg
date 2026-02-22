import type { NorgParseResult } from '@parser';
import { dedent } from './helpers';

export function generateMetadata({ metadata, toc }: NorgParseResult): string {
  return dedent`
    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};
    export default { metadata, toc };
  `;
}
