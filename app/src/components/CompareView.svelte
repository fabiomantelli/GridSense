<script lang="ts">
  import type { Session } from '../lib/types';
  import MultiRecordChart from './MultiRecordChart.svelte';

  let { sessions }: { sessions: Session[] } = $props();

  let selectedUnits = $state<Set<string>>(new Set());
  let included = $state<Set<string>>(new Set());
  // channelChoice[unit][stem] = channel index into that session's analog_channels —
  // namespaced by unit because the same record can supply more than one selected
  // quantity (e.g. VA for "V", IA for "A") through two different channels.
  let channelChoice = $state<Record<string, Record<string, number>>>({});
  // The x-window currently shown, shared across every quantity's chart. uPlot's
  // cursor.sync only broadcasts zoom/reset live between chart instances that are
  // already mounted — a quantity opened after another one was zoomed would
  // otherwise always start back at its own full range instead of the window the
  // operator is actually looking at. Each MultiRecordChart reports its window here
  // (its own zooms/resets, or one relayed from an already-open sibling); newly
  // opened ones are seeded from it.
  let sharedXRange = $state<[number, number] | null>(null);

  // Only sessions with a parsed absolute anchor can be honestly aligned — offering a
  // unit that zero eligible sessions have is a dead end, so don't surface it.
  const eligibleSessions = $derived(sessions.filter((s) => s.metadata.start_epoch_us != null));

  const availableUnits = $derived.by(() => {
    const set = new Set<string>();
    for (const s of eligibleSessions) {
      for (const ch of s.metadata.analog_channels) set.add(ch.units || 'unitless');
    }
    return Array.from(set);
  });

  // Selected units, in the same stable order they appear as pills — not selection
  // order, so quantity sections don't reshuffle as the user clicks pills.
  const selectedUnitsOrdered = $derived(availableUnits.filter((u) => selectedUnits.has(u)));

  function matchingChannels(unit: string, s: Session): { index: number; id: string }[] {
    return s.metadata.analog_channels
      .map((ch, i) => ({ index: i, id: ch.id, units: ch.units || 'unitless' }))
      .filter((ch) => ch.units === unit)
      .map(({ index, id }) => ({ index, id }));
  }

  // Record inclusion depends only on having an absolute timestamp to align on — not
  // on which quantities happen to be selected right now. That keeps this list's
  // shape (and every checkbox's position) fixed no matter how many quantity pills
  // are toggled, instead of the whole section reshaping on every click.
  function disabledReasonFor(s: Session): string | null {
    if (s.metadata.start_epoch_us == null) {
      return "No absolute timestamp available in this record's .cfg — can't align on a synchronized time axis.";
    }
    return null;
  }

  // Fills in a default (first-matching) channel for one unit on one session, without
  // clobbering a choice the user already made.
  function defaultChannel(unit: string, s: Session) {
    if (channelChoice[unit]?.[s.stem] != null) return;
    const first = matchingChannels(unit, s)[0];
    if (!first) return;
    channelChoice = { ...channelChoice, [unit]: { ...(channelChoice[unit] ?? {}), [s.stem]: first.index } };
  }

  function toggleUnit(unit: string) {
    const next = new Set(selectedUnits);
    if (next.has(unit)) {
      next.delete(unit);
    } else {
      next.add(unit);
      // A unit picked after records are already included needs its own default
      // channel on each of them — toggleIncluded only does this for units that
      // exist at the time a record is checked.
      for (const s of sessions) {
        if (included.has(s.stem)) defaultChannel(unit, s);
      }
    }
    selectedUnits = next;
  }

  function toggleIncluded(s: Session) {
    const next = new Set(included);
    if (next.has(s.stem)) {
      next.delete(s.stem);
    } else {
      next.add(s.stem);
      for (const unit of selectedUnitsOrdered) defaultChannel(unit, s);
    }
    included = next;
  }

  function setChannel(unit: string, stem: string, e: Event) {
    const value = Number((e.currentTarget as HTMLSelectElement).value);
    channelChoice = { ...channelChoice, [unit]: { ...(channelChoice[unit] ?? {}), [stem]: value } };
  }

  interface RecordSelection {
    stem: string;
    metadata: Session['metadata'];
    handle: Session['handle'];
    facts: Session['facts'];
    channelIndex: number;
  }

  // Included records that actually have a channel for this unit — records without
  // one just don't appear in this quantity's picker, rather than cluttering it with
  // a disabled "no channel" row.
  function eligibleForUnit(unit: string): Session[] {
    return sessions.filter((s) => included.has(s.stem) && matchingChannels(unit, s).length > 0);
  }

  function recordsForUnit(unit: string): RecordSelection[] {
    return eligibleForUnit(unit)
      .filter((s) => channelChoice[unit]?.[s.stem] != null)
      .map((s) => ({
        stem: s.stem,
        metadata: s.metadata,
        handle: s.handle,
        facts: s.facts,
        channelIndex: channelChoice[unit][s.stem],
      }));
  }
