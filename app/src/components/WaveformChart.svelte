<script lang="ts">
  import type { AnalysisFacts, CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';
  import { resolveChartTheme } from '../lib/theme';
  import { groupChannelsByUnit } from '../lib/channelGrouping';
  import ChannelLane from './ChannelLane.svelte';

  let { metadata, handle, facts }: { metadata: CfgFile; handle: ComtradeHandle; facts: AnalysisFacts } = $props();

  const theme = resolveChartTheme();

  // Groups analog channels by engineering unit, same grouping WaveformChart always
  // used — now driving a stack of compact per-channel lanes within each group's
  // section instead of one big overlaid chart per unit. $derived here is only to
  // satisfy Svelte's reactivity contract for prop reads — this component is keyed
  // by stem and never rebound to a different handle/metadata (see App.svelte).
  const groups: [string, number[]][] = $derived(groupChannelsByUnit(metadata.analog_channels));

  const timestampsMs: Float64Array = (() => {
    const us = handle.timestamps_f64();
    const ms = new Float64Array(us.length);
    for (let i = 0; i < us.length; i++) ms[i] = us[i] / 1000;
    return ms;
  })();
  const fullXRange: [number, number] = [timestampsMs[0] ?? 0, timestampsMs[timestampsMs.length - 1] ?? 1];

  // Same event markers every lane draws — computed once and handed down, not
  // recomputed per lane.
  const onsetMarkersMs = $derived(facts.events.map((e) => e.onset_time_us / 1000));
  const tripMarkersMs = $derived(
    facts.events.filter((e) => e.time_to_trip_us != null).map((e) => (e.onset_time_us + (e.time_to_trip_us as number)) / 1000),
  );

  // Shared Y-range per unit-group, computed once from every channel *in the
  // group* — not just the currently-visible ones — so toggling one channel's
  // visibility never shifts the scale its still-visible siblings depend on for
  // cross-phase magnitude comparison.
  function computeGroupRange(indices: number[]): [number, number] {
    let min = Infinity;
    let max = -Infinity;
    for (const idx of indices) {
      const ys = handle.analog_channel_f32(idx);
      for (let i = 0; i < ys.length; i++) {
        const v = ys[i];
        if (v < min) min = v;
        if (v > max) max = v;
      }
    }
    if (!isFinite(min) || !isFinite(max)) return [-1, 1];
    if (min === max) return [min - 1, max + 1];
    const pad = (max - min) * 0.05;
    return [min - pad, max + pad];
  }
  const groupRanges: [number, number][] = $derived(groups.map(([, indices]) => computeGroupRange(indices)));

  function rmsFor(idx: number): number | null {
    return facts.channel_summaries[idx]?.rms ?? null;
  }

  // Every channel visible by default. Keyed by channel index (not per-group), so
  // each channel keeps a stable identity/color slot regardless of which group's
  // section it renders in. Deliberately a one-time seed from `metadata`, not a
  // derived value — this is mutable UI state the user toggles afterward, and
  // (same as `groups` above) this component is never rebound to different
  // metadata, so "only captures the initial value" is exactly the intent.
  let visible = $state<Set<number>>(new Set(metadata.analog_channels.map((_, i) => i)));

  function toggleChannel(idx: number) {
    const next = new Set(visible);
    if (next.has(idx)) next.delete(idx);
    else next.add(idx);
    visible = next;
  }
  function selectAllInGroup(indices: number[]) {
    const next = new Set(visible);
    for (const idx of indices) next.add(idx);
    visible = next;
  }
  function clearGroup(indices: number[]) {
    const next = new Set(visible);
    for (const idx of indices) next.delete(idx);
    visible = next;
  }
  function visibleInGroup(indices: number[]): number[] {
    return indices.filter((idx) => visible.has(idx));
  }

  // Not reactive state — plain refs collected as lanes mount, read only when the
  // user clicks "Reset zoom". A lane's ChannelLane instance is swapped out (see the
  // {#each} key below) whenever its own showXAxis flips, which naturally
  // reassigns/clears this via bind:this.
  let laneRefsByIndex: Record<number, ChannelLane | undefined> = {};

  function resetZoom() {
    for (const lane of Object.values(laneRefsByIndex)) lane?.resetZoom(fullXRange[0], fullXRange[1]);
  }

  // At most one group expanded to fullscreen at a time — index into `groups`.
  // Per-card (not a single page-level toggle covering every group): a real
  // multi-source file can have several unit groups (V, A, Hz, ...), and a
  // shared toolbar button sitting above the first card both looked like it
  // belonged only to that card AND, when clicked, unexpectedly expanded every
  // other group along with it. Each card owning its own toggle — the same
  // pattern MultiRecordChart/CompareView already uses — makes the scope
  // unambiguous: the button you click is the card that expands.
  let expandedGroupIndex = $state<number | null>(null);
  function toggleExpand(gi: number) {
    expandedGroupIndex = expandedGroupIndex === gi ? null : gi;
  }

  // While a card is expanded it behaves like a lightbox: Esc closes it and the
  // page behind it stops scrolling so the two scroll contexts don't fight.
  $effect(() => {
    if (expandedGroupIndex == null) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    function onKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') expandedGroupIndex = null;
    }
    window.addEventListener('keydown', onKeydown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', onKeydown);
    };
  });
