declare module '*.norg?metadata' {
	import type { MetadataModule } from 'vite-plugin-norg';
	export const metadata: MetadataModule['metadata'];
	const _default: MetadataModule;
	export default _default;
}
