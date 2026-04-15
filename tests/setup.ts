import { vi } from 'vitest';

vi.mock('@parser', async () => {
  const mod = await import('../dist/napi/index.js');
  return {
    parseNorg: mod.parseNorg,
    getThemeCss: mod.getThemeCss,
    OutputMode: mod.OutputMode,
  };
});
