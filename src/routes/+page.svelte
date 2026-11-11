<script lang="ts">
  import { onMount } from 'svelte';
  import { canInitializeProject, importsReady, pipelineBindingsComplete, runButtonReady, sourceFileName, type SourceFile } from '$lib/project-setup';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    ApprovedDatasetMetadata, DatasetMetadataStatus, HandshakeResult,
    ProgressEvent, RunResult, StagedDatasetMetadata, CreatedProject, ImportedResource, PipelineTemplate, SavedPipelineDraft, RegisteredPipeline, SelectedPipeline, ProjectVerification, ExistingProjectResources,
    type DatasetMetadata, type Progress, type RunSummary, type PipelineTemplateInfo, type ProjectVerificationResult, type ExistingProjectResource
  } from '$lib/engine-protocol';

  let project = $state('');
  let newName = $state('');
  let parentFolder = $state('');
  let folderName = $state('');
  let setupProject = $state('');
  let setupFiles = $state<SourceFile[]>([]);
  let setupMessage = $state('');
  let pipelineTemplate = $state<PipelineTemplateInfo | null>(null);
  let pipelineBindings = $state<Record<string, string>>({});
  let pipelineFormats = $state<Record<string, 'csv' | 'parquet'>>({});
  let pipelineSaved = $state(false);
  let pipelineSha = $state('');
  let selectedPipelineSha = $state('');
  let resourceProject = $state('');
  let projectResources = $state<ExistingProjectResource[]>([]);
  let resourcesBusy = $state(false);
  let resourcesMessage = $state('');
  let resourcesError = $state('');
  let canConfigurePipeline = $derived(!!project.trim() && project.trim() === resourceProject && projectResources.length > 0);
  let canRunSelectedPipeline = $derived(runButtonReady(project, resourceProject, selectedPipelineSha));
  let pipelineBusy = $state(false);
  let pipelineError = $state('');
  let pipelineMessage = $state('');
  let bindingsComplete = $derived(pipelineBindingsComplete(pipelineTemplate?.inputs,
    projectResources.map(resource => ({ imported: true, resourceId: resource.resource_id })),
    pipelineBindings, pipelineFormats));
  let setupBusy = $state(false);
  let setupError = $state('');
  let setupReady = $derived(!!setupProject && setupFiles.some(file => file.imported));
  let canInitialize = $derived(canInitializeProject(newName, parentFolder, folderName));
  let busy = $state(false);
  let engineVersion = $state('');
  let error = $state('');
  let information = $state('');
  let progress = $state<Progress | null>(null);
  let run = $state<RunSummary | null>(null);
  let verification = $state<ProjectVerificationResult | null>(null);
  let verificationError = $state('');
  let verificationBusy = $state(false);
  let dataset = $state<DatasetMetadata | null>(null);
  let title = $state('');
  let confirmingUser = $state('');
  let stagedSha = $state<string | null>(null);
  let savedDraft = $state(false);
  let currentProject = $state('');

  function invalidatePipeline() {
    pipelineTemplate = null;
    pipelineBindings = {};
    pipelineFormats = {};
    pipelineSaved = false;
    pipelineSha = '';
    selectedPipelineSha = '';
    pipelineError = '';
    pipelineMessage = '';
    resourceProject = '';
    projectResources = [];
    resourcesMessage = '';
    resourcesError = '';
  }

  function invalidateEditor() {
    verification = null;
    verificationError = '';
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

  async function loadExistingResources() {
    const target = project.trim();
    if (!target) {
      resourcesError = 'Bitte zuerst einen Projektordner auswählen oder einen Projektpfad eintragen.';
      return;
    }
    if (busy || setupBusy || pipelineBusy || verificationBusy || resourcesBusy) {
      resourcesError = 'Der Ladevorgang ist derzeit durch eine andere laufende Aktion gesperrt.';
      return;
    }
    resourcesBusy = true;
    invalidatePipeline();
    error = '';
    resourcesMessage = 'Importierte Projektressourcen werden eingelesen …';
    try {
      const result = ExistingProjectResources.parse(await invoke('list_project_resources', { project: target }));
      if (project.trim() === target) {
        projectResources = result.resources;
        resourceProject = target;
        resourcesMessage = result.resources.length
          ? result.resources.length + ' importierte Ressource(n) geladen. Die technische Pipeline kann konfiguriert werden.'
          : 'Das Projekt wurde gelesen, enthält aber noch keine importierten Ressourcen.';
      }
    } catch (e) {
      if (project.trim() === target) {
        resourcesMessage = '';
        resourcesError = 'Projektressourcen konnten nicht geladen werden: ' + String(e);
      }
    } finally {
      resourcesBusy = false;
    }
  }

  async function chooseExistingProject() {
    if (busy || setupBusy || pipelineBusy || verificationBusy || resourcesBusy) return;
    error = '';
    try {
      const selected = await invoke<string | null>('choose_existing_project');
      // Cancellation must preserve the current selection and editor state.
      if (selected) {
        if (selected !== project.trim()) {
          project = selected;
          invalidateEditor();
          invalidatePipeline();
          run = null;
          progress = null;
        }
        // Re-selecting the same directory must refresh resource discovery too.
        await loadExistingResources();
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function chooseParent() {
    if (busy || setupBusy) return;
    setupError = '';
    try {
      const selected = await invoke<string | null>('choose_project_parent');
      if (selected) parentFolder = selected;
    } catch (e) {
      setupError = String(e);
    }
  }

  async function addSources() {
    if (!setupProject || setupBusy || busy) return;
    setupError = '';
    try {
      const paths = await invoke<string[]>('choose_source_files');
      for (const path of paths) {
        if (!setupFiles.some(file => file.path === path)) {
          setupFiles = [...setupFiles, { path, role: '', imported: false, checksum: '', resourceId: '' }];
        }
      }
    } catch (e) {
      setupError = String(e);
    }
  }

  async function initializeNewProject() {
    if (!canInitialize || busy || setupBusy || setupProject) return;
    setupBusy = true;
    setupError = '';
    setupMessage = '';
    const folder = parentFolder.replace(/[\\/]$/, '');
    const target = folder + (folder.endsWith(':') ? '/' : '/') + folderName.trim();
    try {
      const created = CreatedProject.parse(await invoke('create_project', {
        project: target,
        name: newName.trim()
      }));
      setupProject = created.project_path;
      project = created.project_path;
      invalidateEditor();
      invalidatePipeline();
      setupMessage = 'Projekt angelegt. Bitte Originaldateien auswählen und deren Rollen festlegen.';
    } catch (e) {
      setupError = String(e);
    } finally {
      setupBusy = false;
    }
  }

  async function importSources() {
    if (!setupProject || busy || setupBusy) return;
    const pending = setupFiles.filter(file => !file.imported);
    if (!importsReady(setupFiles)) {
      setupError = 'Jeder noch nicht importierten Datei muss explizit events oder counts zugeordnet werden.';
      return;
    }
    setupBusy = true;
    setupError = '';
    setupMessage = '';
    try {
      for (const file of pending) {
        const imported = ImportedResource.parse(await invoke('import_project_source', {
          project: setupProject,
          source: file.path,
          role: file.role
        }));
        setupFiles = setupFiles.map(item => item.path === file.path
          ? { ...item, imported: true, checksum: imported.sha256, resourceId: imported.resource_id }
          : item);
      }
      if (project.trim() === setupProject) {
        resourceProject = setupProject;
        projectResources = setupFiles.filter(item => item.imported).map(item => ({
          original_name: sourceFileName(item.path), role: item.role,
          resource_id: item.resourceId, sha256: item.checksum
        }));
      }
      setupMessage = 'Dateien unverändert importiert. Das Projekt kann jetzt konfiguriert werden.';
    } catch (e) {
      setupError = String(e) + ' Erfolgreiche Importe bleiben registriert; fehlgeschlagene Dateien können erneut importiert werden.';
    } finally {
      setupBusy = false;
    }
  }

  function removePendingSource(path: string) {
    setupFiles = setupFiles.filter(file => file.path !== path || file.imported);
  }

  async function chooseTechnicalTemplate() {
    if (!canConfigurePipeline || busy || setupBusy || pipelineBusy || resourcesBusy) return;
    pipelineBusy = true;
    pipelineError = '';
    pipelineMessage = '';
    try {
      const result = await invoke<unknown | null>('choose_pipeline_template');
      if (result) {
        const template = PipelineTemplate.parse(result);
        pipelineTemplate = template;
        pipelineBindings = {};
        pipelineFormats = Object.fromEntries(
          template.inputs.map(input => [input.name, input.format])
        );
        pipelineSaved = false;
        pipelineSha = '';
        selectedPipelineSha = '';
      }
    } catch (e) {
      pipelineError = String(e);
    } finally {
      pipelineBusy = false;
    }
  }

  function resetPipelineDraft() {
    pipelineSaved = false;
    pipelineSha = '';
    selectedPipelineSha = '';
    pipelineMessage = '';
  }

  async function saveTechnicalPipeline() {
    if (!pipelineTemplate || !bindingsComplete || !canConfigurePipeline || busy || setupBusy || pipelineBusy || resourcesBusy) return;
    pipelineBusy = true;
    pipelineError = '';
    try {
      const bindings = pipelineTemplate.inputs.map(input => ({
        slot: input.name,
        resource_id: pipelineBindings[input.name],
        format: pipelineFormats[input.name]
      }));
      SavedPipelineDraft.parse(await invoke('save_project_pipeline_draft', {
        project: resourceProject,
        template: pipelineTemplate.path,
        bindings
      }));
      pipelineSaved = true;
      pipelineMessage = 'Technische Pipeline-Konfiguration als bearbeitbarer Projekt-Draft gespeichert.';
    } catch (e) {
      pipelineError = String(e);
    } finally {
      pipelineBusy = false;
    }
  }

  async function registerTechnicalPipeline() {
    if (!pipelineSaved || !canConfigurePipeline || busy || setupBusy || pipelineBusy || resourcesBusy) return;
    pipelineBusy = true;
    pipelineError = '';
    try {
      const registered = RegisteredPipeline.parse(await invoke('register_project_pipeline', {
        project: resourceProject
      }));
      pipelineSha = registered.pipeline_sha256;
      pipelineMessage = 'Unveränderliche Pipeline-Revision registriert. Zur Ausführung noch explizit auswählen.';
    } catch (e) {
      pipelineError = String(e);
    } finally {
      pipelineBusy = false;
    }
  }

  async function selectTechnicalPipeline() {
    if (!pipelineSha || !canConfigurePipeline || busy || setupBusy || pipelineBusy || resourcesBusy) return;
    pipelineBusy = true;
    pipelineError = '';
    try {
      const selected = SelectedPipeline.parse(await invoke('select_registered_pipeline', {
        project: resourceProject,
        pipelineSha256: pipelineSha
      }));
      selectedPipelineSha = selected.active_pipeline_sha256;
      pipelineMessage = 'Technische Pipeline aktiv. Ein RUN wird nicht automatisch gestartet.';
    } catch (e) {
      pipelineError = String(e);
    } finally {
      pipelineBusy = false;
    }
  }

  async function checkEngine() {
    busy = true;
    error = '';
    try {
      const response = HandshakeResult.parse(await invoke('engine_handshake'));
      for (const command of [
        'init-project', 'import-resource', 'register-pipeline', 'select-pipeline', 'run-pipeline', 'show-dataset-metadata',
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

  async function verifyCurrentProject() {
    const target = project.trim();
    if (!target || busy || setupBusy || pipelineBusy || verificationBusy) return;
    verificationBusy = true;
    verification = null;
    verificationError = '';
    try {
      const result = ProjectVerification.parse(await invoke('verify_project', { project: target }));
      // Never display a result belonging to a project that the user has since changed.
      if (project.trim() === target) verification = result;
    } catch (e) {
      if (project.trim() === target) verificationError = String(e);
    } finally {
      verificationBusy = false;
    }
  }

  async function startRun() {
    if (!canRunSelectedPipeline || busy || setupBusy || pipelineBusy || verificationBusy || !!stagedSha) return;
    busy = true;
    error = '';
    run = null;
    verification = null;
    verificationError = '';
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

  <section aria-labelledby="setup-title">
    <h2 id="setup-title">Neues Projekt · Einrichtung</h2>
    <p class="note">Vorhandene FAIRmorph-Engine: Projekt initialisieren, Originaldateien importieren und technische Rollen ausdrücklich zuweisen. Keine automatische wissenschaftliche Freigabe.</p>
    <label for="setup-name">Projektname</label>
    <input id="setup-name" bind:value={newName} disabled={setupBusy || !!setupProject} placeholder="Mein Forschungsprojekt" />
    <label for="setup-parent">Übergeordnetes Verzeichnis</label>
    <div class="setup-inline">
      <input id="setup-parent" bind:value={parentFolder} disabled={setupBusy || !!setupProject} placeholder="Absoluter Pfad zum Speicherort" />
      <button onclick={chooseParent} disabled={setupBusy || !!setupProject || busy}>Ordner auswählen</button>
    </div>
    <label for="setup-folder">Name des neuen Projektordners</label>
    <input id="setup-folder" bind:value={folderName} disabled={setupBusy || !!setupProject} placeholder="MeinProjekt.fairmorph" />
    <button onclick={initializeNewProject} disabled={!canInitialize || setupBusy || !!setupProject || busy}>
      {setupBusy ? 'Engine arbeitet …' : 'Projekt anlegen'}
    </button>
    {#if setupProject}
      <p role="status"><strong>Projekt angelegt:</strong> <code>{setupProject}</code></p>
      <h3>Originaldateien und Ressourcenrollen</h3>
      <p class="note">Jede Originaldatei braucht eine ausdrücklich gewählte Rolle. Die Rollen <code>events</code> und <code>counts</code> sind zunächst technische Ressourcenbezeichnungen, keine bestätigte wissenschaftliche Entity-Zuordnung.</p>
      <button onclick={addSources} disabled={setupBusy || busy}>Dateien auswählen</button>
      {#each setupFiles as file (file.path)}
        <div class="source-row">
          <div class="source-info">
            <strong>{sourceFileName(file.path)}</strong>
            <span class="note">{file.path}</span>
            {#if file.imported}
              <span class="note">Importiert · SHA-256: <code>{file.checksum}</code></span>
            {/if}
          </div>
          <label>
            <span>Rolle</span>
            <select disabled={file.imported || setupBusy} value={file.role}
              onchange={(event) => {
                const role = event.currentTarget.value as '' | 'events' | 'counts';
                setupFiles = setupFiles.map(item => item.path === file.path ? { ...item, role } : item);
              }}>
              <option value="">Bitte wählen</option>
              <option value="events">events – Ereignisse</option>
              <option value="counts">counts – Zählungen</option>
            </select>
          </label>
          {#if !file.imported}
            <button onclick={() => removePendingSource(file.path)} disabled={setupBusy}>Entfernen</button>
          {/if}
        </div>
      {/each}
      <button onclick={importSources} disabled={setupBusy || busy || !importsReady(setupFiles)}>
        Originaldateien importieren
      </button>
      {#if setupReady}
        <p class="note">Das Projekt ist initialisiert und enthält importierte Ressourcen. Für RUN/2 muss anschließend eine deklarative Pipeline registriert und ausgewählt werden. Der vorhandene RUN-Bereich bleibt dafür bestehen.</p>
      {/if}
    {/if}
    {#if setupMessage}<p role="status">{setupMessage}</p>{/if}
    {#if setupError}<p role="alert" class="error">{setupError}</p>{/if}
  </section>

  <section aria-labelledby="project-title">
    <h2 id="project-title">Projekt</h2>
    <label for="project">Projektverzeichnis</label>
    <div class="setup-inline">
      <input id="project" type="text" bind:value={project}
        disabled={busy || setupBusy || pipelineBusy || verificationBusy || resourcesBusy}
        oninput={() => { invalidateEditor(); invalidatePipeline(); run = null; progress = null; }}
        placeholder="/pfad/zum/fairmorph-projekt" autocomplete="off" />
      <button onclick={chooseExistingProject}
        disabled={busy || setupBusy || pipelineBusy || verificationBusy || resourcesBusy}>Projektordner auswählen</button>
    </div>
    <p class="note">Bestehenden Projektordner mit <code>project.json</code> auswählen oder den Pfad manuell eingeben. Nur der Projektpfad wird an den Engine-Prozess übergeben. Fachliche Werte
      werden ausschließlich in Projektdateien gespeichert und von der Engine selbst gelesen.</p>
    <button type="button" onclick={loadExistingResources} disabled={resourcesBusy || !project.trim()}>
      {resourcesBusy ? 'Ressourcen werden geladen …' : 'Projektressourcen laden'}
    </button>
    {#if busy || setupBusy || pipelineBusy || verificationBusy}
      <p class="note" role="status">Während einer anderen Engine-Aktion ist das Laden von Projektressourcen nicht möglich.</p>
    {/if}
    {#if resourcesMessage}<p role="status">{resourcesMessage}</p>{/if}
    {#if resourcesError}<p role="alert" class="error">{resourcesError}</p>{/if}
    {#if !resourcesMessage && !resourcesError}
      <p class="note">Nach der Projektauswahl werden importierte Ressourcen geladen. Bei manueller Pfadeingabe bitte „Projektressourcen laden“ wählen.</p>
    {/if}
    <button onclick={loadDataset} disabled={busy || !project.trim()}>Projektmetadaten laden</button>
  </section>

  {#if canConfigurePipeline}
    <section aria-labelledby="pipeline-title">
      <h2 id="pipeline-title">Technische Pipeline konfigurieren</h2>
      <p class="note">Eine bestehende deklarative <code>fairmorph.pipeline/1</code>-JSON-Vorlage auswählen – auch für ein bereits geöffnetes Projekt. Keine Feldzuordnung, wissenschaftliche Beschreibung oder automatische Interpretation. Jede Eingangsressource wird ausdrücklich mit einer importierten Datei verknüpft.</p>
      <button onclick={chooseTechnicalTemplate} disabled={busy || setupBusy || pipelineBusy}>Pipeline-Vorlage auswählen (JSON)</button>
      {#if pipelineTemplate}
        <p><strong>Vorlage:</strong> <code>{pipelineTemplate.path}</code></p>
        <p class="note">{pipelineTemplate.steps} Schritte · Ausgaben: {pipelineTemplate.outputs.join(', ')}</p>
        {#each pipelineTemplate.inputs as input (input.name)}
          <div class="source-row">
            <div class="source-info"><strong>Eingang: {input.name}</strong><span class="note">Vorlagenformat: {input.format}</span></div>
            <label>
              <span>Importierte Datei</span>
              <select value={pipelineBindings[input.name] ?? ''} disabled={pipelineBusy || !!selectedPipelineSha}
                onchange={event => {
                  pipelineBindings = { ...pipelineBindings, [input.name]: event.currentTarget.value };
                  resetPipelineDraft();
                }}>
                <option value="">Bitte ausdrücklich zuordnen</option>
                {#each projectResources as resource (resource.resource_id)}
                  <option value={resource.resource_id}>{resource.original_name} ({resource.role})</option>
                {/each}
              </select>
            </label>
            <label>
              <span>Quellformat</span>
              <select value={pipelineFormats[input.name] ?? input.format} disabled={pipelineBusy || !!selectedPipelineSha}
                onchange={event => {
                  pipelineFormats = { ...pipelineFormats, [input.name]: event.currentTarget.value as 'csv' | 'parquet' };
                  resetPipelineDraft();
                }}>
                <option value="csv">CSV</option>
                <option value="parquet">Parquet</option>
              </select>
            </label>
          </div>
        {/each}
        <button onclick={saveTechnicalPipeline} disabled={!bindingsComplete || pipelineBusy || busy || setupBusy || !!selectedPipelineSha}>1. Konfiguration speichern</button>
        <button onclick={registerTechnicalPipeline} disabled={!pipelineSaved || pipelineBusy || busy || setupBusy || !!selectedPipelineSha}>2. Pipeline registrieren</button>
        <button onclick={selectTechnicalPipeline} disabled={!pipelineSha || pipelineBusy || busy || setupBusy || !!selectedPipelineSha}>3. Pipeline auswählen</button>
        {#if pipelineSha}<p class="note">Registrierte SHA-256: <code>{pipelineSha}</code></p>{/if}
        {#if selectedPipelineSha}<p role="status">Pipeline ausgewählt. Der technische RUN kann unten separat gestartet werden.</p>{/if}
      {/if}
      {#if pipelineMessage}<p role="status">{pipelineMessage}</p>{/if}
      {#if pipelineError}<p role="alert" class="error">{pipelineError}</p>{/if}
    </section>
  {/if}

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
    <button disabled={!canRunSelectedPipeline || busy || setupBusy || pipelineBusy || verificationBusy || !!stagedSha}
      onclick={startRun}>{busy ? 'Engine arbeitet …' : canRunSelectedPipeline ? 'Technischen RUN starten' : 'Pipeline auswählen, um RUN zu starten'}</button>
    {#if !canRunSelectedPipeline}
      <p class="note" role="status">Noch nicht RUN-bereit: Im aktuellen Projekt zuerst eine technische Pipeline-Vorlage auswählen, die Ressourcen zuordnen und die registrierte Pipeline aktivieren.</p>
    {/if}
    <p class="note">Die Engine prüft die bereits ausgewählte Pipeline und fixiert die aktive
      bestätigte Datensatztitelrevision im neuen RUN-Snapshot. Unbestätigte Drafts bleiben unwirksam.</p>
    <div>
      <button disabled={!project.trim() || busy || setupBusy || pipelineBusy || verificationBusy}
        onclick={verifyCurrentProject}>{verificationBusy ? 'Integrität wird geprüft …' : 'Projektintegrität prüfen'}</button>
      <span class="note">Unabhängig vom RUN; prüft das aktuelle Projekt, ohne einen RUN zu starten.</span>
    </div>
    {#if verification && verification.project_path === project.trim()}
      <div role="status" aria-label="Ergebnis der Projektintegritätsprüfung">
        <p><strong>Integrität bestätigt:</strong> {verification.status}</p>
        <p>Projekt: <code>{verification.project_path}</code></p>
        <p>Quellobjekte: <strong>{verification.source_objects}</strong> · Abgeschlossene RUNs: <strong>{verification.completed_runs}</strong> · Exporte: <strong>{verification.exports}</strong></p>
        <p class="note">Technische Integrität geprüft; keine Aussage über fachliche oder wissenschaftliche Validität.</p>
      </div>
    {/if}
    {#if verificationError}<p role="alert" class="error">Integritätsprüfung fehlgeschlagen: {verificationError}</p>{/if}
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
  .setup-inline { display: flex; align-items: start; gap: .5rem; flex-wrap: wrap; }
  .setup-inline input { flex: 1; min-width: 250px; }
  .source-row { border-top: 1px solid #cbd5e1; padding: .8rem 0; display: flex; align-items: end; gap: .7rem; flex-wrap: wrap; }
  .source-info { flex: 1; min-width: 220px; display: flex; flex-direction: column; gap: .3rem; overflow-wrap: anywhere; }
  .source-row label { min-width: 160px; }
  select { display: block; width: 100%; padding: .6rem; margin-top: .3rem; }
</style>
