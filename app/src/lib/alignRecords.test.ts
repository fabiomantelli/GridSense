import { describe, expect, it } from 'vitest';
import { buildComparisonData } from './alignRecords';

describe('buildComparisonData', () => {
  it('merges two disjoint-x series onto one sorted, deduplicated x-axis', () => {
    const a = { xsSeconds: new Float64Array([0, 1, 2]), ys: new Float32Array([10, 11, 12]) };
    const b = { xsSeconds: new Float64Array([0.5, 1.5]), ys: new Float32Array([20, 21]) };

    const data = buildComparisonData([a, b]);

    // uPlot's join() with JoinNullMode.Remove (0) fills non-matching positions with
    // `undefined` (sparse-array holes), not literal `null`, despite the mode's doc
    // comment describing the semantic concept as "null gaps" — confirmed by running
    // this test, not assumed. `spanGaps: true` on the consuming chart draws through
    // either, and this codebase's `!= null` convention already treats both the same.
    expect(Array.from(data[0])).toEqual([0, 0.5, 1, 1.5, 2]);
    expect(Array.from(data[1])).toEqual([10, undefined, 11, undefined, 12]);
    expect(Array.from(data[2])).toEqual([undefined, 20, undefined, 21, undefined]);
  });

  it('deduplicates a shared x position across records', () => {
    const a = { xsSeconds: new Float64Array([0, 1]), ys: new Float32Array([10, 11]) };
    const b = { xsSeconds: new Float64Array([1, 2]), ys: new Float32Array([21, 22]) };

    const data = buildComparisonData([a, b]);

    expect(Array.from(data[0])).toEqual([0, 1, 2]);
    expect(Array.from(data[1])).toEqual([10, 11, undefined]);
    expect(Array.from(data[2])).toEqual([undefined, 21, 22]);
  });

  it('handles a single record (identity case)', () => {
    const a = { xsSeconds: new Float64Array([0, 1, 2]), ys: new Float32Array([1, 2, 3]) };
    const data = buildComparisonData([a]);
    expect(Array.from(data[0])).toEqual([0, 1, 2]);
    expect(Array.from(data[1])).toEqual([1, 2, 3]);
  });
});
