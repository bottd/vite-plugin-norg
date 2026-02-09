import type { NorgParseResult } from '@parser';
import { lines } from './utils';

export const generateSvelteOutput = ({ html, metadata, toc }: NorgParseResult, css: string) =>
  lines`
    <script lang="ts" module>
      export const metadata = ${metadata ?? {}};
      export const toc = ${toc ?? []};
    </script>
    <script lang="ts">
      ${css && `import "virtual:norg-arborium.css";`}
      const htmlContent = ${html};
    </script>
    {@html htmlContent}
  `;
