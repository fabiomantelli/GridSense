<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';
  import { resolveChartTheme } from '../lib/theme';

  let { metadata, handle }: { metadata: CfgFile; handle: ComtradeHandle } = $props();

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

  let containers = $state<HTMLDivElement[]>([]);
  let plots: uPlot[] = [];

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
        // Shared sync key: dragging/zooming one unit-group's plot moves the cursor
        // (and, via match, the zoom) on the others in lockstep.
        cursor: { sync: { key: 'gridsense-waveform', setSeries: true } },
      };
      const plot = new uPlot(opts, data, el);
      styleLegend(plot);
      return plot;
    });
  }

  // uPlot's built-in legend is functional but visually generic; restyle it to match
  // the app's tokens rather than fighting uPlot's own CSS with !important overrides.
  function styleLegend(plot: uPlot) {
    const legend = plot.root.querySelector('.u-legend') as HTMLElement | null;
    if (!legend) return;
    legend.style.fontSize = '0.8rem';
    legend.style.color = 'var(--text-secondary)';
  }

  function destroyPlots() {
    for (const p of plots) p.destroy();
    plots = [];
  }

  onMount(buildPlots);
  onDestroy(destroyPlots);
</script>

{#if groups.length}
  <div class="charts" style:--u-surface={theme.surface} style:--u-baseline={theme.baseline}>
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

  /* uPlot chrome: axis/legend text and title inherit the app's ink tokens (canvas
     drawing is themed separately via lib/theme.ts, since canvas can't read CSS
     variables). */
  .charts :global(.u-title) {
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .charts :global(.u-legend th) {
    color: var(--text-secondary);
    font-weight: 400;
  }
</style>
