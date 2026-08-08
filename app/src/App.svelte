<script lang="ts">
  import { onDestroy } from 'svelte';
  import DropZone from './components/DropZone.svelte';
  import ChannelTable from './components/ChannelTable.svelte';
  import WaveformChart from './components/WaveformChart.svelte';
  import DigitalTimeline from './components/DigitalTimeline.svelte';
  import FactsPanel from './components/FactsPanel.svelte';
  import CompareView from './components/CompareView.svelte';
  import { pairComtradeFiles, type ComtradePair, type PendingHalf } from './lib/filePairing';
  import { loadComtrade, disposeRecord } from './lib/wasm';
  import type { Session } from './lib/types';

  let sessions = $state<Session[]>([]);
  let activeStem = $state<string | null>(null);
  let viewMode = $state<'single' | 'compare'>('single');
  let pendingHalves = $state<PendingHalf[]>([]);
  let ignored = $state<File[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  function handleFiles(newFiles: File[]) {
    error = null;
    const combined = [...pendingHalves.map((p) => p.file), ...newFiles];
    const result = pairComtradeFiles(combined);
    ignored = result.ignored;
    pendingHalves = result.pending;
    if (result.pairs.length) {
      void loadPairs(result.pairs);
    }
  }

  async function loadPairs(pairs: ComtradePair[]) {
    loading = true;
    try {
      for (const pair of pairs) {
        const cfgBytes = new Uint8Array(await pair.cfg.arrayBuffer());
        const datBytes = new Uint8Array(await pair.dat.arrayBuffer());
        const { handle, metadata, facts } = await loadComtrade(cfgBytes, datBytes);
        const newSession = { stem: pair.stem, metadata, handle, facts };
        // Re-dropping a file with the same stem (e.g. re-testing, or a vendor tool
        // re-exporting the same record) replaces the existing session in place —
        // appending a second entry with the same stem produced two rows sharing one
        // Svelte keyed-each key, which Svelte can't render and silently ignores,
        // making the reload look like it did nothing.
        const existingIdx = sessions.findIndex((s) => s.stem === pair.stem);
        if (existingIdx === -1) {
          sessions = [...sessions, newSession];
        } else {
          disposeRecord(sessions[existingIdx]);
          sessions = sessions.map((s, i) => (i === existingIdx ? newSession : s));
        }
        activeStem = pair.stem;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function closeSession(stem: string) {
    const session = sessions.find((s) => s.stem === stem);
    if (session) {
      disposeRecord(session);
    }
    sessions = sessions.filter((s) => s.stem !== stem);
    if (activeStem === stem) {
      activeStem = sessions[0]?.stem ?? null;
    }
  }

  onDestroy(() => {
    for (const s of sessions) {
      disposeRecord(s);
    }
  });
</script>

<header class="app-header">
  <div class="brand">
    <svg class="mark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M2 12h4l2-7 4 14 3-10 2 3h5" />
    </svg>
    <span class="wordmark">Voltcase</span>
  </div>
  <span class="trust-badge">100% local · nothing is uploaded</span>
</header>

<main>
  <DropZone onFiles={handleFiles} />

  {#if loading}
    <p class="note">Analyzing…</p>
  {/if}
  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if pendingHalves.length}
    <p class="note">
      Waiting for matching file:
      {#each pendingHalves as p (p.stem + p.kind)}
        {p.stem}.{p.kind === 'cfg' ? 'dat' : 'cfg'}
      {/each}
    </p>
  {/if}
  {#if ignored.length}
    <p class="note">Ignored (not .cfg/.dat): {ignored.map((f) => f.name).join(', ')}</p>
  {/if}

  {#if sessions.length}
    {#if sessions.length >= 2}
      <div class="mode-toggle">
        <button onclick={() => (viewMode = viewMode === 'compare' ? 'single' : 'compare')}>
          {viewMode === 'compare' ? '← Back to sessions' : 'Compare records →'}
        </button>
      </div>
    {/if}

    {#if viewMode === 'compare'}
      <CompareView {sessions} />
    {:else}
      <div class="tabs">
        {#each sessions as s (s.stem)}
          <div class="tab" class:active={s.stem === activeStem}>
            <button class="tab-select" onclick={() => (activeStem = s.stem)}>{s.stem}</button>
            <button class="close" aria-label="Close {s.stem}" onclick={() => closeSession(s.stem)}>×</button>
          </div>
        {/each}
      </div>

      {#each sessions as s (s.stem)}
        {#if s.stem === activeStem}
          <section>
            <FactsPanel facts={s.facts} />
          </section>
          <section>
            <h2>Waveforms</h2>
            <WaveformChart metadata={s.metadata} handle={s.handle} facts={s.facts} />
          </section>
          <section>
            <DigitalTimeline metadata={s.metadata} handle={s.handle} />
          </section>
          <section>
            <ChannelTable metadata={s.metadata} handle={s.handle} />
          </section>
        {/if}
      {/each}
    {/if}
  {/if}
</main>

<style>
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.9rem 1.5rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .mark {
    width: 22px;
    height: 22px;
    color: var(--series-1);
  }
  .wordmark {
    font-weight: 600;
    font-size: 0.95rem;
    letter-spacing: -0.01em;
  }
  .trust-badge {
    font-size: 0.75rem;
    color: var(--text-muted);
    background: var(--page);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.25rem 0.7rem;
  }

  main {
    max-width: 980px;
    margin: 0 auto;
    padding: 1.75rem 1.25rem 4rem;
  }
  section {
    margin-top: 1.75rem;
  }
  section h2 {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    margin: 0 0 0.5rem;
  }
  .error {
    color: var(--status-critical);
    font-size: 0.9rem;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .mode-toggle {
    margin-top: 1.5rem;
  }
  .mode-toggle button {
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.8rem;
    font-size: 0.82rem;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .mode-toggle button:hover {
    border-color: var(--series-1);
    color: var(--series-1);
  }

  .tabs {
    display: flex;
    gap: 0.3rem;
    margin: 1rem 0 0;
    flex-wrap: wrap;
  }
  .tab {
    display: flex;
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 0.3rem 0 0.9rem;
  }
  .tab.active {
    border-color: var(--series-1);
    background: color-mix(in srgb, var(--series-1) 10%, var(--surface));
  }
  .tab-select,
  .close {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.4rem 0.3rem;
  }
  .close {
    color: var(--text-muted);
    font-size: 1rem;
    line-height: 1;
  }
  .close:hover {
    color: var(--status-critical);
  }
</style>
