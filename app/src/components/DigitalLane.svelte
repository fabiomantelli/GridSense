<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';
  import { resolveChartTheme } from '../lib/theme';

  // Digital channels used to live in their own component (DigitalTimeline),
  // rendered as raw SVG bars on a fixed 0..800 coordinate system with no time
  // axis at all and no connection to WaveformChart's synced cursor/zoom group.
  // That meant a status change here couldn't be lined up against the analog
  // waveforms above it except by eyeballing — the exact correlation (did the
  // breaker open before or after the fault current cleared?) an analyst
  // actually needs. This is a real uPlot instance instead, joining the same
  // 'voltcase-waveform' sync group as every ChannelLane: same time axis, same
  // synced cursor, same drag-to-zoom/double-click-to-reset.
  let {
    label,
    channelIndex,
    handle,
    timestampsMs,
    showXAxis,
    expanded = false,
  }: {
    label: string;
    channelIndex: number;
    handle: ComtradeHandle;
    timestampsMs: Float64Array;
    showXAxis: boolean;
    expanded?: boolean;
  } = $props();

  const theme = resolveChartTheme();

  const LANE_HEIGHT = 28;
  const X_AXIS_HEIGHT = 32;

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

  // Drawn straight from the boolean sample array via u.valToPos, not from a
  // pre-computed segment list scaled against a fixed viewBox — this is what
  // makes the bars land at the correct pixel under drag-zoom/pan for free,
  // same as every other canvas-drawn overlay in this app.
  function drawStates(u: uPlot) {
    if (!bools || !bools.length) return;
    const ctx = u.ctx;
    ctx.save();
    const top = u.bbox.top + 3;
    const height = u.bbox.height - 6;
    const drawSeg = (i0: number, i1: number, state: boolean) => {
      const x0 = Math.max(u.bbox.left, u.valToPos(timestampsMs[i0], 'x', true));
      const x1 = Math.min(u.bbox.left + u.bbox.width, u.valToPos(timestampsMs[i1], 'x', true));
      if (x1 <= x0) return;
      ctx.fillStyle = state ? theme.series[0] : theme.grid;
      ctx.fillRect(x0, top, Math.max(1, x1 - x0), height);
    };
    let segStart = 0;
    let state = bools[0] === 1;
    for (let i = 1; i < bools.length; i++) {
      const s = bools[i] === 1;
      if (s !== state) {
        drawSeg(segStart, i, state);
        segStart = i;
        state = s;
      }
    }
    drawSeg(segStart, bools.length - 1, state);
    ctx.restore();
  }

  // No per-channel color here (unlike ChannelLane's swatch) — color in this
  // lane encodes *state*, not identity, so the badge is label-only, just a
  // surface-colored pill for legibility over whichever state is filled
  // underneath it.
  function drawBadge(u: uPlot) {
    const ctx = u.ctx;
    ctx.save();
    ctx.font = '11px system-ui, -apple-system, sans-serif';
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'left';
    const maxTextWidth = Math.max(40, Math.min(u.bbox.width * 0.4, 170));
    const displayLabel = truncateToWidth(ctx, label, maxTextWidth);
    const textWidth = ctx.measureText(displayLabel).width;
    const pad = 4;
    const pillH = 16;
    const pillW = pad * 2 + textWidth;
    const x = u.bbox.left + 6;
    const y = u.bbox.top + u.bbox.height / 2 - pillH / 2;

    ctx.globalAlpha = 0.85;
    ctx.fillStyle = theme.surface;
    roundRect(ctx, x, y, pillW, pillH, 4);
    ctx.fill();
    ctx.globalAlpha = 1;

    ctx.fillStyle = theme.text;
    ctx.fillText(displayLabel, x + pad, y + pillH / 2 + 1);
    ctx.restore();
  }

  let container = $state<HTMLDivElement | undefined>(undefined);
  let plot: uPlot | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let bools: Uint8Array | undefined;

  function buildPlot() {
    if (!container) return;
    bools = handle.digital_channel_bools(channelIndex);
    // Series 1 exists only to give uPlot a data source/x-domain — it's never
    // shown; the actual bars are hand-drawn in drawStates from the same
    // boolean array, since a real digital trace needs two fill colors (on
    // *and* off), not the single stroke/fill uPlot's own series supports.
    const data = [timestampsMs, bools] as unknown as uPlot.AlignedData;
    const axisCommon = {
      stroke: theme.text,
      grid: { stroke: theme.grid, width: 1 },
      ticks: { stroke: theme.grid, width: 1 },
      font: '11px system-ui, -apple-system, sans-serif',
    };
    const opts: uPlot.Options = {
      width: container.clientWidth || 800,
      height: showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT,
      // Same unconditional padding as ChannelLane — keeps this lane's plot
      // area aligned with every analog lane above it regardless of which one
      // happens to show the x-axis (see ChannelLane's own comment for why).
      padding: [0, 25, 0, 25],
      scales: { x: { time: false }, y: { range: [0, 1] } },
      axes: [
        { ...axisCommon, show: showXAxis, size: X_AXIS_HEIGHT },
        { show: false },
      ],
      series: [{}, { show: false }],
      legend: { show: false },
      cursor: {
        sync: { key: 'voltcase-waveform', setSeries: true, scales: ['x', null] },
        drag: { x: true, y: false },
      },
      hooks: {
        draw: [drawStates, drawBadge],
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
  class:flexible={expanded}
  style:height={expanded ? undefined : `${showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT}px`}
  style:min-height={expanded ? `${showXAxis ? LANE_HEIGHT + X_AXIS_HEIGHT : LANE_HEIGHT}px` : undefined}
  title={label}
>
  <div class="plot-mount" bind:this={container}></div>
</div>

<style>
  .lane {
    position: relative;
    width: 100%;
    background: var(--surface);
  }
  .plot-mount {
    width: 100%;
    height: 100%;
  }
  .lane.flexible {
    flex: 1 1 0;
  }
</style>
