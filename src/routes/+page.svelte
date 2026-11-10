<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    HandshakeResult, ProgressEvent, RunResult,
    type Progress, type RunSummary
  } from '$lib/engine-protocol';

  let project = $state('');
  let busy = $state(false);
  let engineVersion = $state('');
  let error = $state('');
  let progress = $state<Progress | null>(null);
  let run = $state<RunSummary | null>(null);

  onMount(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    listen<unknown>('engine-progress', ({ payload }) => {
      if (!busy || disposed) return;
      const parsed = ProgressEvent.safeParse(payload);
      if (parsed.success) progress = parsed.data;
      else error = 'Ungültiges Fortschrittsereignis der Engine.';
    }).then(stop => {
      if (disposed) stop();
      else unsubscribe = stop;
    }).catch(e => error = String(e));
    return () => { disposed = true; unsubscribe?.(); };
  });

  async function checkEngine() {
    busy = true;
    error = '';
    try {
      const response = HandshakeResult.parse(await invoke('engine_handshake'));
      if (!response.commands.includes('run-pipeline')) throw new Error('Engine unterstützt run-pipeline nicht.');
      engineVersion = response.engine_version;
    } catch (e) {
      engineVersion = '';
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function startRun() {
    if (!project.trim() || busy) return;
    busy = true;
    error = '';
    run = null;
    progress = null;
    try {
      const response = await invoke('run_project', { project: project.trim() });
      run = RunResult.parse(response);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>FAIRmorph Workbench · RUN/2</title>
</svelte:head>

<main>
  <h1>FAIRmorph Workbench</h1>
  <p class="intro">Vertikaler v0.13-PoC: Ein bestehendes Projekt ausführen und die Engine-Ergebnisse anzeigen.</p>

  <section aria-labelledby="engine-title">
    <h2 id="engine-title">Engine</h2>
    <button disabled={busy} onclick={checkEngine}>Verbindung prüfen</button>
    {#if engineVersion}<p>Engine-Version: <code>{engineVersion}</code></p>{/if}
  </section>

  <section aria-labelledby="run-title">
    <h2 id="run-title">Bestätigten Projekt-RUN starten</h2>
    <label for="project">Projektverzeichnis</label>
    <input id="project" type="text" bind:value={project} disabled={busy}
      placeholder="/pfad/zum/fairmorph-projekt" autocomplete="off" />
    <p class="note">Die Workbench übergibt nur den Projektpfad. Die Engine liest die ausgewählte,
      freigegebene Pipeline und alle fachlichen Konfigurationen selbst aus dem Projekt.
      Die Workbench schreibt keine gepinnten Revisionen oder RUN-Dateien.</p>
    <button disabled={busy || !project.trim()} onclick={startRun}>
      {busy ? 'Engine arbeitet …' : 'RUN starten'}
    </button>
    {#if progress}
      <p role="status">{progress.stage}: {progress.percentage}% — {progress.message}</p>
    {/if}
    {#if error}<p role="alert" class="error">{error}</p>{/if}
    {#if run}
      <div aria-live="polite">
        <h3>RUN abgeschlossen</h3>
        <p>RUN-ID: <code>{run.run_id}</code></p>
        <table>
          <thead><tr><th>Ausgabetabelle</th><th>Zeilen</th><th>Spalten</th></tr></thead>
          <tbody>
            {#each run.outputs as output}
              <tr><td>{output.name}</td><td>{output.rows}</td><td>{output.columns.join(', ')}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
  <p class="note">Kein EML-Export und keine automatische fachliche Freigabe: diese Erweiterungen
    erfordern erst einen versionierten Projektkonfigurations- und Exportvertrag.</p>
</main>

<style>
  main { max-width: 850px; margin: 2rem auto; padding: 0 1.5rem; font-family: system-ui, sans-serif; }
  .intro, .note { color: #475569; line-height: 1.5; }
  .note { font-size: .9rem; }
  section { border: 1px solid #cbd5e1; padding: 1.25rem; border-radius: 8px; margin: 1rem 0; }
  label { display: block; margin-bottom: .5rem; font-weight: 600; }
  input { display: block; box-sizing: border-box; width: 100%; padding: .6rem; margin-bottom: .75rem; }
  button { padding: .6rem .9rem; cursor: pointer; }
  button:disabled { cursor: not-allowed; opacity: .5; }
  .error { color: #9f1239; white-space: pre-wrap; overflow-wrap: anywhere; }
  table { width: 100%; border-collapse: collapse; text-align: left; }
  th, td { border-bottom: 1px solid #cbd5e1; padding: .5rem; }
  code { overflow-wrap: anywhere; }
</style>
