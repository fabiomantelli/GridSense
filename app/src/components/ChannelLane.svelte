<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';
  import { resolveChartTheme } from '../lib/theme';

  // One compact uPlot instance per analog channel — the "many channels" answer to
  // WaveformChart's old one-instance-per-unit-group approach, which put every
  // channel sharing a unit on one axis and became an illegible tangle past a
  // handful of channels. Each lane is a small multiple: same shared Y-range as its
  // siblings (for at-a-glance magnitude comparison), synced zoom/cursor, its own
  // in-canvas label badge instead of a legend.
  let {
    label,
    color,
    channelIndex,
    handle,
    timestampsMs,
    yRange,
    showXAxis,
    onsetMarkersMs,
    tripMarkersMs,
    expanded = false,
  }: {
    label: string;
    color: string;
    channelIndex: number;
    handle: ComtradeHandle;
    timestampsMs: Float64Array;
    yRange: [number, number];
    showXAxis: boolean;
    onsetMarkersMs: number[];
    tripMarkersMs: number[];
    // Whether this lane's card is currently the fullscreen-expanded one. In
    // the normal in-page view every lane stays a fixed, compact height
    // regardless of channel count — that's the point of this view for
    // many-channel files. But a group with only 3-4 channels leaves a lot of
    // dead vertical space once expanded to fullscreen, while a group with 20
    // fills or overflows it. Expanded lanes grow (min-height still the
    // compact baseline, so a many-channel group is unaffected and still
    // scrolls) rather than staying pinned at their compact height.
    expanded?: boolean;
  } = $props();

  const theme = resolveChartTheme();

  // Canvas-only height (no legend/title exist on this chart at all — both are
  // suppressed below — so unlike WaveformChart/MultiRecordChart, there's no DOM
  // chrome whose real height needs a requestAnimationFrame-deferred measurement;
  // these are just plain constants).
  const LANE_HEIGHT = 64;
  // A compact x-axis band, shown only on the last currently-visible lane in a
  // group — deliberately smaller than uPlot's ~50px default, since this axis only
  // ever needs to show the same handful of short tick labels WaveformChart's
  // original per-group axis showed.
  const X_AXIS_HEIGHT = 32;

  // Real relay files carry channel ids far longer than this session's synthetic
  // test fixtures ever had (e.g. "IF22+IF23+UF27 (SRC 4) Ic THD" — 30 chars) — an
  // untruncated badge for one of those was wide enough to cover most of a lane's
  // waveform. Binary search (not a fixed character count) because this font isn't
  // monospace, so a char-count heuristic would either under- or over-truncate
  // depending on which letters happen to be in the id.
  function truncateToWidth(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
    if (ctx.measureText(text).width <= maxWidth) return text;
    const ellipsis = '…';
    let lo = 0;
    let hi = text.length;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (ctx.measureText(text.slice(0, mid) + ellipsis).width <= maxWidth) lo = mid;
      else hi = mid - 1;
    }
    return text.slice(0, lo) + ellipsis;
  }

  function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  function drawMarkers(u: uPlot) {
    const ctx = u.ctx;
    ctx.save();
    // Thinner than the data line (2px) on purpose: these are reference
    // annotations (an onset/trip moment), not the signal itself — the
    // thickness gap gives the eye a hierarchy between "this is data" and
    // "this marks a moment," matching the dataviz skill's hairline spec for
    // structural/reference lines vs. its 2px spec for actual data marks.
    ctx.lineWidth = 1;
    const draw = (ms: number, markerColor: string, dashed: boolean) => {
      const x = u.valToPos(ms, 'x', true);
      if (x < u.bbox.left || x > u.bbox.left + u.bbox.width) return;
      ctx.strokeStyle = markerColor;
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

  // Floating in-canvas identity badge, replacing the legend a chart this compact
  // has no room for. The swatch carries the series color; the label text stays in
  // the ink token — text never wears the data color (dataviz skill).
  function drawBadge(u: uPlot) {
    const ctx = u.ctx;
    ctx.save();
    ctx.font = '11px system-ui, -apple-system, sans-serif';
    ctx.textBaseline = 'middle';
    // uPlot's own x-axis tick-label drawing sets ctx.textAlign = 'center' (to
    // center each label under its tick) and never restores it — since that
    // happens on the shared canvas context right before this hook fires, the
    // lane that actually shows the x-axis inherited 'center' here, while every
    // other lane (no axis drawn, nothing to leave textAlign dirty) kept the
    // canvas default of 'left'. A centered fillText draws half its width
    // *left* of the given x, landing back on top of the swatch instead of
    // beside it — explicit 'left' makes every lane's badge draw identically
    // regardless of what the axis happened to leave behind.
    ctx.textAlign = 'left';
    // Cap the badge at roughly a third of the lane's own width (and an absolute
    // ceiling, so a single very narrow lane can't blow it back up) — it's an
    // identity hint, not a full legend; the container's `title` attribute below
    // carries the untruncated id on hover.
    const maxTextWidth = Math.max(40, Math.min(u.bbox.width * 0.32, 150));
    const displayLabel = truncateToWidth(ctx, label, maxTextWidth);
    const textWidth = ctx.measureText(displayLabel).width;
    const pad = 4;
    const swatch = 8;
    const gap = 4;
    const pillH = 18;
    const pillW = pad * 2 + swatch + gap + textWidth;
    const x = u.bbox.left + 6;
    const y = u.bbox.top + 6;

    ctx.globalAlpha = 0.85;
    ctx.fillStyle = theme.surface;
    roundRect(ctx, x, y, pillW, pillH, 4);
    ctx.fill();
    ctx.globalAlpha = 1;

    ctx.fillStyle = color;
    roundRect(ctx, x + pad, y + (pillH - swatch) / 2, swatch, swatch, 2);
    ctx.fill();

    ctx.fillStyle = theme.text;
    ctx.fillText(displayLabel, x + pad + swatch + gap, y + pillH / 2 + 1);
    ctx.restore();
  }

  let container = $state<HTMLDivElement | undefined>(undefined);
  let plot: uPlot | undefined;
  let resizeObserver: ResizeObserver | undefined;

  function buildPlot() {
    if (!container) return;
    const data = [timestampsMs, handle.analog_channel_f32(channelIndex)] as unknown as uPlot.AlignedData;
    const axisCommon = {
      stroke: theme.text,
      grid: { stroke: theme.grid, width: 1 },
      ticks: { stroke: theme.grid, width: 1 },
      font: '11px system-ui, -apple-system, sans-serif',
    };
    const opts: uPlot.Options = {
      width: container.clientWidth || 800,
      height: showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT,
      // uPlot auto-pads the left/right edges whenever the bottom (x) axis is
      // shown but the left (y) axis isn't (autoPadSide in uPlot.esm.js: it
      // reserves half the y-axis's default *size* — 25px, from yAxisOpts.size
      // = 50 — on each side, room for the first/last tick label's own half-
      // width to not clip past the plot edge) — since showXAxis is only true
      // on one lane per group, that lane alone got this padding, indenting it
      // relative to every sibling lane that got none. A flat 0 padding fixed
      // the alignment but then clipped the "0" tick's own label on that lane
      // (it's centered on x=0, so half of it rendered past the now-flush left
      // edge). Hardcoding the same 25px uPlot would have auto-computed —
      // unconditionally, on every lane, not just the one with a visible axis —
      // keeps every lane's plot area aligned AND leaves the tick label room.
      padding: [0, 25, 0, 25],
      scales: { x: { time: false }, y: { range: yRange } },
      axes: [
        { ...axisCommon, show: showXAxis, size: X_AXIS_HEIGHT },
        { show: false },
      ],
      // 2px, the dataviz skill's validated line-mark spec — also matches
      // MultiRecordChart's own series width, so a channel reads with the same
      // visual weight whether it's in a compact lane or a full-size compare chart.
      series: [{}, { stroke: color, width: 2 }],
      legend: { show: false },
      cursor: {
        // Same sync key WaveformChart's group-charts already used — every lane in
        // every group shares one sync group, reproducing today's cross-quantity
        // zoom/cursor sync just distributed over more, smaller instances.
        sync: { key: 'voltcase-waveform', setSeries: true, scales: ['x', null] },
        drag: { x: true, y: false },
      },
      hooks: {
        draw: [drawMarkers, drawBadge],
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

    // No legend/title chrome to account for here (both suppressed above), so —
    // unlike WaveformChart/MultiRecordChart — this needs neither a static
    // authoritative collapsed-container height hack nor a rAF-deferred chrome
    // measurement: the container's CSS height already matches opts.height exactly
    // (see the .lane rule below), so ResizeObserver only ever reacts to real width
    // changes.
    resizeObserver = new ResizeObserver(() => {
      if (!container || !plot) return;
      const width = container.clientWidth;
      const height = container.clientHeight;
      if (width > 0 && height > 0) plot.setSize({ width, height });
    });
    resizeObserver.observe(container);
  }

  function destroyPlot() {
    resizeObserver?.disconnect();
    resizeObserver = undefined;
    plot?.destroy();
    plot = undefined;
  }

  export function resetZoom(min: number, max: number) {
    plot?.setScale('x', { min, max });
  }

  onMount(buildPlot);
  onDestroy(destroyPlot);
</script>

<div
  class="lane"
  class:with-axis={showXAxis}
  class:flexible={expanded}
  style:height={expanded ? undefined : `${showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT}px`}
  style:min-height={expanded ? `${showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT}px` : undefined}
  title={label}
  bind:this={container}
></div>

<style>
  .lane {
    width: 100%;
    background: var(--surface);
    transition: background-color 0.1s ease;
  }
  /* Only meaningful while expanded (see the `expanded` prop doc comment) — the
     parent .lanes flex container gives every flexible lane an equal share of
     any leftover vertical space, on top of its min-height floor. A
     ResizeObserver already watches this container (see buildPlot) and calls
     plot.setSize() on any size change, so growing/shrinking via flex here
     resizes the actual chart canvas too, not just the empty div. */
  .lane.flexible {
    flex: 1 1 0;
  }
  /* Separation between lanes comes from the parent .lanes flex gap (a real
     sliver of the recessive page color) instead of a border stroke — a
     hairline drawn on every single lane read as "glued together" once there
     were a dozen-plus of them stacked; a gap of actual space reads as
     considered spacing instead of ink. */
  .lane:hover {
    background: var(--page);
  }
</style>
