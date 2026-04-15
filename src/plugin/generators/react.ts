import type { NorgParseResult } from '@parser';
import { dedent, addEmbedImports } from './helpers';

export function generateReact(
  { htmlParts, metadata, toc, embedComponents = [], embedCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const children = htmlParts
    .flatMap((part, i) => [
      `<div dangerouslySetInnerHTML={{ __html: ${JSON.stringify(part)} }} />`,
      ...(i < embedComponents.length ? [`<Embed${i} />`] : []),
    ])
    .join('\n    ');

  return dedent`
    ${css ? 'import "virtual:norg-arborium.css";' : null}
    ${embedCss && filePath ? `import 'virtual:norg-css:${filePath}';` : null}
    ${addEmbedImports(embedComponents, filePath)}

    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};

    export function Component() {
      return <>${children}</>;
    }
    export default Component;
  `;
}
