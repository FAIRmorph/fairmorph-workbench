<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  type CsvSummary = {
    source_file: string;
    columns: string[];
    processed_records: number;
    dataset_sha256: string;
    output_file?: string;
  };

  type CsvEvent =
    | { event: 'progress'; stage: string; percentage: number; message: string }
    | { event: 'result'; status: 'success'; data: CsvSummary }
    | { event: 'result'; status: 'error'; message: string };

  let sourcePath = $state('');
  let running = $state(false);
  let ready = $state(false);
  let progress = $state(0);
  let message = $state('');
  let summary = $state<CsvSummary | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    let removed = false;
    let unlisten: UnlistenFn | undefined;
    listen<string>('csv-event', ({ payload }) => {
      try {
        const event = JSON.parse(payload) as CsvEvent;
        if (event.event === 'progress') {
          progress = event.percentage;
          message = event.message;
        } else if (event.status === 'success') {
          progress = 100;
          message = 'CSV inspection completed';
          summary = event.data;
          running = false;
        } else {
          error = event.message;
          running = false;
        }
      } catch {
        error = 'Received an invalid response from the engine';
        running = false;
      }
    }).then((stop) => {
      if (removed) stop();
      else {
        unlisten = stop;
        ready = true;
      }
    }).catch((cause) => { error = String(cause); });
    return () => {
      removed = true;
      ready = false;
      unlisten?.();
    };
  });

  async function chooseFile() {
    error = null;
    try {
      const selected = await invoke<string | null>('select_csv_file');
      if (selected) {
        sourcePath = selected;
        summary = null;
        progress = 0;
      }
    } catch (cause) {
      error = String(cause);
    }
  }

  async function inspect() {
    if (!sourcePath.trim() || !ready || running) return;
    running = true;
    progress = 0;
    summary = null;
    error = null;
    message = 'Opening local CSV file';
    try {
      await invoke('inspect_csv', { source: sourcePath });
    } catch (cause) {
      if (!error) error = String(cause);
      running = false;
    }
  }
</script>

<section class="card">
  <h2>CSV-Datensatz untersuchen</h2>
  <p>Wähle eine lokale CSV-Datei. FAIRmorph liest Spalten und Datensatzanzahl, prüft die Struktur und berechnet einen SHA-256-Fingerabdruck. Die Datei bleibt unverändert.</p>

  <div class="file-row">
    <button type="button" onclick={chooseFile} disabled={running}>CSV auswählen</button>
    <span class="path" title={sourcePath}>{sourcePath || 'Noch keine Datei ausgewählt'}</span>
  </div>
  <button class="primary" type="button" onclick={inspect} disabled={!ready || !sourcePath || running}>
    {running ? 'Untersuche Datei …' : 'CSV untersuchen'}
  </button>

  {#if running || progress > 0}
    <div class="progress" aria-live="polite">
      <label for="csv-progress">Fortschritt: {progress.toFixed(0)} %</label>
      <progress id="csv-progress" value={progress} max="100"></progress>
      <span>{message}</span>
    </div>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if summary}
    <div class="result">
      <h3>Ergebnis</h3>
      <p><strong>Datensätze:</strong> {summary.processed_records.toLocaleString('de-DE')}</p>
      <p><strong>Spalten ({summary.columns.length}):</strong></p>
      <ul>
        {#each summary.columns as column}
          <li><code>{column}</code></li>
        {/each}
      </ul>
      <p><strong>SHA-256 der Quelldatei:</strong></p>
      <code class="hash">{summary.dataset_sha256}</code>
    </div>
  {/if}
</section>

<style>
  .card { width: min(100%, 650px); box-sizing: border-box; margin: 1rem auto; padding: 1.5rem; border: 1px solid #dbe3eb; border-radius: 10px; background: #fff; color: #172233; }
  h2 { margin-top: 0; }
  p { line-height: 1.5; }
  .file-row { display: flex; align-items: center; flex-wrap: wrap; gap: 0.75rem; margin-bottom: 1rem; }
  .path { min-width: 0; flex: 1; overflow-wrap: anywhere; font-size: 0.85rem; }
  button { padding: 0.55rem 0.9rem; background: #e1edf5; border: 0; border-radius: 6px; cursor: pointer; }
  button.primary { background: #0369a1; color: white; }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  .progress { margin-top: 1rem; display: grid; gap: 0.4rem; }
  progress { width: 100%; }
  .error { color: #a11919; background: #fff1f1; padding: 0.75rem; border-radius: 6px; }
  .result { margin-top: 1rem; background: #f0fdf4; padding: 1rem; border-radius: 6px; }
  .result ul { padding-left: 1.4rem; }
  .hash { overflow-wrap: anywhere; font-size: 0.8rem; }
</style>
