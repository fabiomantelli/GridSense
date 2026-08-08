<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { AnalysisFacts, CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';
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
  // UTC getters, not local (getHours/getMonth/etc.) — voltcase-core's
  // parse_comtrade_timestamp deliberately applies no UTC offset (the COMTRADE
  // format carries none to apply: timestamp_start_raw is just the relay's own
  // clock reading, with no timezone attached), so start_epoch_us's digits
  // *are* the raw file's digits, reinterpreted as literal UTC. Reading them
  // back with local getters silently re-shifted them by the browser's own
  // timezone offset — the displayed time no longer matched what's actually
  // printed in the .cfg file. UTC getters round-trip the original digits
  // exactly, regardless of what timezone the viewer's browser happens to be in.
  function formatAbsoluteTick(sec: number): string {
    const d = new Date(sec * 1000);
    const hh = String(d.getUTCHours()).padStart(2, '0');
    const mm = String(d.getUTCMinutes()).padStart(2, '0');
    const ss = String(d.getUTCSeconds()).padStart(2, '0');
    const ms = String(d.getUTCMilliseconds()).padStart(3, '0');
    return `${hh}:${mm}:${ss}.${ms}`;
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
        // Same primary/secondary hierarchy as ChannelLane's single-record
        // markers: a classified fault (identified kind) stays fully bold, an
        // "Unclassified event" dims — a noisy real file can carry 100+ of
        // these and drawing every one at full strength buries the one
        // classified fault among them.
        const classified = e.kind !== 'Unclassified';
        ctx.lineWidth = classified ? 1.5 : 1;
        ctx.globalAlpha = classified ? 1 : 0.35;
        ctx.beginPath();
        ctx.moveTo(x, u.bbox.top);
        ctx.lineTo(x, u.bbox.top + u.bbox.height);
        ctx.stroke();
      }
    });
    ctx.restore();
  }

  // This component's original fixed canvas height, from before the expand
  // feature existed — still the target for the collapsed (non-expanded) state.
  const CANVAS_HEIGHT = 280;

  let container = $state<HTMLDivElement | undefined>(undefined);
  let plot: uPlot | undefined;
  let isReady = false;
  let isExpanded = $state(false);
  let resizeObserver: ResizeObserver | undefined;

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
      height: CANVAS_HEIGHT,
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
      // live: false — keeps the legend for identity (color + record name) and
      // its click-to-hide-series interaction, but drops the per-series value
      // cell that used to update on every cursor move. Confirmed in uPlot's
      // own source (initLegendRow): with live:false, legendCols is never
      // populated, so no value <td> is created for any row at all — this
      // isn't hiding the cell with CSS, uPlot just doesn't build it.
      legend: { show: true, live: false },
      cursor: {
        // Distinct key from WaveformChart's 'voltcase-waveform' — that key is a
        // global string in uPlot, so reusing it would cross-sync zoom/cursor with any
        // single-record tab mounted elsewhere.
        sync: { key: 'voltcase-compare', setSeries: true, scales: ['x', null] },
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

    // The collapsed (non-expanded) container needs an authoritative CSS height —
    // see the ResizeObserver comment below for why leaving it content-driven would
    // get the chart stuck at fullscreen size after exiting. But that height has to
    // be *canvas height (280, this component's original fixed value) + however
    // tall the legend actually renders*, not just 280 — the legend is real DOM
    // taking up real space below the canvas, inside the same container, so sizing
    // the container to exactly the old canvas-only height leaves no room for it,
    // and the ResizeObserver below (which treats the container's height as the
    // budget for canvas+legend together) then has no choice but to shrink the
    // canvas to make the legend fit.
    //
    // Deferred a frame on purpose: uPlot's own stylesheet (imported above) lays the
    // legend out as one horizontal row via CSS, but reading its offsetHeight in the
    // same synchronous tick as `new uPlot(...)` — before the browser has applied
    // that stylesheet to the just-inserted table — catches it in plain-HTML-table
    // form instead, one row per record stacked vertically (confirmed by logging the
    // legend's outerHTML: a much taller table for what renders as a single row one
    // frame later). rAF waits for that layout pass to actually happen first.
    requestAnimationFrame(() => {
      if (!container) return;
      const legendHeight = container.querySelector<HTMLElement>('.u-legend')?.offsetHeight ?? 0;
      container.style.height = `${CANVAS_HEIGHT + legendHeight}px`;
    });

    // uPlot sizes itself once at construction and never re-measures its container on
    // its own — needed so the canvas actually grows/shrinks when the expand toggle
    // (or a window resize) changes the container's real size. setSize's `height`
    // governs only the plotting area (axes+canvas); the legend renders as extra DOM
    // below that inside the same container. Feeding it container.clientHeight
    // directly would leave no room for the legend — it'd overflow past the
    // container's own box — and since that overflow grows the container's natural
    // content height too, every observation would ratchet the canvas taller than
    // the last forever. Subtracting the legend's own (canvas-independent) height
    // keeps the whole uPlot root inside the container it was actually given. This
    // can legitimately fire once before the rAF above has corrected the
    // container's height (reading the same not-yet-laid-out legend) — harmless: it
    // briefly undersizes the canvas by the same margin, and self-corrects the
    // moment the rAF's height change triggers this observer again.
    //
    // Also re-sets container.style.height every time, not just once at mount:
    // the legend's real height can change after mount for reasons that have
    // nothing to do with a resize this observer would otherwise catch (e.g. its
    // row wrapping differently at a width the container itself didn't change
    // to, or web fonts finishing their swap after the rAF above already ran).
    // Leaving the mount-time snapshot as the only source of truth let the
    // container's CSS height drift stale — the legend would then overflow past
    // the container's own (too-short) box, visually landing on top of the
    // .caption text sitting right below it in the DOM. Setting the *same*
    // computed height here is a no-op (ResizeObserver doesn't refire on an
    // unchanged size), so this can't loop.
    resizeObserver = new ResizeObserver(() => {
      if (!container || !plot) return;
      const legendHeight = container.querySelector<HTMLElement>('.u-legend')?.offsetHeight ?? 0;
      if (!isExpanded) container.style.height = `${CANVAS_HEIGHT + legendHeight}px`;
      const width = container.clientWidth;
      const height = container.clientHeight - legendHeight;
      if (width > 0 && height > 0) plot.setSize({ width, height });
    });
    resizeObserver.observe(container);
  }

  function resetZoom() {
    plot?.setScale('x', { min: fullXRange[0], max: fullXRange[1] });
  }

  function destroyPlot() {
    resizeObserver?.disconnect();
    resizeObserver = undefined;
    plot?.destroy();
    plot = undefined;
  }

  function toggleExpand() {
    isExpanded = !isExpanded;
  }

  // While expanded, the chart takes over the viewport like a lightbox: Esc closes it
  // (the discoverable, expected way out of any full-screen-ish overlay) and the page
  // behind it stops scrolling so the two scroll contexts don't fight each other.
  $effect(() => {
    if (!isExpanded) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    function onKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') isExpanded = false;
    }
    window.addEventListener('keydown', onKeydown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', onKeydown);
    };
  });

  onMount(buildPlot);
  onDestroy(destroyPlot);
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

