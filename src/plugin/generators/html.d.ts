declare module '*.norg' {
	import { HtmlModule } from 'vite-plugin-norg';
	export const metadata: HtmlModule['metadata'];
	export const html: HtmlModule['html'];
}
