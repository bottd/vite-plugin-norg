import { NorgParseResult } from '../napi';

export const generateHtmlOutput = ({ html, metadata }: NorgParseResult) => {
  const metadataJson = JSON.stringify(metadata ?? {});
  const htmlJson = JSON.stringify(html);

  return [
    `export const metadata = ${metadataJson};`,
    `export const html = ${htmlJson};`,
    `export default { metadata, html };`,
  ].join('\n');
};
