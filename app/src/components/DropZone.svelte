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
  <p>Drop a COMTRADE <code>.cfg</code> + <code>.dat</code> pair here</p>
  <p class="hint">or</p>
  <label class="file-input-label">
    Choose files
    <input type="file" multiple accept=".cfg,.dat" onchange={handleInputChange} />
  </label>
</div>

<style>
  .drop-zone {
    border: 2px dashed #888;
    border-radius: 8px;
    padding: 3rem 2rem;
    text-align: center;
    transition: border-color 0.15s, background-color 0.15s;
  }
  .drop-zone.dragging {
    border-color: #4a9eff;
    background-color: rgba(74, 158, 255, 0.08);
  }
  .hint {
    opacity: 0.6;
    margin: 0.5rem 0;
  }
  .file-input-label {
    display: inline-block;
    padding: 0.5rem 1rem;
    border: 1px solid #888;
    border-radius: 4px;
    cursor: pointer;
  }
  .file-input-label input {
    display: none;
  }
</style>
