<script lang="ts">
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { onMount, onDestroy } from 'svelte';
  import type { AnalysisFacts, CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';
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

  // This component's original fixed canvas height, from before the expand
  // feature existed — still the target for the collapsed (non-expanded) state.
  const CANVAS_HEIGHT = 220;

  let containers = $state<HTMLDivElement[]>([]);
  let plots: uPlot[] = [];
  let resizeObservers: ResizeObserver[] = [];
  // At most one group expanded to fullscreen at a time — index into `groups`.
  let expandedGroupIndex = $state<number | null>(null);

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
        height: CANVAS_HEIGHT,
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
          sync: { key: 'voltcase-waveform', setSeries: true, scales: ['x', null] },
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

      // The collapsed container needs canvas height (220) + however tall the
      // title and legend actually render, not just 220 — those are real DOM
      // taking up real space in the same container, so sizing it to exactly the
      // old canvas-only height leaves them no room, and the ResizeObserver below
      // (which treats the container's height as the budget for title+canvas+legend
      // together) then has no choice but to shrink the canvas to fit them in.
      //
      // Deferred a frame on purpose: uPlot's own stylesheet (imported above) lays
      // the legend out as one horizontal row via CSS, but reading its offsetHeight
      // in the same synchronous tick as `new uPlot(...)` — before the browser has
      // applied that stylesheet to the just-inserted table — catches it in
      // plain-HTML-table form instead, one row per series stacked vertically
      // (confirmed by logging the legend's outerHTML: a >300px-tall table for what
      // renders as a ~29px single row one frame later). rAF waits for that layout
      // pass to actually happen first.
      requestAnimationFrame(() => {
        if (!el.isConnected) return;
        const chromeHeight =
          (el.querySelector<HTMLElement>('.u-title')?.offsetHeight ?? 0) +
          (el.querySelector<HTMLElement>('.u-legend')?.offsetHeight ?? 0);
        el.style.height = `${CANVAS_HEIGHT + chromeHeight}px`;
      });

      // uPlot sizes itself once at construction and never re-measures its
      // container on its own — needed so the canvas actually grows/shrinks when a
      // card's expand toggle (or a window resize) changes the container's real
      // size. setSize's height governs only the plotting area, not the title
      // (above) or legend (below) uPlot renders as extra DOM in the same
      // container — subtracting both keeps the whole uPlot root inside the space
      // it was actually given instead of overflowing it. That overflow mattering
      // isn't just cosmetic: since it would grow the container's own *content*
      // height too, every observation would ratchet the canvas taller than the
      // last forever the moment the container ever went back to being
      // content-sized instead of CSS-sized (see the collapsed .chart-container
      // height rule below). This can legitimately fire once before the rAF above
      // has corrected the container's height (reading the same not-yet-laid-out
      // legend, same as that callback would without the deferral) — harmless: it
      // briefly undersizes the canvas by the same margin, and self-corrects the
      // moment the rAF's height change triggers this observer again.
      const ro = new ResizeObserver(() => {
        const titleHeight = el.querySelector<HTMLElement>('.u-title')?.offsetHeight ?? 0;
        const legendHeight = el.querySelector<HTMLElement>('.u-legend')?.offsetHeight ?? 0;
        const width = el.clientWidth;
        const height = el.clientHeight - titleHeight - legendHeight;
        if (width > 0 && height > 0) plot.setSize({ width, height });
      });
      ro.observe(el);
      resizeObservers.push(ro);

      return plot;
    });
  }

  function resetZoom() {
    for (const p of plots) {
      p.setScale('x', { min: fullXRange[0], max: fullXRange[1] });
    }
  }

  function destroyPlots() {
    for (const ro of resizeObservers) ro.disconnect();
    resizeObservers = [];
    for (const p of plots) p.destroy();
    plots = [];
  }

  function toggleExpand(gi: number) {
    expandedGroupIndex = expandedGroupIndex === gi ? null : gi;
  }

  // While a card is expanded it behaves like a lightbox: Esc closes it and the
  // page behind it stops scrolling so the two scroll contexts don't fight.
  $effect(() => {
    if (expandedGroupIndex == null) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    function onKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') expandedGroupIndex = null;
    }
    window.addEventListener('keydown', onKeydown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', onKeydown);
    };
  });

  onMount(buildPlots);
  onDestroy(destroyPlots);
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

{#if groups.length}
  <div class="toolbar">
    <button class="reset-zoom" onclick={resetZoom}>Reset zoom</button>
    <span class="hint">drag to zoom · double-click to reset</span>
  </div>
  {#if expandedGroupIndex != null}
    <button class="backdrop" onclick={() => (expandedGroupIndex = null)} aria-label="Exit fullscreen"></button>
  {/if}
  <div class="charts">
    {#each groups as [units], gi (units)}
      <div class="chart-card" class:expanded={expandedGroupIndex === gi}>
        <div class="card-toolbar">
          {#if expandedGroupIndex === gi}
            <span class="hint">Esc or click outside to exit</span>
          {/if}
          <button
            class="expand-toggle"
            onclick={() => toggleExpand(gi)}
            aria-label={expandedGroupIndex === gi ? 'Exit fullscreen' : `Expand ${units} chart`}
          >
            {@render (expandedGroupIndex === gi ? shrinkIcon : expandIcon)()}
          </button>
        </div>
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
  .card-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.6rem;
    margin-bottom: 0.4rem;
  }
  .expand-toggle {
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
  .chart-container {
    width: 100%;
    /* Fallback only, for the brief window before the script sets a precise inline
       height (canvas + actual title/legend height) once the plot exists — see
       buildPlots. Needs *some* authoritative (non-content-driven) height even as a
       fallback: a content-sized collapsed container would leave the chart stuck at
       fullscreen size after exiting (see the ResizeObserver comment in the script
       block). flex:1 above still wins over this while expanded, since the `1`
       shorthand sets flex-basis:0%. */
    height: 220px;
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
