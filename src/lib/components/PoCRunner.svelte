<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  // Typdefinitionen für das Python-Protokoll
  interface ProgressPayload {
    event: 'progress';
    stage: string;
    percentage: number;
    message: string;
  }

  interface ResultPayload {
    event: 'result';
    status: string;
    data: {
      processed_records: number;
      engine_python_version: string;
      dataset_sha256: string;
      completed_at: string;
      loaded_file: string;
    };
  }

  type EngineEvent = ProgressPayload | ResultPayload;

  // Svelte 5 Runes State (für Svelte 4: einfach "let records = 1000;" etc. nutzen)
  let records = $state(1000);
  let isRunning = $state(false);
  let currentStage = $state('');
  let currentMessage = $state('');
  let progressPercent = $state(0);
  let resultData = $state<ResultPayload['data'] | null>(null);
  let errorMessage = $state<string | null>(null);

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    // Registriere Event-Listener für stdout-Events aus Rust
    unlisten = await listen<string>('poc-event', (event) => {
      try {
        const payload: EngineEvent = JSON.parse(event.payload);

        if (payload.event === 'progress') {
          currentStage = payload.stage;
          progressPercent = payload.percentage;
          currentMessage = payload.message;
        } else if (payload.event === 'result') {
          progressPercent = 100;
          resultData = payload.data;
          isRunning = false;
        }
      } catch (err) {
        console.error('Failed to parse NDJSON payload:', event.payload, err);
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function startBenchmark() {
    isRunning = true;
    errorMessage = null;
    resultData = null;
    progressPercent = 0;
    currentMessage = 'Starte Prozess...';

    try {
      await invoke('run_poc', { records });
    } catch (err) {
      errorMessage = String(err);
      isRunning = false;
    }
  }
</script>

<div class="card">
  <h2>Engine IPC Test (PoC)</h2>
  <p class="description">
    Startet den Subprozess <code>fairmorph-engine poc</code> und streamt NDJSON-Events live in die UI.
  </p>

  <div class="control-row">
    <label for="records">Datensätze:</label>
    <input
      id="records"
      type="number"
      min="10"
      max="1000000"
      step="100"
      bind:value={records}
      disabled={isRunning}
    />
    <button onclick={startBenchmark} disabled={isRunning}>
      {isRunning ? 'Läuft...' : 'Benchmark starten'}
    </button>
  </div>

  {#if isRunning || progressPercent > 0}
    <div class="progress-section">
      <div class="progress-labels">
        <span class="stage-badge">{currentStage || 'PENDING'}</span>
        <span class="percent">{progressPercent.toFixed(0)}%</span>
      </div>
      <div class="progress-bar-bg">
        <div class="progress-bar-fill" style="width: {progressPercent}%;"></div>
      </div>
      <p class="status-msg">{currentMessage}</p>
    </div>
  {/if}

  {#if errorMessage}
    <div class="error-box">
      <strong>Fehler beim IPC-Aufruf:</strong>
      <pre>{errorMessage}</pre>
    </div>
  {/if}

  {#if resultData}
    <div class="result-box">
      <h3>Ergebnis der Transformation</h3>
      <table>
        <tbody>
          <tr>
            <td><strong>Python Engine Version:</strong></td>
            <td><code>{resultData.engine_python_version}</code></td>
          </tr>
          <tr>
            <td><strong>Verarbeitete Datensätze:</strong></td>
            <td>{resultData.processed_records.toLocaleString()}</td>
          </tr>
          <tr>
            <td><strong>Dataset SHA-256:</strong></td>
            <td><code>{resultData.dataset_sha256}</code></td>
          </tr>
          <tr>
            <td><strong>Abgeschlossen um:</strong></td>
            <td>{new Date(resultData.completed_at).toLocaleTimeString()}</td>
          </tr>
          <tr>
            <td><strong>Ausgeführte Datei:</strong></td>
            <td>{resultData.loaded_file}</td>
          </tr>
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .card {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 1.5rem;
    max-width: 600px;
    margin: 1rem auto;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
    font-family: inherit;
  }

  h2 {
    margin-top: 0;
    font-size: 1.25rem;
    color: #0f172a;
  }

  .description {
    color: #64748b;
    font-size: 0.875rem;
    margin-bottom: 1.5rem;
  }

  .control-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  input {
    padding: 0.45rem 0.6rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.9rem;
    width: 120px;
  }

  button {
    background: #0284c7;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.5rem 1rem;
    font-weight: 500;
    cursor: pointer;
  }

  button:disabled {
    background: #94a3b8;
    cursor: not-allowed;
  }

  .progress-section {
    margin: 1rem 0;
    padding: 1rem;
    background: #f8fafc;
    border-radius: 6px;
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .stage-badge {
    background: #e2e8f0;
    color: #334155;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .percent {
    font-weight: 600;
    color: #0284c7;
  }

  .progress-bar-bg {
    width: 100%;
    height: 8px;
    background: #e2e8f0;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: #0284c7;
    transition: width 0.3s ease;
  }

  .status-msg {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: #475569;
  }

  .result-box {
    margin-top: 1.5rem;
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
    border-radius: 6px;
    padding: 1rem;
  }

  .result-box h3 {
    margin-top: 0;
    color: #166534;
    font-size: 1rem;
  }

  table {
    width: 100%;
    font-size: 0.85rem;
    border-collapse: collapse;
  }

  td {
    padding: 0.3rem 0;
    color: #1e293b;
  }

  code {
    background: #e2e8f0;
    padding: 0.15rem 0.3rem;
    border-radius: 3px;
    font-size: 0.8rem;
    word-break: break-all;
  }

  .error-box {
    margin-top: 1rem;
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #991b1b;
    padding: 0.75rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }
</style>