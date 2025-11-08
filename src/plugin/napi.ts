import type { NorgParseResult } from '@parser';

export interface NorgMetadata {
  title?: string;
  author?: string;
  description?: string;
  [key: string]: unknown;
}

export type NorgParser = (content: string) => NorgParseResult;

const napiModule = require('../../dist/napi/index.node');
export const getWasmParser: NorgParser = napiModule.parseNorg;
