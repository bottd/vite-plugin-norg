import { html, toc } from '../content/index.norg';
import { renderNav, renderPage } from './layout';

renderNav();
renderPage(html, toc);
