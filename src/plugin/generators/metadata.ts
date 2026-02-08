import type { NorgMetadataResult } from '@parser';

export function generateMetadataOutput({ metadata }: NorgMetadataResult): string {
  return [
    `export const metadata = ${JSON.stringify(metadata ?? {})};`,
    `export default { metadata };`,
  ].join('\n');
}