</script>

<div class="compare-view">
  <section class="card">
    <h3>1. Pick quantities to compare</h3>
    {#if availableUnits.length === 0}
      <p class="note">No loaded records have a parseable absolute timestamp to align on.</p>
    {:else}
      <div class="pills">
        {#each availableUnits as unit}
          <button class="pill" class:active={selectedUnits.has(unit)} onclick={() => toggleUnit(unit)}>{unit}</button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="card">
    <h3>2. Pick records to include</h3>
    <div class="session-list">
      {#each sessions as s (s.stem)}
        {@const reason = disabledReasonFor(s)}
        <div class="session-row" class:disabled={reason != null}>
          <label>
            <input type="checkbox" disabled={reason != null} checked={included.has(s.stem)} onchange={() => toggleIncluded(s)} />
            {s.stem}
          </label>
          {#if reason}
            <span class="reason">{reason}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if included.size > 4}
      <p class="note">Comparing more than about 4 records at once can be hard to read even with the legend.</p>
    {/if}
  </section>

  {#each selectedUnitsOrdered as unit (unit)}
    {@const eligible = eligibleForUnit(unit)}
    {@const unitRecords = recordsForUnit(unit)}
    <details class="chart-card" open>
      <summary>{unit}{#if unitRecords.length}&nbsp;— {unitRecords.length} record{unitRecords.length === 1 ? '' : 's'}{/if}</summary>
      <div class="content">
        {#if eligible.length === 0}
          <p class="note">None of the included records have a {unit} channel.</p>
        {:else}
          <div class="channel-picker">
            {#each eligible as s (s.stem)}
              {@const channels = matchingChannels(unit, s)}
              <label class="channel-row">
                <span class="stem">{s.stem}</span>
                <select value={channelChoice[unit]?.[s.stem]} onchange={(e) => setChannel(unit, s.stem, e)}>
                  {#each channels as ch}
                    <option value={ch.index}>{ch.id}</option>
                  {/each}
                </select>
              </label>
            {/each}
          </div>
          {#if unitRecords.length}
            {#key unitRecords.map((r) => `${r.stem}:${r.channelIndex}`).join(',')}
              <MultiRecordChart
                records={unitRecords}
                units={unit}
                initialRange={sharedXRange}
                onRangeChange={(r) => (sharedXRange = r)}
              />
            {/key}
          {/if}
        {/if}
      </div>
    </details>
  {/each}
</div>

<style>
  .compare-view {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1rem;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 1rem;
    box-shadow: var(--shadow-card);
  }
  .card h3 {
    margin: 0 0 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .pills {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .pill {
    background: var(--page);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    padding: 0.35rem 0.9rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .pill.active {
    border-color: var(--series-1);
    background: color-mix(in srgb, var(--series-1) 12%, var(--surface));
    color: var(--text-primary);
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .session-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.88rem;
  }
  .session-row.disabled {
    color: var(--text-muted);
  }
  .session-row label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 12rem;
  }
  .reason {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .note {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin: 0.5rem 0 0;
  }

  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card);
  }
  .chart-card summary {
    cursor: pointer;
    padding: 0.7rem 1rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    user-select: none;
  }
  .chart-card summary::marker {
    color: var(--text-muted);
  }
  .chart-card .content {
    padding: 0.9rem 1rem 1rem;
    border-top: 1px solid var(--border);
  }
  .chart-card .note {
    margin: 0;
  }

  .channel-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem 1.25rem;
    margin-bottom: 0.9rem;
  }
  .channel-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .channel-row .stem {
    color: var(--text-secondary);
  }
  .channel-row select {
    background: var(--page);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.2rem 0.5rem;
    color: var(--text-primary);
  }
</style>
