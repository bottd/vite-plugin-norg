declare module '*.norg' {
	import { ReactModule } from 'vite-plugin-norg';
	export const metadata: ReactModule['metadata'];
	export const Component: ReactModule['Component'];
}
