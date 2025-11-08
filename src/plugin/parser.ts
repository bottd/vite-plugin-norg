import type { parseNorg } from '@parser';

const { parseNorg: parse } = require('../../dist/napi/index.node');

export const parseNorgContent: typeof parseNorg = parse;
