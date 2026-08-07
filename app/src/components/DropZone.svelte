<script lang="ts">
  let { onFiles }: { onFiles: (files: File[]) => void } = $props();
  let dragging = $state(false);

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length) onFiles(files);
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    dragging = true;
  }

  function handleDragLeave() {
    dragging = false;
  }

  function handleInputChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    if (files.length) onFiles(files);
    input.value = '';
  }
</script>

<div
  class="drop-zone"
  class:dragging
  role="region"
  aria-label="COMTRADE file drop zone"
  ondrop={handleDrop}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
>
  <svg class="drop-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M7 17.5a4 4 0 01-.7-7.94 5 5 0 019.4-2.1A4.5 4.5 0 0117.5 16" />
    <path d="M12 12v7" />
    <path d="M9.5 16l2.5-2.5L14.5 16" />
  </svg>
  <p class="primary">Arraste um par COMTRADE <code>.cfg</code> + <code>.dat</code></p>
  <p class="hint">o arquivo nunca sai do seu navegador</p>
  <label class="file-input-label">
    Escolher arquivos
    <input type="file" multiple accept=".cfg,.dat" onchange={handleInputChange} />
  </label>
</div>

<style>
  .drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 3rem 2rem;
    background: var(--surface);
    transition:
      border-color 0.15s,
      background-color 0.15s;
  }
  .drop-zone.dragging {
    border-color: var(--series-1);
    background: color-mix(in srgb, var(--series-1) 8%, var(--surface));
  }
  .drop-icon {
    width: 40px;
    height: 40px;
    color: var(--text-muted);
    margin-bottom: 0.75rem;
  }
  .dragging .drop-icon {
    color: var(--series-1);
  }
  .primary {
    margin: 0;
    font-size: 0.95rem;
  }
  .primary code {
    background: var(--page);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.05rem 0.35rem;
    font-size: 0.85em;
  }
  .hint {
    margin: 0.3rem 0 1rem;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .file-input-label {
    display: inline-block;
    padding: 0.45rem 1rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.85rem;
    background: var(--surface-raised);
  }
  .file-input-label:hover {
    border-color: var(--series-1);
    color: var(--series-1);
  }
  .file-input-label input {
    display: none;
  }
</style>
