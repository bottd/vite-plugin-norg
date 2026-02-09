import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateMetadataOutput = ({ metadata }: NorgParseResult) =>
  lines`
    export const metadata = ${metadata ?? {}};
    export default { metadata };
  `;
