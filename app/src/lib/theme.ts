export interface ChartTheme {
  series: [string, string, string];
  surface: string;
  grid: string;
  baseline: string;
  text: string;
}

const LIGHT: ChartTheme = {
  series: ['#2a78d6', '#eb6834', '#1baf7a'],
  surface: '#fcfcfb',
  grid: '#e1e0d9',
  baseline: '#c3c2b7',
  text: '#898781',
};

const DARK: ChartTheme = {
  series: ['#3987e5', '#d95926', '#199e70'],
  surface: '#1a1a19',
  grid: '#2c2c2a',
  baseline: '#383835',
  text: '#898781',
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
