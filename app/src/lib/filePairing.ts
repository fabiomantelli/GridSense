export interface ComtradePair {
  stem: string;
  cfg: File;
  dat: File;
}

export interface PendingHalf {
  stem: string;
  kind: 'cfg' | 'dat';
  file: File;
}

export interface PairingResult {
  pairs: ComtradePair[];
  pending: PendingHalf[];
  ignored: File[];
}

function stemAndExt(name: string): { stem: string; ext: string } {
  const dot = name.lastIndexOf('.');
  if (dot <= 0) {
    return { stem: name, ext: '' };
  }
  return { stem: name.slice(0, dot), ext: name.slice(dot + 1).toLowerCase() };
}

/**
 * Groups dropped files into .cfg/.dat pairs by matching basename stem
 * (case-insensitive on the extension, exact on the stem — COMTRADE tooling is
 * inconsistent about stem casing across vendors, so we don't fold that too).
 * Non-cfg/dat files are returned as `ignored` rather than erroring, since v1 only
 * handles COMTRADE.
 */
export function pairComtradeFiles(files: File[]): PairingResult {
  const byStem = new Map<string, { cfg?: File; dat?: File }>();
  const ignored: File[] = [];

  for (const file of files) {
    const { stem, ext } = stemAndExt(file.name);
    if (ext !== 'cfg' && ext !== 'dat') {
      ignored.push(file);
      continue;
    }
    const entry = byStem.get(stem) ?? {};
    entry[ext] = file;
    byStem.set(stem, entry);
  }

  const pairs: ComtradePair[] = [];
  const pending: PendingHalf[] = [];

  for (const [stem, entry] of byStem) {
    if (entry.cfg && entry.dat) {
      pairs.push({ stem, cfg: entry.cfg, dat: entry.dat });
    } else if (entry.cfg) {
      pending.push({ stem, kind: 'cfg', file: entry.cfg });
    } else if (entry.dat) {
      pending.push({ stem, kind: 'dat', file: entry.dat });
    }
  }

  return { pairs, pending, ignored };
}
