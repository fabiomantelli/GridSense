<script lang="ts">
  import { onDestroy } from 'svelte';
  import DropZone from './components/DropZone.svelte';
  import ChannelTable from './components/ChannelTable.svelte';
  import WaveformChart from './components/WaveformChart.svelte';
  import DigitalTimeline from './components/DigitalTimeline.svelte';
  import FactsPanel from './components/FactsPanel.svelte';
  import { pairComtradeFiles, type ComtradePair, type PendingHalf } from './lib/filePairing';
  import { loadComtrade, disposeRecord } from './lib/wasm';
  import type { CfgFile } from './lib/types';
  import type { ComtradeHandle } from './wasm-pkg/gridsense_wasm';

  interface Session {
    stem: string;
    metadata: CfgFile;
    handle: ComtradeHandle;
  }

  let sessions = $state<Session[]>([]);
  let activeStem = $state<string | null>(null);
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
        const { handle, metadata } = await loadComtrade(cfgBytes, datBytes);
        sessions = [...sessions, { stem: pair.stem, metadata, handle }];
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
      disposeRecord({ handle: session.handle, metadata: session.metadata });
    }
    sessions = sessions.filter((s) => s.stem !== stem);
    if (activeStem === stem) {
      activeStem = sessions[0]?.stem ?? null;
    }
  }

  onDestroy(() => {
    for (const s of sessions) {
      disposeRecord({ handle: s.handle, metadata: s.metadata });
    }
  });
</script>

<main>
  <h1>Utility Investigator</h1>
  <p class="tagline">Drop a COMTRADE file pair. Parsing happens entirely in your browser — nothing is uploaded.</p>

  <DropZone onFiles={handleFiles} />

  {#if loading}
    <p>Parsing…</p>
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
          <h2>Analysis</h2>
          <FactsPanel handle={s.handle} />
        </section>
        <section>
          <h2>Waveforms</h2>
          <WaveformChart metadata={s.metadata} handle={s.handle} />
        </section>
        <section>
          <h2>Digital channels</h2>
          <DigitalTimeline metadata={s.metadata} handle={s.handle} />
        </section>
        <section>
          <h2>Raw samples</h2>
          <ChannelTable metadata={s.metadata} handle={s.handle} />
        </section>
      {/if}
    {/each}
  {/if}
</main>

<style>
  main {
    max-width: 960px;
    margin: 0 auto;
    padding: 2rem 1rem;
    font-family: system-ui, sans-serif;
  }
  .tagline {
    opacity: 0.7;
    margin-top: -0.5rem;
  }
  section {
    margin-top: 1.5rem;
  }
  section h2 {
    font-size: 1rem;
    opacity: 0.8;
    margin-bottom: 0.5rem;
  }
  .error {
    color: #ff6b6b;
  }
  .note {
    opacity: 0.8;
    font-size: 0.9rem;
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    margin: 1rem 0 0.5rem;
    border-bottom: 1px solid #444;
  }
  .tab {
    display: flex;
    align-items: center;
    border: 1px solid #444;
    border-bottom: none;
    border-radius: 4px 4px 0 0;
    padding: 0 0.25rem 0 0.75rem;
  }
  .tab.active {
    background: rgba(255, 255, 255, 0.08);
  }
  .tab-select,
  .close {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0.4rem 0.25rem;
  }
  .close {
    opacity: 0.6;
  }
  .close:hover {
    opacity: 1;
  }
</style>
