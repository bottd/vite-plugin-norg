<script lang="ts">
  import Content, { metadata } from '../../content/index.norg';
</script>

<svelte:head>
  <title>{metadata.title}</title>
  <meta name="description" content={metadata.description} />
</svelte:head>

<div class="hero">
  <h1>vite-plugin-norg</h1>
  <p class="tagline">Write in Norg. Render anywhere.</p>
  <div class="hero-actions">
    <a href="https://github.com/bottd/vite-plugin-norg" class="btn btn-primary">GitHub</a>
    <a href="https://github.com/bottd/vite-plugin-norg/tree/main/examples" class="btn btn-secondary">Examples</a>
  </div>
</div>

<div class="content">
  <Content />
</div>

<style>
  .hero {
    text-align: center;
    padding: 4rem 1.5rem 2rem;
  }
  .hero h1 {
    font-size: 3rem;
    margin: 0;
    letter-spacing: -0.02em;
  }
  .tagline {
    font-size: 1.3rem;
    color: #666;
    margin: 0.75rem 0 2rem;
  }
  .hero-actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
  }
  .btn {
    display: inline-block;
    padding: 0.6rem 1.5rem;
    border-radius: 6px;
    text-decoration: none;
    font-weight: 500;
    font-size: 1rem;
  }
  .btn-primary {
    background: #1a1a1a;
    color: #fff;
  }
  .btn-primary:hover {
    background: #333;
  }
  .btn-secondary {
    background: #f0f0f0;
    color: #1a1a1a;
  }
  .btn-secondary:hover {
    background: #e0e0e0;
  }
  .content {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
  }
  .content :global(a) {
    color: #0066cc;
  }
</style>
