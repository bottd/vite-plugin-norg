import { mock } from 'bun:test';
import { parseNorg, getThemeCss, OutputMode } from '../dist/napi/index.js';

mock.module('@parser', () => ({
  parseNorg,
  getThemeCss,
  OutputMode,
}));
