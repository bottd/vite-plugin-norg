import type { NorgParseResult } from '@parser';
import { dedent, addInlineImports } from './helpers';

export function generateReact(
  { htmlParts, metadata, toc, inlineComponents = [], inlineCss = '' }: NorgParseResult,
  css: string,
  filePath?: string
): string {
  const children = htmlParts
    .flatMap((part, i) => [
      ...(part
        ? [
            `React.createElement("div", { dangerouslySetInnerHTML: { __html: ${JSON.stringify(part)} } })`,
          ]
        : []),
      ...(i < inlineComponents.length ? [`React.createElement(Inline${i})`] : []),
    ])
    .join(', ');

  return dedent`
    import React from "react";
    ${css ? 'import "virtual:norg-arborium.css";' : null}
    ${inlineCss && filePath ? `import 'virtual:norg-css:${filePath}';` : null}
    ${addInlineImports(inlineComponents, filePath)}

    export const metadata = ${JSON.stringify(metadata ?? {})};
    export const toc = ${JSON.stringify(toc ?? [])};

    ${
      inlineComponents.length > 0
        ? dedent`
        export const Component = () => React.createElement(React.Fragment, null, ${children});
        export default Component;
      `
        : dedent`
        const htmlContent = ${JSON.stringify(htmlParts.join(''))};

        export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });
        export default Component;
      `
    }
  `;
}
