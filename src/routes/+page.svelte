<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    ApprovedDatasetMetadata, DatasetMetadataStatus, HandshakeResult,
    ProgressEvent, RunResult, StagedDatasetMetadata,
    type DatasetMetadata, type Progress, type RunSummary
  } from '$lib/engine-protocol';

  let project = $state('');
  let busy = $state(false);
  let engineVersion = $state('');
  let error = $state('');
  let information = $state('');
  let progress = $state<Progress | null>(null);
  let run = $state<RunSummary | null>(null);
  let dataset = $state<DatasetMetadata | null>(null);
  let title = $state('');
  let confirmingUser = $state('');
  let stagedSha = $state<string | null>(null);
  let savedDraft = $state(false);
  let currentProject = $state('');

  function invalidateEditor() {
    dataset = null;
    stagedSha = null;
    savedDraft = false;
    information = '';
    title = '';
  }

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
      for (const command of [
        'run-pipeline', 'show-dataset-metadata',
        'stage-dataset-metadata', 'approve-dataset-metadata'
      ]) {
        if (!response.commands.includes(command)) {
          throw new Error('Engine unterstützt ' + command + ' nicht.');
        }
      }
      engineVersion = response.engine_version;
    } catch (e) {
      engineVersion = '';
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function loadDataset() {
    if (busy || !project.trim()) return;
    busy = true;
    error = '';
    information = '';
    stagedSha = null;
    savedDraft = false;
    const target = project.trim();
    try {
      const response = DatasetMetadataStatus.parse(
        await invoke('dataset_metadata_status', { project: target })
      );
      dataset = response;
      currentProject = target;
      title = response.title ?? '';
    } catch (e) {
      invalidateEditor();
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function saveDraft() {
    if (busy || !dataset || project.trim() !== currentProject) return;
    busy = true;
    error = '';
    information = '';
    stagedSha = null;
    savedDraft = false;
    try {
      await invoke('save_dataset_metadata_draft', {
        project: currentProject,
        title,
        parentRevisionSha256: dataset.revision_sha256
      });
      savedDraft = true;
      information = 'Entwurf gespeichert, noch nicht registriert oder bestätigt.';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function stageDraft() {
    if (!savedDraft || busy || project.trim() !== currentProject) return;
    busy = true;
    error = '';
    stagedSha = null;
    try {
      const staged = StagedDatasetMetadata.parse(
        await invoke('stage_dataset_metadata', { project: currentProject })
      );
      stagedSha = staged.revision_sha256;
      information = 'Unveränderliche Revision registriert, aber noch nicht freigegeben.';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function confirmDraft() {
    if (!stagedSha || !confirmingUser.trim() || busy || project.trim() !== currentProject) return;
    busy = true;
    error = '';
    const revision = stagedSha;
    try {
      const approved = ApprovedDatasetMetadata.parse(
        await invoke('confirm_dataset_metadata', {
          project: currentProject,
          revisionSha256: revision,
          confirmedBy: confirmingUser.trim()
        })
      );
      dataset = approved;
      title = approved.title;
      stagedSha = null;
      savedDraft = false;
      information = 'Revision ausdrücklich bestätigt und jetzt für neue RUNs aktiv.';
    } catch (e) {
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
      run = RunResult.parse(await invoke('run_project', { project: project.trim() }));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>FAIRmorph Workbench · P0.6</title></svelte:head>
<main>
  <h1>FAIRmorph Workbench</h1>
  <p class="intro">Rahmenwerk v0.13 · Vertikaler PoC mit projektlokalen Konfigurationsrevisionen.</p>

  <section aria-labelledby="engine-title">
    <h2 id="engine-title">Engine-Verbindung</h2>
    <button disabled={busy} onclick={checkEngine}>Verbindung prüfen</button>
    {#if engineVersion}<p>Engine-Version: <code>{engineVersion}</code></p>{/if}
  </section>

  <section aria-labelledby="project-title">
    <h2 id="project-title">Projekt</h2>
    <label for="project">Projektverzeichnis</label>
    <input id="project" type="text" bind:value={project} disabled={busy}
      oninput={invalidateEditor}
      placeholder="/pfad/zum/fairmorph-projekt" autocomplete="off" />
    <p class="note">Nur der Projektpfad wird an den Engine-Prozess übergeben. Fachliche Werte
      werden ausschließlich in Projektdateien gespeichert und von der Engine selbst gelesen.</p>
    <button onclick={loadDataset} disabled={busy || !project.trim()}>Projektmetadaten laden</button>
  </section>

  <section aria-labelledby="metadata-title">
    <h2 id="metadata-title">Datensatztitel · neue Revision</h2>
    {#if dataset && project.trim() === currentProject}
      <p>Aktueller Status: {dataset.status === 'approved' ? 'bestätigt' : 'noch kein Datensatztitel bestätigt'}</p>
      {#if dataset.revision_sha256}
        <p class="note">Aktive Revision: <code>{dataset.revision_sha256}</code></p>
      {/if}
      <label for="title">Datensatztitel</label>
      <input id="title" type="text" bind:value={title} maxlength="255" disabled={busy}
        oninput={() => { stagedSha = null; savedDraft = false; information = ''; }} />
      <button disabled={busy || title.trim().length < 5 || title.trim().length > 255}
        onclick={saveDraft}>1. Entwurf speichern</button>
      <button disabled={busy || !savedDraft} onclick={stageDraft}>2. Revision registrieren</button>
      {#if stagedSha}
        <p class="note">Vorgeschlagene Revision: <code>{stagedSha}</code></p>
        <label for="confirming-user">Verantwortliche Person</label>
        <input id="confirming-user" type="text" bind:value={confirmingUser} disabled={busy} />
        <button disabled={busy || !confirmingUser.trim()} onclick={confirmDraft}>
          3. Diese Revision ausdrücklich bestätigen
        </button>
      {/if}
      {#if information}<p role="status">{information}</p>{/if}
    {:else}
      <p class="note">Zuerst vorhandene Projektmetadaten laden. Kein automatischer Freigabeakt.</p>
    {/if}
  </section>

  <section aria-labelledby="run-title">
    <h2 id="run-title">RUN/2 ausführen</h2>
    <button disabled={busy || !project.trim() || !!stagedSha}
      onclick={startRun}>{busy ? 'Engine arbeitet …' : 'Bestätigten Projekt-RUN starten'}</button>
    <p class="note">Die Engine prüft die bereits ausgewählte Pipeline und fixiert die aktive
      bestätigte Datensatztitelrevision im neuen RUN-Snapshot. Unbestätigte Drafts bleiben unwirksam.</p>
    {#if progress}<p role="status">{progress.stage}: {progress.percentage}% — {progress.message}</p>{/if}
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
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  <p class="note">EML-Export, Feldsemantik-Preview und weitere P0.6/P0.7-Funktionen sind noch offen.</p>
</main>
<style>
  main { max-width: 850px; margin: 2rem auto; padding: 0 1.5rem; font-family: system-ui, sans-serif; }
  .intro, .note { color: #475569; line-height: 1.5; }
  .note { font-size: .9rem; }
  section { border: 1px solid #cbd5e1; padding: 1.25rem; border-radius: 8px; margin: 1rem 0; }
  label { display: block; margin-bottom: .5rem; font-weight: 600; }
  input { display: block; box-sizing: border-box; width: 100%; padding: .6rem; margin-bottom: .75rem; }
  button { padding: .6rem .9rem; margin: 0 .5rem .5rem 0; cursor: pointer; }
  button:disabled { cursor: not-allowed; opacity: .5; }
  .error { color: #9f1239; white-space: pre-wrap; overflow-wrap: anywhere; }
  table { width: 100%; border-collapse: collapse; text-align: left; }
  th, td { border-bottom: 1px solid #cbd5e1; padding: .5rem; }
  code { overflow-wrap: anywhere; }
</style>
