import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateReactOutput = ({ html, metadata, toc }: NorgParseResult, css: string) =>
  lines`
    import React from "react";
    ${css && `import "virtual:norg-arborium.css";`}
    export const metadata = ${metadata ?? {}};
    export const toc = ${toc ?? []};
    const htmlContent = ${html};
    export const Component = () => React.createElement("div", { dangerouslySetInnerHTML: { __html: htmlContent } });
    export default Component;
  `;
