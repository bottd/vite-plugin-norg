import { mock } from 'bun:test';
import { parseNorg, parseNorgWithFramework, getThemeCss } from '../dist/napi/index.js';

mock.module('@parser', () => ({ parseNorg, parseNorgWithFramework, getThemeCss }));
