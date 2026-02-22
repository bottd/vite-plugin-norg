import type { NorgParseResult } from '@parser';
import { dedent, addInlineImports } from './helpers';

export function generateReact(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const children = htmlParts
    .flatMap((part, i) => [
      ...(part ? [`<div dangerouslySetInnerHTML={{ __html: ${JSON.stringify(part)} }} />`] : []),
      ...(i < inlineComponents.length ? [`<Inline${i} />`] : []),
    ])
    .join('\n    ');

  return dedent`
    ${css ? 'import "virtual:norg-arborium.css";' : null}
    ${inlineCss && filePath ? `import 'virtual:norg-css:${filePath}';` : null}
    ${addInlineImports(inlineComponents, filePath)}

    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};

    export function Component() {
      return <>${children}</>;
    }
    export default Component;
  `;
}