{#if isExpanded}
  <button class="backdrop" onclick={toggleExpand} aria-label="Exit fullscreen"></button>
{/if}
<div class="chart-card" class:expanded={isExpanded}>
  <div class="toolbar">
    <button class="reset-zoom" onclick={resetZoom}>Reset zoom</button>
    <span class="hint">drag to zoom · double-click to reset{isExpanded ? ' · Esc to exit' : ''}</span>
    <button class="expand-toggle" onclick={toggleExpand} aria-label={isExpanded ? 'Exit fullscreen' : 'Expand chart'}>
      {@render (isExpanded ? shrinkIcon : expandIcon)()}
    </button>
  </div>
  <div class="chart-container" bind:this={container}></div>
  <p class="caption">
    Times shown exactly as recorded in each source .cfg file — the format carries no timezone, so this is a direct
    read of each record's own clock, not corrected or shifted to your browser's timezone.
  </p>
</div>

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
    margin-left: auto;
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
  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 0.75rem;
    box-shadow: var(--shadow-card);
  }
  .chart-card.expanded {
    position: fixed;
    inset: 2rem;
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .chart-card.expanded .chart-container {
    flex: 1;
    min-height: 0;
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
  .chart-container {
    width: 100%;
    /* Fallback only, for the brief window before the script sets a precise inline
       height (canvas + actual legend height) once the plot exists — see buildPlot.
       Needs *some* authoritative (non-content-driven) height even as a fallback:
       exiting expanded mode is circular otherwise, the container reporting "no
       change" because it was simply echoing whatever oversized canvas was still
       sitting inside it, and the chart would stay stuck at fullscreen size forever.
       flex:1 below still wins over this while expanded (flex-basis from the `1`
       shorthand is 0%, not this height), so this only governs the collapsed state. */
    height: 280px;
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
