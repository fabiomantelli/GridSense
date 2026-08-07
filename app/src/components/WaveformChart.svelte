<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { AnalysisFacts, CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';
  import { resolveChartTheme } from '../lib/theme';

  let { metadata, handle, facts }: { metadata: CfgFile; handle: ComtradeHandle; facts: AnalysisFacts } = $props();

  const theme = resolveChartTheme();

  // Group analog channels by engineering unit so each stacked plot shares a
  // meaningful Y-axis (e.g. all voltages together, all currents together).
  const groups: [string, number[]][] = (() => {
    const byUnit = new Map<string, number[]>();
    metadata.analog_channels.forEach((ch, i) => {
      const key = ch.units || 'unitless';
      const list = byUnit.get(key) ?? [];
      list.push(i);
      byUnit.set(key, list);
    });
    return Array.from(byUnit.entries());
  })();

  const timestampsMs: Float64Array = (() => {
    const us = handle.timestamps_f64();
    const ms = new Float64Array(us.length);
    for (let i = 0; i < us.length; i++) ms[i] = us[i] / 1000;
    return ms;
  })();
  const fullXRange: [number, number] = [timestampsMs[0] ?? 0, timestampsMs[timestampsMs.length - 1] ?? 1];

  // Vertical reference lines: solid at each detected event's onset, dashed at the
  // correlated breaker trip (if any) — the chart shows exactly what the facts panel
  // already claims, not a separate/unverifiable rendering.
  const onsetMarkersMs = $derived(facts.events.map((e) => e.onset_time_us / 1000));
  const tripMarkersMs = $derived(
    facts.events.filter((e) => e.time_to_trip_us != null).map((e) => (e.onset_time_us + (e.time_to_trip_us as number)) / 1000),
  );

  let containers = $state<HTMLDivElement[]>([]);
  let plots: uPlot[] = [];

  function drawMarkers(u: uPlot) {
    const ctx = u.ctx;
    ctx.save();
    ctx.lineWidth = 1.5;
    const draw = (ms: number, color: string, dashed: boolean) => {
      const x = u.valToPos(ms, 'x', true);
      if (x < u.bbox.left || x > u.bbox.left + u.bbox.width) return;
      ctx.strokeStyle = color;
      ctx.setLineDash(dashed ? [5, 4] : []);
      ctx.beginPath();
      ctx.moveTo(x, u.bbox.top);
      ctx.lineTo(x, u.bbox.top + u.bbox.height);
      ctx.stroke();
    };
    for (const ms of onsetMarkersMs) draw(ms, theme.markerCritical, false);
    for (const ms of tripMarkersMs) draw(ms, theme.markerNeutral, true);
    ctx.restore();
  }

  function buildPlots() {
    plots = groups.map(([units, indices], gi) => {
      const el = containers[gi];

      const data = [timestampsMs, ...indices.map((idx) => handle.analog_channel_f32(idx))] as unknown as uPlot.AlignedData;
      const series: uPlot.Series[] = [
        {},
        ...indices.map((idx, si) => ({
          label: metadata.analog_channels[idx].id,
          stroke: theme.series[si % theme.series.length],
          width: 2,
        })),
      ];
      const axisCommon = {
        stroke: theme.text,
        grid: { stroke: theme.grid, width: 1 },
        ticks: { stroke: theme.grid, width: 1 },
        font: '12px system-ui, -apple-system, sans-serif',
      };
      const opts: uPlot.Options = {
        width: el.clientWidth || 800,
        height: 220,
        title: units,
        scales: { x: { time: false } },
        axes: [
          { ...axisCommon, label: 't (ms)' },
          { ...axisCommon, label: units, size: 56 },
        ],
        series,
        legend: { show: true },
        cursor: {
          // scales:['x',null] propagates zoom/pan (not just the crosshair) to every
          // synced plot, so dragging to zoom the voltage plot zooms current too.
          sync: { key: 'gridsense-waveform', setSeries: true, scales: ['x', null] },
          drag: { x: true, y: false },
        },
        hooks: {
          draw: [drawMarkers],
          setSelect: [
            (u) => {
              if (u.select.width > 4) {
                const min = u.posToVal(u.select.left, 'x');
                const max = u.posToVal(u.select.left + u.select.width, 'x');
                u.setScale('x', { min, max });
              }
              u.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
            },
          ],
        },
      };
      const plot = new uPlot(opts, data, el);
      el.ondblclick = () => resetZoom();
      return plot;
    });
  }

  function resetZoom() {
    for (const p of plots) {
      p.setScale('x', { min: fullXRange[0], max: fullXRange[1] });
    }
  }

  function destroyPlots() {
    for (const p of plots) p.destroy();
    plots = [];
  }

  onMount(buildPlots);
  onDestroy(destroyPlots);
</script>

{#if groups.length}
  <div class="toolbar">
    <button class="reset-zoom" onclick={resetZoom}>Reset zoom</button>
    <span class="hint">arraste para dar zoom · duplo clique para resetar</span>
  </div>
  <div class="charts">
    {#each groups as [units], gi (units)}
      <div class="chart-card">
        <div class="chart-container" bind:this={containers[gi]}></div>
      </div>
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
  .charts {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 0.75rem;
    box-shadow: var(--shadow-card);
  }
  .chart-container {
    width: 100%;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  /* uPlot chrome: title/legend text inherits the app's ink tokens (canvas drawing —
     axes, grid, event markers — is themed separately via lib/theme.ts, since canvas
     can't read CSS variables). */
  .charts :global(.u-title) {
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .charts :global(.u-legend th) {
    color: var(--text-secondary);
    font-weight: 400;
    font-size: 0.78rem;
  }
  .charts :global(.u-select) {
    background: color-mix(in srgb, var(--series-1) 15%, transparent);
    border: 1px solid var(--series-1);
  }
</style>
