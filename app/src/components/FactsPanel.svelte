<script lang="ts">
  import type { AnalysisFacts, FaultKind } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';

  let { handle }: { handle: ComtradeHandle } = $props();

  // Session components are keyed by stem and never rebound to a different handle;
  // $derived here is only to satisfy Svelte's reactivity contract for the prop read.
  const facts = $derived(handle.run_analysis() as AnalysisFacts);

  function describeFaultKind(kind: FaultKind): string {
    if (kind === 'ThreePhase') return 'Three-phase fault';
    if (kind === 'Unclassified') return 'Unclassified event (insufficient correlated evidence)';
    if (typeof kind === 'object' && 'PhaseToGround' in kind) return `Phase ${kind.PhaseToGround} to ground fault`;
    if (typeof kind === 'object' && 'PhaseToPhase' in kind) {
      const [p1, p2] = kind.PhaseToPhase;
      return `Phase ${p1} to phase ${p2} fault`;
    }
    return 'Unknown event';
  }

  function fmtUs(us: number): string {
    return `${(us / 1000).toFixed(2)} ms`;
  }
</script>

<div class="facts">
  <section>
    <h3>Record</h3>
    <p>
      {facts.record_summary.station_name} / {facts.record_summary.device_id} —
      {facts.record_summary.sample_count} samples, {fmtUs(facts.record_summary.duration_us)} duration,
      {facts.record_summary.line_frequency} Hz nominal
    </p>
  </section>

  <section>
    <h3>Events</h3>
    {#if facts.events.length === 0}
      <p class="note">No step-change events detected.</p>
    {:else}
      {#each facts.events as e}
        <div class="event-card">
          <p class="event-kind">{describeFaultKind(e.kind)}</p>
          <p>Onset at {fmtUs(e.onset_time_us)} (sample {e.onset_sample}), channel group "{e.involved_group_label}"</p>
          {#if e.current_multiple !== null}
            <p>Fault current ≈ {e.current_multiple.toFixed(2)}× pre-event baseline</p>
          {/if}
          {#if e.breaker_channel_id}
            <p>
              Digital channel "{e.breaker_channel_id}" changed state{#if e.time_to_trip_us !== null}
                {' '}{fmtUs(e.time_to_trip_us)} after onset{/if}
            </p>
          {/if}
        </div>
      {/each}
    {/if}
  </section>

  <section>
    <h3>Sequence components (first cycle)</h3>
    {#if facts.sequence_component_groups.length === 0}
      <p class="note">No three-phase channel groups identified.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Group</th>
            <th>Units</th>
            <th>Zero</th>
            <th>Positive</th>
            <th>Negative</th>
          </tr>
        </thead>
        <tbody>
          {#each facts.sequence_component_groups as g}
            <tr>
              <td>{g.group_label}</td>
              <td>{g.units}</td>
              <td>{g.zero_magnitude.toFixed(3)}</td>
              <td>{g.positive_magnitude.toFixed(3)}</td>
              <td>{g.negative_magnitude.toFixed(3)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <section>
    <h3>Channel summary</h3>
    <table>
      <thead>
        <tr>
          <th>Channel</th>
          <th>Units</th>
          <th>Min</th>
          <th>Max</th>
          <th>Mean</th>
          <th>RMS</th>
        </tr>
      </thead>
      <tbody>
        {#each facts.channel_summaries as c}
          <tr>
            <td>{c.id}</td>
            <td>{c.units}</td>
            <td>{c.min.toFixed(3)}</td>
            <td>{c.max.toFixed(3)}</td>
            <td>{c.mean.toFixed(3)}</td>
            <td>{c.rms.toFixed(3)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
</div>

<style>
  .facts section {
    margin-bottom: 1.25rem;
  }
  .facts h3 {
    font-size: 0.9rem;
    opacity: 0.8;
    margin-bottom: 0.4rem;
  }
  .event-card {
    border: 1px solid #444;
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }
  .event-card p {
    margin: 0.15rem 0;
  }
  .event-kind {
    font-weight: 600;
  }
  table {
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  th,
  td {
    border: 1px solid #444;
    padding: 0.25rem 0.5rem;
    text-align: right;
  }
  th:first-child,
  td:first-child {
    text-align: left;
  }
  .note {
    opacity: 0.7;
    font-size: 0.9rem;
  }
</style>
