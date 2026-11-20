<script setup lang="ts">
import 'virtual:norg-arborium.css';
import { ref, computed, shallowRef } from 'vue';

import Index, { metadata as indexMeta } from '../content/index.norg';
import GettingStarted, { metadata as gsMeta } from '../content/getting-started.norg';
import Configuration, { metadata as configMeta } from '../content/configuration.norg';
import EmbedComponents, { metadata as embedMeta } from '../content/embed-components.norg';

const pages = [
  { id: 'index', label: 'Home', component: shallowRef(Index), meta: indexMeta },
  { id: 'getting-started', label: 'Getting Started', component: shallowRef(GettingStarted), meta: gsMeta },
  { id: 'configuration', label: 'Configuration', component: shallowRef(Configuration), meta: configMeta },
  { id: 'embed-components', label: 'Embed Components', component: shallowRef(EmbedComponents), meta: embedMeta },
] as const;

const currentPage = ref('index');
const current = computed(() => pages.find(p => p.id === currentPage.value) ?? pages[0]);
</script>

<template>
  <div class="layout">
    <nav>
      <span class="logo">vite-plugin-norg</span>
      <ul>
        <li v-for="{ id, label } in pages" :key="id">
          <button :class="{ active: currentPage === id }" @click="currentPage = id">{{ label }}</button>
        </li>
      </ul>
    </nav>
    <main>
      <component :is="current.component" />
    </main>
  </div>
</template>

<style>
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  color: #1a1a1a;
  background: #fafafa;
}
.layout {
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
}
ul {
  display: flex;
  gap: 1rem;
  list-style: none;
  margin: 0;
  padding: 0;
}
button {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 0.95rem;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
}
button:hover {
  color: #1a1a1a;
}
button.active {
  color: #1a1a1a;
  font-weight: 600;
}
main {
  padding-bottom: 4rem;
}
main :deep(pre) {
  padding: 1rem;
  border-radius: 6px;
  overflow-x: auto;
}
main :deep(code) {
  font-size: 0.9em;
}
main :deep(a) {
  color: #0066cc;
}
</style>
