import init, { parse_comtrade, ComtradeHandle } from '../wasm-pkg/gridsense_wasm.js';
import type { CfgFile } from './types';

let ready: Promise<unknown> | null = null;

/** Loads and instantiates the WASM module exactly once, on first use. */
function ensureInit(): Promise<unknown> {
  if (!ready) {
    ready = init();
  }
  return ready;
}

export interface LoadedRecord {
  handle: ComtradeHandle;
  metadata: CfgFile;
}

/**
 * Parses a COMTRADE record from raw .cfg/.dat bytes. The returned handle owns WASM
 * heap memory and must be released with `disposeRecord` when the session ends (e.g.
 * before loading a new file), or the WASM heap grows unbounded.
 */
export async function loadComtrade(cfgBytes: Uint8Array, datBytes: Uint8Array): Promise<LoadedRecord> {
  await ensureInit();
  const handle = parse_comtrade(cfgBytes, datBytes);
  const metadata = handle.metadata() as CfgFile;
  return { handle, metadata };
}

export function disposeRecord(record: LoadedRecord | null): void {
  record?.handle.free();
}
