import 'virtual:norg-arborium.css';

const navItems = [
  { href: '/', label: 'Home' },
  { href: '/getting-started', label: 'Getting Started' },
  { href: '/configuration', label: 'Configuration' },
  { href: '/embed-components', label: 'Embed Components' },
];

export function renderNav() {
  const nav = document.getElementById('nav') as HTMLElement;
  nav.innerHTML = `
    <a href="/" class="logo">vite-plugin-norg</a>
    <ul>
      ${navItems.map(({ href, label }) => `<li><a href="${href}">${label}</a></li>`).join('')}
    </ul>
  `;
}

export function renderPage(html: string, toc: { title: string; level: number; id: string }[]) {
  const content = document.getElementById('content') as HTMLElement;

  if (toc.length > 0) {
    const tocHtml = `<aside class="toc"><h3>On this page</h3><ul>${toc
      .map(
        entry =>
          `<li style="padding-left: ${(entry.level - 1) * 0.75}rem"><a href="#${entry.id}">${entry.title}</a></li>`
      )
      .join('')}</ul></aside>`;
    content.innerHTML = `<div class="page-with-toc">${tocHtml}<article>${html}</article></div>`;
  } else {
    content.innerHTML = html;
  }
}

// Inject global styles
const style = document.createElement('style');
style.textContent = `
  body {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    color: #1a1a1a;
    background: #fafafa;
  }
  #app {
    max-width: 900px;
    margin: 0 auto;
    padding: 0 1.5rem;
  }
  nav {
    display: flex;
    align-items: center;
    gap: 2rem;
    padding: 1rem 0;
    border-bottom: 1px solid #e5e5e5;
    margin-bottom: 2rem;
  }
  .logo {
    font-weight: 700;
    font-size: 1.1rem;
    text-decoration: none;
    color: #1a1a1a;
  }
  nav ul {
    display: flex;
    gap: 1.5rem;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  nav a {
    color: #666;
    text-decoration: none;
  }
  nav a:hover {
    color: #1a1a1a;
  }
  main {
    padding-bottom: 4rem;
  }
  main pre {
    padding: 1rem;
    border-radius: 6px;
    overflow-x: auto;
  }
  main code {
    font-size: 0.9em;
  }
  main a {
    color: #0066cc;
  }
  .toc {
    margin-bottom: 2rem;
    padding: 1rem;
    background: #f5f5f5;
    border-radius: 6px;
  }
  .toc h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #666;
  }
  .toc ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .toc li {
    margin: 0.25rem 0;
  }
  .toc a {
    color: #444;
    text-decoration: none;
    font-size: 0.9rem;
  }
  .toc a:hover {
    color: #0066cc;
  }
`;
document.head.appendChild(style);
