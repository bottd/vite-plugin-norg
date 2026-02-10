export interface MetadataModule {
  metadata: Record<string, unknown>;
}

declare module '*.norg?metadata' {
	export const metadata: Record<string, unknown>;
	const _default: MetadataModule;
	export default _default;
}
