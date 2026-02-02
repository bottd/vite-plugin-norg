import { mock } from 'bun:test';
import { parseNorg, getThemeCss } from '../dist/napi/index.js';

mock.module('@parser', () => ({ parseNorg, getThemeCss }));
