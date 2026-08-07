<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { AnalysisFacts, CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';
  import { resolveChartTheme } from '../lib/theme';
  import { buildComparisonData } from '../lib/alignRecords';

  interface RecordSelection {
    stem: string;
    metadata: CfgFile;
    handle: ComtradeHandle;
    facts: AnalysisFacts;
    channelIndex: number;
  }

  // Assumes the parent only ever passes records with a non-null start_epoch_us
  // (CompareView disables selection otherwise) and a fixed selection for this
  // component's lifetime — CompareView remounts this component (via a {#key}) when
  // the selection changes, matching WaveformChart's "build once per mount" pattern
  // rather than reactively patching an existing uPlot instance's series list.
  let { records, units }: { records: RecordSelection[]; units: string } = $props();

  const theme = resolveChartTheme();

  function toSeconds(metadata: CfgFile, timestampsUs: Float64Array): Float64Array {
    const start = metadata.start_epoch_us as number;
    const out = new Float64Array(timestampsUs.length);
    for (let i = 0; i < timestampsUs.length; i++) out[i] = (start + timestampsUs[i]) / 1_000_000;
    return out;
  }

  // Session components are keyed by selection (see CompareView's {#key}) and never
  // rebound to a different `records` array; $derived here is only to satisfy
  // Svelte's reactivity contract for prop reads.
  const data: uPlot.AlignedData = $derived.by(() =>
    buildComparisonData(
      records.map((r) => ({
        xsSeconds: toSeconds(r.metadata, r.handle.timestamps_f64()),
        ys: r.handle.analog_channel_f32(r.channelIndex),
      })),
    ),
  );

  const fullXRange: [number, number] = (() => {
    const xs = data[0] as number[] | Float64Array;
    return [xs[0] ?? 0, xs[xs.length - 1] ?? 1];
  })();

  // Each record's own already-computed fault-onset marker, in that record's color —
  // shows which station's relay saw the disturbance first. No breaker-trip (dashed)
  // markers here (v1 scope cut): N colors x 2 dash styles gets noisy fast, and each
  // record's own trip correlation is still visible in its single-record tab.
  function drawMarkers(u: uPlot) {
    const ctx = u.ctx;
    ctx.save();
    ctx.lineWidth = 1.5;
    ctx.setLineDash([]);
    records.forEach((r, i) => {
      const start = r.metadata.start_epoch_us as number;
      ctx.strokeStyle = theme.series[i % theme.series.length];
      for (const e of r.facts.events) {
        const sec = (start + e.onset_time_us) / 1_000_000;
        const x = u.valToPos(sec, 'x', true);
        if (x < u.bbox.left || x > u.bbox.left + u.bbox.width) continue;
        ctx.beginPath();
        ctx.moveTo(x, u.bbox.top);
        ctx.lineTo(x, u.bbox.top + u.bbox.height);
        ctx.stroke();
      }
    });
    ctx.restore();
  }

  let container = $state<HTMLDivElement | undefined>(undefined);
  let plot: uPlot | undefined;

  function buildPlot() {
    if (!container) return;
    const axisCommon = {
      stroke: theme.text,
      grid: { stroke: theme.grid, width: 1 },
      ticks: { stroke: theme.grid, width: 1 },
      font: '12px system-ui, -apple-system, sans-serif',
    };
    const series: uPlot.Series[] = [
      {},
      ...records.map((r, i) => ({
        label: `${r.stem} · ${r.metadata.analog_channels[r.channelIndex].id}`,
        stroke: theme.series[i % theme.series.length],
        width: 2,
        // Required: draws each record's line through the positions left
        // undefined/null by every other record after the join() merge.
        spanGaps: true,
      })),
    ];
    const opts: uPlot.Options = {
      width: container.clientWidth || 800,
      height: 280,
      scales: { x: { time: true } },
      axes: [
        { ...axisCommon, label: 'time' },
        { ...axisCommon, label: units, size: 56 },
      ],
      series,
      legend: { show: true },
      cursor: {
        // Distinct key from WaveformChart's 'gridsense-waveform' — that key is a
        // global string in uPlot, so reusing it would cross-sync zoom/cursor with any
        // single-record tab mounted elsewhere.
        sync: { key: 'gridsense-compare', setSeries: true, scales: ['x', null] },
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
    plot = new uPlot(opts, data, container);
    container.ondblclick = () => resetZoom();
  }

  function resetZoom() {
    plot?.setScale('x', { min: fullXRange[0], max: fullXRange[1] });
  }

  function destroyPlot() {
    plot?.destroy();
    plot = undefined;
  }

  onMount(buildPlot);
  onDestroy(destroyPlot);
</script>

<div class="toolbar">
  <button class="reset-zoom" onclick={resetZoom}>Reset zoom</button>
  <span class="hint">drag to zoom · double-click to reset</span>
</div>
<div class="chart-card">
  <div class="chart-container" bind:this={container}></div>
</div>
<p class="caption">
  Times shown in your browser's local timezone — source .cfg files carry no UTC offset, so this is a direct read of
  each record's timestamp, not a corrected one.
</p>

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
  .caption {
    color: var(--text-muted);
    font-size: 0.78rem;
    margin: 0.5rem 0 0;
  }

  /* uPlot chrome: title/legend text inherits the app's ink tokens (canvas drawing —
     axes, grid, event markers — is themed separately via lib/theme.ts, since canvas
     can't read CSS variables). */
  :global(.u-legend th) {
    color: var(--text-secondary);
    font-weight: 400;
    font-size: 0.78rem;
  }
  :global(.u-select) {
    background: color-mix(in srgb, var(--series-1) 15%, transparent);
    border: 1px solid var(--series-1);
  }
</style>
