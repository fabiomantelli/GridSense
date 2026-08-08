import { describe, expect, it } from 'vitest';
import { groupChannelsByUnit, unitOf } from './channelGrouping';
import type { AnalogChannelDef } from './types';

function ch(id: string, units: string | null): AnalogChannelDef {
  return {
    index: 0,
    id,
    phase: null,
    circuit_component: null,
    units: units ?? '',
    a: 1,
    b: 0,
    skew: 0,
    min: -1,
    max: 1,
    primary: 1,
    secondary: 1,
    ps: 'P',
  };
}

describe('unitOf', () => {
  it('falls back to "unitless" for empty/missing units', () => {
    expect(unitOf({ units: '' })).toBe('unitless');
    expect(unitOf({ units: null })).toBe('unitless');
    expect(unitOf({})).toBe('unitless');
  });

  it('passes through a real unit unchanged', () => {
    expect(unitOf({ units: 'V' })).toBe('V');
  });
});

describe('groupChannelsByUnit', () => {
  it('groups channel indices by unit, in first-seen unit order', () => {
    const channels = [ch('VA', 'V'), ch('VB', 'V'), ch('VC', 'V'), ch('IA', 'A'), ch('IB', 'A'), ch('IC', 'A')];

    const groups = groupChannelsByUnit(channels);

    expect(groups).toEqual([
      ['V', [0, 1, 2]],
      ['A', [3, 4, 5]],
    ]);
  });

  it('interleaved units still group by unit, not position', () => {
    const channels = [ch('VA', 'V'), ch('IA', 'A'), ch('VB', 'V'), ch('IB', 'A')];

    const groups = groupChannelsByUnit(channels);

    expect(groups).toEqual([
      ['V', [0, 2]],
      ['A', [1, 3]],
    ]);
  });

  it('treats missing units as a single shared "unitless" group', () => {
    const channels = [ch('X1', null), ch('X2', '')];

    const groups = groupChannelsByUnit(channels);

    expect(groups).toEqual([['unitless', [0, 1]]]);
  });

  it('returns an empty list for no channels', () => {
    expect(groupChannelsByUnit([])).toEqual([]);
  });
});
