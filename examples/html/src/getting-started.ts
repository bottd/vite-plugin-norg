import { html, toc } from '../content/getting-started.norg';
import { renderNav, renderPage } from './layout';

renderNav();
renderPage(html, toc);
