<script lang="ts">
  import type { CfgFile } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';

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

<p class="summary">
  {metadata.station_name} / {metadata.device_id} — {timestamps.length} samples,
  {metadata.analog_channels.length} analog, {metadata.digital_channels.length} digital
  {#if timestamps.length > maxRows}
    (showing first {maxRows})
  {/if}
</p>

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

<style>
  .table-scroll {
    overflow-x: auto;
    max-height: 60vh;
    overflow-y: auto;
  }
  table {
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
  }
  th,
  td {
    border: 1px solid #444;
    padding: 0.25rem 0.5rem;
    text-align: right;
    white-space: nowrap;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--table-header-bg, #222);
  }
  .summary {
    font-size: 0.9rem;
    opacity: 0.8;
  }
</style>
