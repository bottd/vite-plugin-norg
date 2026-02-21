import type { TocEntry } from '@parser';

export interface MetadataModule {
	metadata: Record<string, unknown>;
	toc: TocEntry[];
}

declare module '*.norg?metadata' {
	import type { MetadataModule } from 'vite-plugin-norg';
	export const metadata: MetadataModule['metadata'];
	export const toc: MetadataModule['toc'];
	const _default: MetadataModule;
	export default _default;
}
