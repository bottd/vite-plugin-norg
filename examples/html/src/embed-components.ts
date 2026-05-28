import { html, toc } from '../content/embed-components.norg';
import { renderNav, renderPage } from './layout';

renderNav();
renderPage(html, toc);
