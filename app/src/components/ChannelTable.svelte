<script lang="ts">
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/voltcase_wasm';

  let {
    metadata,
    handle,
    maxRows = 100,
  }: { metadata: CfgFile; handle: ComtradeHandle; maxRows?: number } = $props();

  // Each session mounts its own ChannelTable instance (keyed by stem in App.svelte),
  // so these props are fixed for the component's lifetime — $derived here is about
  // satisfying Svelte's reactivity contract, not about the file changing underneath us.
  const timestamps = $derived(handle.timestamps_f64());
  const analogColumns = $derived(
    metadata.analog_channels.map((def, i) => ({ def, values: handle.analog_channel_f32(i) })),
  );
  const digitalColumns = $derived(
    metadata.digital_channels.map((def, i) => ({ def, values: handle.digital_channel_bools(i) })),
  );

  const rowCount = $derived(Math.min(timestamps.length, maxRows));
  const rows = $derived(Array.from({ length: rowCount }, (_, i) => i));
</script>

<details class="raw-table">
  <summary>
    Raw samples — {timestamps.length} samples, {metadata.analog_channels.length} analog, {metadata.digital_channels.length}
    digital
    {#if timestamps.length > maxRows}
      &nbsp;(showing first {maxRows})
    {/if}
  </summary>
  <div class="table-scroll">
    <table>
      <thead>
        <tr>
          <th>t (µs)</th>
          {#each analogColumns as col}
            <th>{col.def.id} ({col.def.units})</th>
          {/each}
          {#each digitalColumns as col}
            <th>{col.def.id}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as row}
          <tr>
            <td>{timestamps[row].toFixed(1)}</td>
            {#each analogColumns as col}
              <td>{col.values[row].toFixed(3)}</td>
            {/each}
            {#each digitalColumns as col}
              <td>{col.values[row] ? '1' : '0'}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</details>

<style>
  .raw-table {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card);
  }
  summary {
    cursor: pointer;
    padding: 0.7rem 1rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    user-select: none;
  }
  summary::marker {
    color: var(--text-muted);
  }
  .table-scroll {
    overflow: auto;
    max-height: 60vh;
    border-top: 1px solid var(--border);
  }
  table {
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
    font-size: 0.82rem;
    width: 100%;
  }
  th,
  td {
    padding: 0.3rem 0.6rem;
    text-align: right;
    white-space: nowrap;
  }
  td {
    border-bottom: 1px solid var(--border);
  }
  tbody tr:nth-child(even) {
    background: var(--page);
  }
  th {
    position: sticky;
    top: 0;
    background: var(--surface-raised);
    color: var(--text-muted);
    font-weight: 500;
    font-size: 0.75rem;
    border-bottom: 1px solid var(--border-strong);
  }
</style>
