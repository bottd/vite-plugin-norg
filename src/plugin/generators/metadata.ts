import type { NorgMetadataResult } from '@parser';
import { lines } from './utils';

export const generateMetadataOutput = ({ metadata }: NorgMetadataResult) =>
  lines`
    export const metadata = ${metadata ?? {}};
    export default { metadata };
  `;
