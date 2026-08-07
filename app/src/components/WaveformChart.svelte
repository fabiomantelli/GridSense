<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';

  let { metadata, handle }: { metadata: CfgFile; handle: ComtradeHandle } = $props();

  const PALETTE = ['#4a9eff', '#ff6b6b', '#51cf66', '#ffa94d', '#cc5de8', '#20c997', '#f06595', '#94d82d'];

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
          stroke: PALETTE[si % PALETTE.length],
          width: 1.5,
        })),
      ];
      const opts: uPlot.Options = {
        width: el.clientWidth || 800,
        height: 220,
        title: units,
        scales: { x: { time: false } },
        axes: [{ label: 't (ms)' }, { label: units }],
        series,
        // Shared sync key: dragging/zooming one unit-group's plot moves the cursor
        // (and, via match, the zoom) on the others in lockstep.
        cursor: { sync: { key: 'gridsense-waveform', setSeries: true } },
      };
      return new uPlot(opts, data, el);
    });
  }

  function destroyPlots() {
    for (const p of plots) p.destroy();
    plots = [];
  }

  onMount(buildPlots);
  onDestroy(destroyPlots);
</script>

{#if groups.length}
  <div class="charts">
    {#each groups as [units], gi (units)}
      <div class="chart-container" bind:this={containers[gi]}></div>
    {/each}
  </div>
{:else}
  <p class="note">No analog channels in this record.</p>
{/if}

<style>
  .charts {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .chart-container {
    width: 100%;
  }
  .note {
    opacity: 0.7;
    font-size: 0.9rem;
  }
</style>
