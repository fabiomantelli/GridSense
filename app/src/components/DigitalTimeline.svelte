<script lang="ts">
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';
  import { resolveChartTheme } from '../lib/theme';

  let { metadata, handle }: { metadata: CfgFile; handle: ComtradeHandle } = $props();

  const theme = resolveChartTheme();

  const WIDTH = 800;
  const ROW_HEIGHT = 28;
  const LABEL_WIDTH = 190;
  const PLOT_WIDTH = WIDTH - LABEL_WIDTH;
  const FONT_SIZE = 12;
  // Rough average glyph width for this sans-serif at FONT_SIZE, used only to decide
  // when to truncate — doesn't need to be exact, just conservative enough that a
  // truncated label plus "…" reliably fits before the timeline bars start.
  const MAX_LABEL_CHARS = Math.floor((LABEL_WIDTH - 8) / (FONT_SIZE * 0.58));

  function truncateLabel(label: string): string {
    return label.length > MAX_LABEL_CHARS ? `${label.slice(0, MAX_LABEL_CHARS - 1).trimEnd()}…` : label;
  }

  interface Segment {
    startMs: number;
    endMs: number;
    state: boolean;
  }

  // Session components are keyed by stem and never rebound to a different handle;
  // $derived here is only to satisfy Svelte's reactivity contract for prop reads.
  const timestampsMs: number[] = $derived(Array.from(handle.timestamps_f64(), (v) => v / 1000));
  const t0 = $derived(timestampsMs[0] ?? 0);
  const totalDurationMs = $derived(timestampsMs.length > 1 ? timestampsMs[timestampsMs.length - 1] - t0 : 1);

  function toSegments(values: Uint8Array): Segment[] {
    const segments: Segment[] = [];
    if (values.length === 0) return segments;
    let segStart = timestampsMs[0];
    let state = values[0] === 1;
    for (let i = 1; i < values.length; i++) {
      const s = values[i] === 1;
      if (s !== state) {
        segments.push({ startMs: segStart, endMs: timestampsMs[i], state });
        segStart = timestampsMs[i];
        state = s;
      }
    }
    segments.push({ startMs: segStart, endMs: timestampsMs[timestampsMs.length - 1], state });
    return segments;
  }

  const channels = $derived(
    metadata.digital_channels.map((def, i) => ({
      def,
      segments: toSegments(handle.digital_channel_bools(i)),
    })),
  );

  function xPos(ms: number): number {
    if (totalDurationMs <= 0) return 0;
    return ((ms - t0) / totalDurationMs) * PLOT_WIDTH;
  }
</script>

{#if channels.length}
  <details class="timeline-card">
    <summary>Digital channels — {channels.length} canais</summary>
    <div class="content">
      <svg
        viewBox="0 0 {WIDTH} {channels.length * ROW_HEIGHT + 4}"
        class="digital-timeline"
        role="img"
        aria-label="Digital channel state timeline"
      >
        <!-- Bars first, labels last: labels must always paint on top so a long
             channel name is never hidden behind a bar it happens to overlap. -->
        {#each channels as ch, row}
          {#each ch.segments as seg}
            <rect
              x={LABEL_WIDTH + xPos(seg.startMs)}
              y={row * ROW_HEIGHT + 4}
              width={Math.max(1, xPos(seg.endMs) - xPos(seg.startMs))}
              height={ROW_HEIGHT - 8}
              rx="2"
              fill={seg.state ? theme.series[0] : theme.grid}
            />
          {/each}
        {/each}
        {#each channels as ch, row}
          <text x="0" y={row * ROW_HEIGHT + ROW_HEIGHT / 2 + 4} font-size={FONT_SIZE} fill={theme.text}>
            {truncateLabel(ch.def.id)}
            {#if ch.def.id.length > MAX_LABEL_CHARS}
              <title>{ch.def.id}</title>
            {/if}
          </text>
        {/each}
      </svg>
      <p class="legend">
        <span class="swatch on" style:background={theme.series[0]}></span> 1 (energizado/fechado)
        <span class="swatch off" style:background={theme.grid}></span> 0 (desenergizado/aberto)
      </p>
    </div>
  </details>
{:else}
  <p class="note">No digital channels in this record.</p>
{/if}

<style>
  .timeline-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card);
  }
  .timeline-card summary {
    cursor: pointer;
    padding: 0.7rem 1rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    user-select: none;
  }
  .timeline-card summary::marker {
    color: var(--text-muted);
  }
  .content {
    padding: 0.75rem;
    border-top: 1px solid var(--border);
  }
  .digital-timeline {
    width: 100%;
    height: auto;
  }
  .legend {
    font-size: 0.78rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin: 0.6rem 0 0;
  }
  .swatch {
    display: inline-block;
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 2px;
    margin-left: 0.5rem;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.9rem;
  }
</style>
