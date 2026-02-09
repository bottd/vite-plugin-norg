import { mock } from 'bun:test';
import { parseNorg, parseNorgMetadata, getThemeCss } from '../dist/napi/index.js';

mock.module('@parser', () => ({ parseNorg, parseNorgMetadata, getThemeCss }));
