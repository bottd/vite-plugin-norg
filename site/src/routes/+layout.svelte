<script lang="ts">
  import 'virtual:norg-arborium.css';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="site">
  <header>
    <div class="header-inner">
      <a href="/" class="logo">vite-plugin-norg</a>
      <nav>
        <a href="https://github.com/bottd/vite-plugin-norg">GitHub</a>
      </nav>
    </div>
  </header>
  <main>
    {@render children()}
  </main>
  <footer>
    <p>MIT License &middot; Built with SvelteKit &amp; vite-plugin-norg</p>
  </footer>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    color: #1a1a1a;
    background: #fff;
  }
  .site {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    border-bottom: 1px solid #e5e5e5;
  }
  .header-inner {
    max-width: 1100px;
    margin: 0 auto;
    padding: 1rem 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .logo {
    font-weight: 700;
    font-size: 1.2rem;
    text-decoration: none;
    color: #1a1a1a;
  }
  nav a {
    color: #666;
    text-decoration: none;
    font-size: 0.95rem;
  }
  nav a:hover {
    color: #1a1a1a;
  }
  main {
    flex: 1;
  }
  footer {
    border-top: 1px solid #e5e5e5;
    padding: 2rem 1.5rem;
    text-align: center;
    color: #999;
    font-size: 0.85rem;
  }
  main :global(pre) {
    padding: 1rem;
    border-radius: 6px;
    overflow-x: auto;
  }
  main :global(code) {
    font-size: 0.9em;
  }
</style>
