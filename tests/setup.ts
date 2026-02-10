import { mock } from 'bun:test';
import { parseNorg, parseNorgWithFramework, parseNorgMetadata, getThemeCss } from '../dist/napi/index.js';

mock.module('@parser', () => ({ parseNorg, parseNorgWithFramework, parseNorgMetadata, getThemeCss }));
