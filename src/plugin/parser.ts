import type { parseNorg } from '@parser';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { parseNorg: parse } = require('../../dist/napi/index.node');

export const parseNorgContent: typeof parseNorg = parse;
