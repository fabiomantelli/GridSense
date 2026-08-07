import uPlot from 'uplot';

export interface RecordTimeSeries {
  xsSeconds: Float64Array;
  ys: Float32Array;
}

/**
 * Merges N independently-sampled time series onto one shared, sorted x-axis via
 * uPlot's own `join()` utility — different COMTRADE records have different sample
 * rates/counts and their absolute sample instants essentially never coincide, so a
 * plain shared x-array (what single-record aligned-mode charts use) doesn't work
 * here. `join()` produces one sorted, deduplicated x-array plus one y-array per
 * record, with `null` at every position that belongs to a different record.
 *
 * Each resulting series must be drawn with `spanGaps: true` on the consuming uPlot
 * instance — that's what lets each record's line draw through the positions left
 * `null` by every other record, rather than showing a break there.
 */
export function buildComparisonData(series: RecordTimeSeries[]): uPlot.AlignedData {
  const tables = series.map((s) => [s.xsSeconds, s.ys]) as unknown as uPlot.AlignedData[];
  // 0 = JoinNullMode.Remove, per uplot's d.ts ("use for series with spanGaps: true").
  // Not referenced as `uPlot.JoinNullMode.Remove`: it's a `const enum` with no
  // runtime object in the shipped build, and this project's tsconfig has
  // isolatedModules: true, which rejects const-enum value imports across modules.
  const nullModes: uPlot.JoinNullMode[][] = tables.map(() => [0]);
  return uPlot.join(tables, nullModes);
}
