<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { parseEngineEvent, type EngineResultData } from '$lib/engine-protocol';

  let records = $state(1000);
  let listenerReady = $state(false);
  let isRunning = $state(false);
  let currentStage = $state('');
  let currentMessage = $state('');
  let progressPercent = $state(0);
  let resultData = $state<EngineResultData | null>(null);
  let errorMessage = $state<string | null>(null);

  let pendingResult: EngineResultData | null = null;
  let receivedError = false;

  onMount(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    listen<string>('poc-event', ({ payload }) => {
      if (!isRunning) return;
      try {
        const event = parseEngineEvent(payload);
        if (event.event === 'progress') {
          currentStage = event.stage;
          progressPercent = event.percentage;
          currentMessage = event.message;
        } else if (event.status === 'success') {
          pendingResult = event.data;
          currentMessage = 'Engine-Ergebnis empfangen';
        } else {
          errorMessage = event.message;
          receivedError = true;
        }
      } catch {
        errorMessage = 'Ungültiges Ereignis von der Engine empfangen.';
        receivedError = true;
      }
    }).then((stop) => {
      if (disposed) stop();
      else {
        unlisten = stop;
        listenerReady = true;
      }
    }).catch((error) => {
      if (!disposed) errorMessage = 'Engine-Eventlistener konnte nicht gestartet werden: ' + String(error);
    });

    return () => {
      disposed = true;
      listenerReady = false;
      unlisten?.();
    };
  });

  async function startBenchmark() {
    if (!listenerReady || isRunning) return;
    if (!Number.isSafeInteger(records) || records < 10 || records > 1_000_000) {
      errorMessage = 'Bitte eine ganze Datensatzanzahl zwischen 10 und 1.000.000 eingeben.';
      return;
    }

    isRunning = true;
    errorMessage = null;
    resultData = null;
    pendingResult = null;
    receivedError = false;
    currentStage = 'STARTING';
    progressPercent = 0;
    currentMessage = 'Starte Engine-Prozess …';

    try {
      await invoke('run_poc', { records });
      if (!receivedError && pendingResult) {
        resultData = pendingResult;
        progressPercent = 100;
        currentStage = 'COMPLETED';
        currentMessage = 'Engine erfolgreich abgeschlossen';
      } else if (!errorMessage) {
        errorMessage = 'Engine beendet, aber kein gültiges Ergebnis empfangen.';
      }
    } catch (error) {
      if (!errorMessage) errorMessage = 'Engine-Aufruf fehlgeschlagen: ' + String(error);
    } finally {
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
      step="1"
      bind:value={records}
      disabled={isRunning}
    />
    <button onclick={startBenchmark} disabled={isRunning || !listenerReady}>
      {isRunning ? 'Läuft...' : 'Benchmark starten'}
    </button>
  </div>

  {#if isRunning || progressPercent > 0}
    <div class="progress-section" aria-live="polite">
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
    <div class="error-box" role="alert">
      <strong>Fehler beim IPC-Aufruf:</strong>
      <pre>{errorMessage}</pre>
    </div>
  {/if}

  {#if resultData}
    <div class="result-box" aria-live="polite">
      <h3>Ergebnis des Engine-PoC</h3>
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