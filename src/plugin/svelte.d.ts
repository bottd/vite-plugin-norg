declare module '*.norg' {
	import { SvelteModule } from 'vite-plugin-norg';
	const component: SvelteModule['default'];
	export const metadata: SvelteModule['metadata'];
	export default component;
}
