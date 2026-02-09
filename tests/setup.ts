import { mock } from 'bun:test';
import { parseNorg, parseNorgMetadata, parseNorgWithFramework, getThemeCss } from '../dist/napi/index.js';

mock.module('@parser', () => ({ parseNorg, parseNorgMetadata, parseNorgWithFramework, getThemeCss }));
