<script lang="ts">
  import type { AnalysisFacts, FaultKind } from '../lib/types';
  import type { ComtradeHandle } from '../wasm-pkg/gridsense_wasm';

  let { handle }: { handle: ComtradeHandle } = $props();

  // Session components are keyed by stem and never rebound to a different handle;
  // $derived here is only to satisfy Svelte's reactivity contract for the prop read.
  const facts = $derived(handle.run_analysis() as AnalysisFacts);

  type Severity = 'good' | 'warning' | 'critical';

  function severityOf(kind: FaultKind): Severity {
    return kind === 'Unclassified' ? 'warning' : 'critical';
  }

  function describeFaultKind(kind: FaultKind): string {
    if (kind === 'ThreePhase') return 'Three-phase fault';
    if (kind === 'Unclassified') return 'Unclassified event';
    if (typeof kind === 'object' && 'PhaseToGround' in kind) return `Phase ${kind.PhaseToGround} to ground fault`;
    if (typeof kind === 'object' && 'PhaseToPhase' in kind) {
      const [p1, p2] = kind.PhaseToPhase;
      return `Phase ${p1} to phase ${p2} fault`;
    }
    return 'Unknown event';
  }

  function fmtMs(us: number): string {
    return `${(us / 1000).toFixed(2)} ms`;
  }
</script>

{#snippet checkIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M8 12.5l2.5 2.5L16 9" />
  </svg>
{/snippet}
{#snippet alertIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 3.5l9 15.5H3l9-15.5z" />
    <path d="M12 9.5v4" />
    <circle cx="12" cy="16.7" r="0.15" fill="currentColor" stroke="currentColor" stroke-width="1.5" />
  </svg>
{/snippet}
{#snippet infoIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5" />
    <circle cx="12" cy="8" r="0.15" fill="currentColor" stroke="currentColor" stroke-width="1.5" />
  </svg>
{/snippet}

<div class="facts">
  <section class="record-card">
    <h3>{facts.record_summary.station_name}<span class="device-id">/ {facts.record_summary.device_id}</span></h3>
    <div class="stat-strip">
      <div class="stat">
        <span class="stat-value">{facts.record_summary.sample_count}</span>
        <span class="stat-label">amostras</span>
      </div>
      <div class="stat">
        <span class="stat-value">{fmtMs(facts.record_summary.duration_us)}</span>
        <span class="stat-label">duração</span>
      </div>
      <div class="stat">
        <span class="stat-value">{facts.record_summary.line_frequency} Hz</span>
        <span class="stat-label">nominal</span>
      </div>
    </div>
  </section>

  <section>
    <h4 class="section-label">Events</h4>
    {#if facts.events.length === 0}
      <div class="status-pill good">
        <span class="status-icon">{@render checkIcon()}</span>
        Nenhum evento de mudança abrupta detectado (limiar: 20% de RMS ciclo a ciclo)
      </div>
    {:else}
      <div class="event-list">
        {#each facts.events as e}
          {@const severity = severityOf(e.kind)}
          <div class="event-card {severity}">
            <span class="status-icon">{@render (severity === 'critical' ? alertIcon : infoIcon)()}</span>
            <div class="event-body">
              <p class="event-kind">{describeFaultKind(e.kind)}</p>
              <dl class="event-facts">
                <div>
                  <dt>Onset</dt>
                  <dd>{fmtMs(e.onset_time_us)} (amostra {e.onset_sample}), grupo "{e.involved_group_label}"</dd>
                </div>
                {#if e.current_multiple !== null}
                  <div>
                    <dt>Corrente de falta</dt>
                    <dd>≈ {e.current_multiple.toFixed(2)}× a linha de base pré-evento</dd>
                  </div>
                {/if}
                {#if e.breaker_channel_id}
                  <div>
                    <dt>Disjuntor</dt>
                    <dd>
                      "{e.breaker_channel_id}" mudou de estado{#if e.time_to_trip_us !== null}
                        {' '}{fmtMs(e.time_to_trip_us)} após o início{/if}
                    </dd>
                  </div>
                {/if}
              </dl>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section>
    <h4 class="section-label">Sequence components (first cycle)</h4>
    {#if facts.sequence_component_groups.length === 0}
      <p class="note">No three-phase channel groups identified.</p>
    {:else}
      <div class="table-card">
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
                <td class="label-col">{g.group_label}</td>
                <td>{g.units}</td>
                <td>{g.zero_magnitude.toFixed(3)}</td>
                <td>{g.positive_magnitude.toFixed(3)}</td>
                <td>{g.negative_magnitude.toFixed(3)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>

  <section>
    <h4 class="section-label">Channel summary</h4>
    <div class="table-card">
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
              <td class="label-col">{c.id}</td>
              <td>{c.units}</td>
              <td>{c.min.toFixed(3)}</td>
              <td>{c.max.toFixed(3)}</td>
              <td>{c.mean.toFixed(3)}</td>
              <td>{c.rms.toFixed(3)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
</div>

<style>
  .facts section {
    margin-bottom: 1.5rem;
  }
  .section-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    margin: 0 0 0.5rem;
  }

  .record-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 0.9rem 1rem;
    box-shadow: var(--shadow-card);
  }
  .record-card h3 {
    margin: 0 0 0.6rem;
    font-size: 1rem;
  }
  .device-id {
    color: var(--text-muted);
    font-weight: 400;
    margin-left: 0.35rem;
  }
  .stat-strip {
    display: flex;
    gap: 1.5rem;
  }
  .stat {
    display: flex;
    flex-direction: column;
  }
  .stat-value {
    font-size: 1.1rem;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .status-pill {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    border-radius: var(--radius-md);
    padding: 0.7rem 0.9rem;
    font-size: 0.9rem;
  }
  .status-pill.good {
    background: var(--status-good-bg);
    color: var(--text-primary);
  }
  .status-pill.good .status-icon {
    color: var(--status-good);
  }

  .event-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .event-card {
    display: flex;
    gap: 0.7rem;
    border-radius: var(--radius-md);
    padding: 0.8rem 1rem;
    border-left: 3px solid transparent;
  }
  .event-card.critical {
    background: var(--status-critical-bg);
    border-left-color: var(--status-critical);
  }
  .event-card.warning {
    background: var(--status-warning-bg);
    border-left-color: var(--status-warning);
  }
  .event-card.critical .status-icon {
    color: var(--status-critical);
  }
  .event-card.warning .status-icon {
    color: var(--status-serious);
  }
  .status-icon svg {
    width: 20px;
    height: 20px;
    display: block;
  }
  .event-kind {
    font-weight: 600;
    margin: 0 0 0.4rem;
  }
  .event-facts {
    margin: 0;
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  .event-facts div {
    display: flex;
    gap: 0.4rem;
  }
  .event-facts dt {
    color: var(--text-muted);
    min-width: 8rem;
    flex-shrink: 0;
  }
  .event-facts dd {
    margin: 0;
    color: var(--text-secondary);
  }

  .table-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: auto;
    box-shadow: var(--shadow-card);
  }
  table {
    border-collapse: collapse;
    font-size: 0.85rem;
    width: 100%;
  }
  th,
  td {
    padding: 0.4rem 0.7rem;
    text-align: right;
    border-bottom: 1px solid var(--border);
  }
  th {
    color: var(--text-muted);
    font-weight: 500;
    font-size: 0.78rem;
  }
  tr:last-child td {
    border-bottom: none;
  }
  .label-col {
    text-align: left;
    color: var(--text-primary);
    font-weight: 500;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.9rem;
  }
</style>
