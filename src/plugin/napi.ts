// Type definitions
export interface NorgMetadata {
  title?: string;
  author?: string;
  description?: string;
  [key: string]: unknown;
}

export interface TocEntry {
  level: number;
  title: string;
  id: string;
}

export interface NorgParseResult {
  metadata: NorgMetadata | null;
  html: string;
  toc: TocEntry[];
}

export type NorgParser = (content: string) => NorgParseResult;

// Import the napi module and export the parser directly
const napiModule = require('../../dist/napi/index.node');
export const getWasmParser: NorgParser = napiModule.parseNorg;
