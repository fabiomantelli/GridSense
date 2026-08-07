<script lang="ts">
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';

  let { metadata, handle }: { metadata: CfgFile; handle: ComtradeHandle } = $props();

  const WIDTH = 800;
  const ROW_HEIGHT = 26;
  const LABEL_WIDTH = 130;
  const PLOT_WIDTH = WIDTH - LABEL_WIDTH;

  interface Segment {
    startMs: number;
    endMs: number;
    state: boolean;
  }

  // Session components are keyed by stem and never rebound to a different handle, so
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
  <svg viewBox="0 0 {WIDTH} {channels.length * ROW_HEIGHT + 4}" class="digital-timeline" role="img"
    aria-label="Digital channel state timeline">
    {#each channels as ch, row}
      <text x="0" y={row * ROW_HEIGHT + ROW_HEIGHT / 2 + 4} font-size="12" fill="currentColor">{ch.def.id}</text>
      {#each ch.segments as seg}
        <rect
          x={LABEL_WIDTH + xPos(seg.startMs)}
          y={row * ROW_HEIGHT + 3}
          width={Math.max(1, xPos(seg.endMs) - xPos(seg.startMs))}
          height={ROW_HEIGHT - 6}
          fill={seg.state ? '#51cf66' : '#495057'}
        />
      {/each}
    {/each}
  </svg>
  <p class="legend">
    <span class="swatch on"></span> closed/energized (1)
    <span class="swatch off"></span> open/de-energized (0)
  </p>
{:else}
  <p class="note">No digital channels in this record.</p>
{/if}

<style>
  .digital-timeline {
    width: 100%;
    height: auto;
  }
  .legend {
    font-size: 0.8rem;
    opacity: 0.75;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .swatch {
    display: inline-block;
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 2px;
    margin-left: 0.5rem;
  }
  .swatch.on {
    background: #51cf66;
  }
  .swatch.off {
    background: #495057;
  }
  .note {
    opacity: 0.7;
    font-size: 0.9rem;
  }
</style>
