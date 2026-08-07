<script lang="ts">
  import type { Session } from '../lib/types';
  import MultiRecordChart from './MultiRecordChart.svelte';

  let { sessions }: { sessions: Session[] } = $props();

  let selectedUnit = $state<string | null>(null);
  let included = $state<Set<string>>(new Set());
  let channelChoice = $state<Record<string, number>>({});

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

  function matchingChannels(s: Session): { index: number; id: string }[] {
    if (!selectedUnit) return [];
    const unit = selectedUnit;
    return s.metadata.analog_channels
      .map((ch, i) => ({ index: i, id: ch.id, units: ch.units || 'unitless' }))
      .filter((ch) => ch.units === unit)
      .map(({ index, id }) => ({ index, id }));
  }

  function disabledReasonFor(s: Session): string | null {
    if (s.metadata.start_epoch_us == null) {
      return "No absolute timestamp available in this record's .cfg — can't align on a synchronized time axis.";
    }
    if (matchingChannels(s).length === 0) {
      return `No ${selectedUnit} channels in this record.`;
    }
    return null;
  }

  function selectUnit(unit: string) {
    selectedUnit = unit;
    // Selections are per-unit, not stacked across units — keeps the "one axis per
    // chart" rule simple (v1 scope: compare one quantity at a time).
    included = new Set();
    channelChoice = {};
  }

  function toggleIncluded(s: Session) {
    const next = new Set(included);
    if (next.has(s.stem)) {
      next.delete(s.stem);
    } else {
      next.add(s.stem);
      if (channelChoice[s.stem] == null) {
        const first = matchingChannels(s)[0];
        if (first) channelChoice = { ...channelChoice, [s.stem]: first.index };
      }
    }
    included = next;
  }

  function setChannel(stem: string, e: Event) {
    const value = Number((e.currentTarget as HTMLSelectElement).value);
    channelChoice = { ...channelChoice, [stem]: value };
  }

  interface RecordSelection {
    stem: string;
    metadata: Session['metadata'];
    handle: Session['handle'];
    facts: Session['facts'];
    channelIndex: number;
  }

  const selectedRecords = $derived.by((): RecordSelection[] => {
    if (!selectedUnit) return [];
    return sessions
      .filter((s) => included.has(s.stem) && channelChoice[s.stem] != null)
      .map((s) => ({
        stem: s.stem,
        metadata: s.metadata,
        handle: s.handle,
        facts: s.facts,
        channelIndex: channelChoice[s.stem],
      }));
  });
</script>

<div class="compare-view">
  <section class="card">
    <h3>1. Pick a quantity to compare</h3>
    {#if availableUnits.length === 0}
      <p class="note">No loaded records have a parseable absolute timestamp to align on.</p>
    {:else}
      <div class="pills">
        {#each availableUnits as unit}
          <button class="pill" class:active={unit === selectedUnit} onclick={() => selectUnit(unit)}>{unit}</button>
        {/each}
      </div>
    {/if}
  </section>

  {#if selectedUnit}
    <section class="card">
      <h3>2. Pick records and a channel from each</h3>
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
            {:else if included.has(s.stem)}
              <select value={channelChoice[s.stem]} onchange={(e) => setChannel(s.stem, e)}>
                {#each matchingChannels(s) as ch}
                  <option value={ch.index}>{ch.id}</option>
                {/each}
              </select>
            {/if}
          </div>
        {/each}
      </div>
      {#if included.size > 4}
        <p class="note">Comparing more than about 4 records at once can be hard to read even with the legend.</p>
      {/if}
    </section>
  {/if}

  {#if selectedRecords.length}
    <section class="card">
      <h3>Comparison</h3>
      {#key selectedRecords.map((r) => `${r.stem}:${r.channelIndex}`).join(',')}
        <MultiRecordChart records={selectedRecords} units={selectedUnit as string} />
      {/key}
    </section>
  {/if}
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
  .session-row select {
    background: var(--page);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.2rem 0.5rem;
    color: var(--text-primary);
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
</style>
