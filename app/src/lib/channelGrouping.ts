import type { AnalogChannelDef } from './types';

export function unitOf(ch: { units?: string | null }): string {
  return ch.units || 'unitless';
}

// Groups analog-channel indices by engineering unit, in first-seen order — so a
// record whose channels are laid out V,V,V,A,A,A groups as [["V",[0,1,2]],["A",[3,4,5]]]
// rather than an unpredictable Set/Map-insertion order.
export function groupChannelsByUnit(channels: AnalogChannelDef[]): [string, number[]][] {
  const byUnit = new Map<string, number[]>();
  channels.forEach((ch, i) => {
    const key = unitOf(ch);
    const list = byUnit.get(key) ?? [];
    list.push(i);
    byUnit.set(key, list);
  });
  return Array.from(byUnit.entries());
}