</script>

{#snippet expandIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M8 3H5a2 2 0 00-2 2v3M16 3h3a2 2 0 012 2v3M21 16v3a2 2 0 01-2 2h-3M8 21H5a2 2 0 01-2-2v-3" />
  </svg>
{/snippet}
{#snippet shrinkIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M9 3v3a2 2 0 01-2 2H4M15 3v3a2 2 0 002 2h3M21 15h-3a2 2 0 00-2 2v3M3 15h3a2 2 0 012 2v3" />
  </svg>
{/snippet}

{#if groups.length}
  <div class="toolbar">
    <button class="reset-zoom" onclick={resetZoom}>Reset zoom</button>
    <span class="hint">drag to zoom · double-click to reset</span>
  </div>
  {#if expandedGroupIndex != null}
    <button class="backdrop" onclick={() => (expandedGroupIndex = null)} aria-label="Exit fullscreen"></button>
  {/if}
  <div class="charts">
    {#each groups as [units, indices], gi (units)}
      {@const visIdx = visibleInGroup(indices)}
      {@const lastVisible = visIdx[visIdx.length - 1]}
      {@const isExpanded = expandedGroupIndex === gi}
      <details class="chart-card" class:expanded={isExpanded} open>
        <summary>
          <span class="summary-row">
            <span>{units} — {visIdx.length} of {indices.length} channel{indices.length === 1 ? '' : 's'} shown</span>
            <span class="bulk-actions">
              {#if isExpanded}
                <span class="hint">Esc or click outside to exit</span>
              {/if}
              {#if indices.length > 1}
                <button
                  class="link-button"
                  onclick={(e) => {
                    // preventDefault, not just stopPropagation: a click inside
                    // <summary> runs the browser's native open/close toggle as
                    // a default action gated on event.defaultPrevented, not on
                    // whether propagation was stopped — stopPropagation alone
                    // still let this button's click silently collapse the card
                    // out from under the click.
                    e.preventDefault();
                    e.stopPropagation();
                    selectAllInGroup(indices);
                  }}
                  disabled={visIdx.length === indices.length}
                >
                  Select all
                </button>
                <button
                  class="link-button"
                  onclick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    clearGroup(indices);
                  }}
                  disabled={visIdx.length === 0}
                >
                  Clear
                </button>
              {/if}
              <button
                class="expand-toggle"
                onclick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  toggleExpand(gi);
                }}
                aria-label={isExpanded ? 'Exit fullscreen' : `Expand ${units} chart`}
              >
                {@render (isExpanded ? shrinkIcon : expandIcon)()}
              </button>
            </span>
          </span>
        </summary>
        <div class="content">
          <div class="channel-picker">
            {#each indices as idx (idx)}
              {@const rms = rmsFor(idx)}
              <label class="channel-row">
                <input type="checkbox" checked={visible.has(idx)} onchange={() => toggleChannel(idx)} />
                <span class="swatch" style:background={theme.series[indices.indexOf(idx) % theme.series.length]}></span>
                <span class="chan-id">{metadata.analog_channels[idx].id}</span>
                {#if rms != null}
                  <span class="chan-rms">RMS {rms.toFixed(3)}</span>
                {/if}
              </label>
            {/each}
          </div>
          {#if visIdx.length === 0}
            <p class="note">No channels selected in this group.</p>
          {:else}
            <div class="lanes">
              {#each visIdx as idx (idx + ':' + (idx === lastVisible))}
                <ChannelLane
                  bind:this={laneRefsByIndex[idx]}
                  label={metadata.analog_channels[idx].id}
                  color={theme.series[indices.indexOf(idx) % theme.series.length]}
                  channelIndex={idx}
                  {handle}
                  {timestampsMs}
                  yRange={groupRanges[gi]}
                  showXAxis={idx === lastVisible}
                  {onsetMarkersMs}
                  {tripMarkersMs}
                  {units}
                  expanded={isExpanded}
                />
              {/each}
            </div>
          {/if}
        </div>
      </details>
    {/each}
  </div>
{:else}
  <p class="note">No analog channels in this record.</p>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.6rem;
  }
  .reset-zoom {
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.3rem 0.7rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .reset-zoom:hover {
    border-color: var(--series-1);
    color: var(--series-1);
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .expand-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.3rem;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .expand-toggle:hover {
    border-color: var(--series-1);
    color: var(--series-1);
  }
  .expand-toggle svg {
    width: 16px;
    height: 16px;
    display: block;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
    background: rgba(0, 0, 0, 0.6);
    border: none;
    padding: 0;
    cursor: default;
  }
  .charts {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card);
  }
  /* Expanded state deliberately does NOT use display:flex/grid directly on
     <details> to size summary vs. .content — confirmed via a live A/B test
     (identical flex CSS applied to a plain sibling <div> vs. to this actual
     <details> element) that Chromium does not constrain a flex/grid item's
     height inside <details> the way it does for an ordinary element: .content
     kept sizing to its full unconstrained content height and silently
     overflowed past the card's own box (clipped by overflow:hidden, so lanes
     at the bottom were cut off with no way to scroll to them — the "corta
     alguns gráficos" bug). Both children are independently position:fixed
     instead, stacked via a hardcoded summary height, sidestepping <details>'s
     layout entirely. */
  .chart-card.expanded {
    position: fixed;
    inset: 2rem;
    z-index: 100;
    overflow: hidden;
  }
  .chart-card.expanded summary {
    position: fixed;
    top: 2rem;
    left: 2rem;
    right: 2rem;
    height: 3.25rem;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    overflow: hidden;
    z-index: 101;
  }
  .chart-card.expanded .content {
    position: fixed;
    top: calc(2rem + 3.25rem);
    left: 2rem;
    right: 2rem;
    bottom: 2rem;
    box-sizing: border-box;
    /* A plain <div>, not <details>, so flex sizing here is reliable (see the
       comment above on why <details> itself can't be trusted with this).
       The channel picker keeps its natural height; .lanes gets whatever's
       left and does its own scrolling — moving scroll here instead of onto
       this whole box keeps the picker on-screen and clickable no matter how
       far down the lane stack you've scrolled. */
    display: flex;
    flex-direction: column;
    overflow: hidden;
    z-index: 100;
  }
  .chart-card.expanded .lanes {
    /* Lets a group with few channels grow to fill the leftover vertical
       space instead of leaving it empty (each ChannelLane's own min-height
       floor — see its `expanded` prop — keeps a many-channel group exactly
       as compact as before, still scrolling here when it doesn't fit). */
    flex: 1;
    min-height: 0;
    overflow: auto;
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
    padding: 0.75rem 1rem 1rem;
    border-top: 1px solid var(--border);
  }
  .summary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    width: 100%;
  }
  .bulk-actions {
    display: flex;
    align-items: center;
    gap: 0.9rem;
  }
  .link-button {
    background: none;
    border: none;
    padding: 0;
    font-size: 0.78rem;
    color: var(--text-secondary);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .link-button:hover:not(:disabled) {
    color: var(--series-1);
  }
  .link-button:disabled {
    color: var(--text-muted);
    cursor: default;
    text-decoration: none;
  }
  .channel-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1.1rem;
    margin-bottom: 0.75rem;
  }
  .channel-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }
  .swatch {
    display: inline-block;
    width: 0.65rem;
    height: 0.65rem;
    border-radius: 2px;
    flex: none;
  }
  .chan-id {
    color: var(--text-primary);
  }
  .chan-rms {
    color: var(--text-muted);
    font-size: 0.76rem;
  }
  .lanes {
    display: flex;
    flex-direction: column;
    /* The gap shows the recessive page color through it — a sliver of real
       space, not a stroke — so lanes read as separate rows without the
       cramped, glued-together look a 1px hairline gave a dozen-plus stacked
       lanes. */
    gap: 3px;
    padding: 3px;
    background: var(--page);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .chart-card .note {
    margin: 0;
  }
</style>
