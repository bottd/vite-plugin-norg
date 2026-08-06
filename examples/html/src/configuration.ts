import { html, toc } from '../content/configuration.norg';
import { renderNav, renderPage } from './layout';

renderNav();
renderPage(html, toc);
