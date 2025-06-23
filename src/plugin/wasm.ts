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

let parseNorg: NorgParser | null = null;

export async function getWasmParser() {
  if (parseNorg) {
    return parseNorg;
  }

  try {
    const wasmModule = await import('../../pkg/vite_plugin_norg_parser.js');

    await wasmModule.default();

    parseNorg = wasmModule.parse_norg as NorgParser;
    return parseNorg;
  } catch (error) {
    throw new Error(`Failed to load norg parser: ${error}`);
  }
}
