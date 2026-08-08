export type ThemeName = 'light' | 'dark';

const STORAGE_KEY = 'voltcase-theme';

function systemPrefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

// index.html's inline pre-mount script already set this from localStorage (or
// `?theme=`) before Svelte ever ran, so first paint and this store agree —
// re-deriving here rather than re-reading localStorage directly keeps that one
// script the single source of truth for "what decided the theme on load."
function initialTheme(): ThemeName {
  const explicit = document.documentElement.dataset.theme;
  if (explicit === 'light' || explicit === 'dark') return explicit;
  return systemPrefersDark() ? 'dark' : 'light';
}

let current = $state<ThemeName>(initialTheme());

export function getTheme(): ThemeName {
  return current;
}

// Chart chrome (lib/theme.ts's resolveChartTheme) reads document.documentElement's
// data-theme attribute fresh each time it's called rather than reactively — by
// design, since it colors canvas draw calls, not CSS. Setting the attribute here
// means any chart component that remounts after this call (see the `{#key}`
// wrappers in App.svelte/CompareView.svelte) picks up the new theme correctly;
// existing already-drawn canvases are untouched until they do.
export function setTheme(next: ThemeName) {
  current = next;
  document.documentElement.dataset.theme = next;
  localStorage.setItem(STORAGE_KEY, next);
}

export function toggleTheme() {
  setTheme(current === 'dark' ? 'light' : 'dark');
}
