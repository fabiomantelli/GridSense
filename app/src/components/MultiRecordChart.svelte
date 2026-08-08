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
  //
  // initialRange/onRangeChange thread a shared x-window through CompareView: uPlot's
  // cursor.sync only ever broadcasts LIVE scale changes between instances that are
  // already mounted in the same sync group, so a quantity opened after another one
  // was already zoomed would otherwise always start back at its own full range.
  // initialRange seeds a freshly-mounted chart with whatever window is currently
  // shared; onRangeChange reports this chart's window (its own zooms, resets, or a
  // sync update received from a sibling) back up so later-opened charts inherit it.
  let {
    records,
    units,
    initialRange = null,
    onRangeChange,
  }: {
    records: RecordSelection[];
    units: string;
    initialRange?: [number, number] | null;
    onRangeChange?: (range: [number, number]) => void;
  } = $props();

  const theme = resolveChartTheme();

  function toSeconds(metadata: CfgFile, timestampsUs: Float64Array): Float64Array {
    const start = metadata.start_epoch_us as number;
    const out = new Float64Array(timestampsUs.length);
    for (let i = 0; i < timestampsUs.length; i++) out[i] = (start + timestampsUs[i]) / 1_000_000;
    return out;
  }

  // Records being compared here span at most a few seconds, so uPlot's default
  // hierarchical time-axis formatter (built for dashboards spanning days/months,
  // which only relabels the digits that changed since the previous tick) never pays
  // for itself — it just prints a bare, ambiguous ":14.080" on every tick after the
  // first and buries the actual hour/minute in a small one-time corner label. Every
  // tick gets a full, self-contained HH:MM:ss.mmm instead.
  function formatAbsoluteTick(sec: number): string {
    const d = new Date(sec * 1000);
    const hh = String(d.getHours()).padStart(2, '0');
    const mm = String(d.getMinutes()).padStart(2, '0');
    const ss = String(d.getSeconds()).padStart(2, '0');
    const ms = String(d.getMilliseconds()).padStart(3, '0');
    return `${hh}:${mm}:${ss}.${ms}`;
  }

  // The legend's hover value is a single reading, not a repeated tick label, so it
  // can afford the date too — and needs to, since uPlot's own default here dropped
  // seconds entirely (e.g. "2026-04-16 3:02am"), useless for locating a sub-cycle
  // disturbance.
  function formatAbsoluteDateTime(sec: number): string {
    const d = new Date(sec * 1000);
    const y = d.getFullYear();
    const mo = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${mo}-${day} ${formatAbsoluteTick(sec)}`;
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
      // Series index i+1: index 0 in u.series is the x-series. Respects the
      // legend's own click-to-hide toggle — a record hidden from the plot
      // shouldn't still leave its onset marker behind.
      if (u.series[i + 1].show === false) return;
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
  let isReady = false;

  function buildPlot() {
    if (!container) return;
    const axisCommon = {
      stroke: theme.text,
      grid: { stroke: theme.grid, width: 1 },
      ticks: { stroke: theme.grid, width: 1 },
      font: '12px system-ui, -apple-system, sans-serif',
    };
    const series: uPlot.Series[] = [
      { value: (_u, raw) => (raw == null ? '--' : formatAbsoluteDateTime(raw)) },
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
        {
          ...axisCommon,
          label: 'time',
          values: (_u, splits) => splits.map(formatAbsoluteTick),
          // uPlot's tick-density picker assumes a fixed 50px/label (its default
          // `space`) regardless of what the axis's `values` fn actually renders —
          // it never measures real text. "HH:MM:ss.mmm" measures ~66px at this
          // font, so the default would let ticks land closer together than the
          // labels are wide. A little over the measured width keeps a visible gap
          // at any zoom level instead of letting labels touch or overlap.
          space: 90,
        },
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
        // Fires on this chart's own zoom/reset and on a scale change arriving via
        // cursor.sync from an already-mounted sibling chart — either way, report the
        // resulting window upward so a quantity opened after this one starts there
        // instead of at its own full range. Skipped until after the initial
        // construction-time auto-range/seed below, which isn't a user zoom.
        setScale: [
          (u, key) => {
            if (key !== 'x' || !isReady) return;
            const { min, max } = u.scales.x;
            if (min != null && max != null) onRangeChange?.([min, max]);
          },
        ],
        ready: [() => (isReady = true)],
      },
    };
    plot = new uPlot(opts, data, container);
    if (initialRange) plot.setScale('x', { min: initialRange[0], max: initialRange[1] });
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
