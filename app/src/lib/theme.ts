export interface ChartTheme {
  /** Categorical series palette, in palette-slot order (dataviz skill's validated
   *  8-slot set). Only slots [0,2] are validated for simultaneous *all-pairs* display
   *  (e.g. a single record's A/B/C phase channels); the full 8 are validated for
   *  adjacent/sequential use (e.g. one color per compared record), which is legal
   *  past slot 3 only paired with a visible legend as secondary encoding — uPlot's
   *  built-in legend already provides that everywhere this is used. Callers index
   *  modulo `.length`, so any number of series degrades gracefully (repeating
   *  colors) rather than breaking. */
  series: string[];
  surface: string;
  grid: string;
  baseline: string;
  text: string;
  /** Vertical marker at a detected event's onset (status-critical, from palette.md). */
  markerCritical: string;
  /** Vertical marker at a correlated breaker/state-change time — dashed, neutral. */
  markerNeutral: string;
}

const LIGHT: ChartTheme = {
  series: ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#4a3aa7', '#e34948'],
  surface: '#fcfcfb',
  grid: '#e1e0d9',
  baseline: '#c3c2b7',
  text: '#898781',
  markerCritical: '#d03b3b',
  markerNeutral: '#52514e',
};

// Dark slot 8 (#e66767) is numerically identical to markerCritical below — harmless
// today since MultiRecordChart colors its onset markers by record identity
// (theme.series[i]), not by markerCritical/markerNeutral (those stay single-record
// fault/breaker semantics). Only matters if those two concepts are ever mixed.
const DARK: ChartTheme = {
  series: ['#3987e5', '#d95926', '#199e70', '#c98500', '#d55181', '#008300', '#9085e9', '#e66767'],
  surface: '#1a1a19',
  grid: '#2c2c2a',
  baseline: '#383835',
  text: '#898781',
  markerCritical: '#e66767',
  markerNeutral: '#c3c2b7',
};

/**
 * Resolves the current theme once, for chart chrome that can't read CSS custom
 * properties (uPlot draws axes/grid on canvas). Mirrors app.css's precedence: an
 * explicit `data-theme` wins, otherwise the OS preference. Resolved at chart-mount
 * time, not reactively — an OS theme flip mid-session won't repaint an already-built
 * chart without a reload, matching how most canvas-based dashboards behave.
 */
export function resolveChartTheme(): ChartTheme {
  const explicit = document.documentElement.dataset.theme;
  if (explicit === 'dark') return DARK;
  if (explicit === 'light') return LIGHT;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? DARK : LIGHT;
}
