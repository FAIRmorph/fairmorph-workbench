# FAIRmorph Workbench — User Guide

This guide is for people who want to **operate the currently implemented desktop proof of concept** without knowing its development history. Workbench controls a separately installed local FAIRmorph Engine. Scientific interpretation and data publication are **not** automated by the UI.

**Coverage:** Create/open a project, import two sample CSVs, register/select an existing technical pipeline template, run it explicitly, inspect results, verify the project, and optionally edit a dataset title through a separate confirmation.

## 1. Install and start

You need:

- **Node.js 22+** and **pnpm** (the Workbench's package manager).
- **Rust/Cargo** stable and [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system.
- **Python 3.11+** with a compatible local checkout of [FAIRmorph Engine](https://github.com/FAIRmorph/fairmorph-engine).
- A desktop environment with native Tauri dialogs and a writable directory for projects.

Install the Engine **in a separate directory**, for example as a sibling of Workbench:

```bash
git clone https://github.com/FAIRmorph/fairmorph-engine.git
cd fairmorph-engine
python -m venv .venv
# Linux/macOS: source .venv/bin/activate
# Windows PowerShell: .venv\Scripts\Activate.ps1
python -m pip install -e .
python -m fairmorph_engine.cli handshake
```

Then run the Workbench from its own repository:

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm tauri dev
```

**Important:** Execute the Workbench commands from the Workbench checkout, **not** from the Engine checkout. If the two folders are not siblings, set `FAIRMORPH_ENGINE_PYTHON` to the **absolute path of the Engine virtual environment's Python executable** before starting Tauri, for example:

```bash
export FAIRMORPH_ENGINE_PYTHON=/absolute/path/fairmorph-engine/.venv/bin/python
pnpm tauri dev
```

On Windows PowerShell use `$env:FAIRMORPH_ENGINE_PYTHON = 'C:\path\fairmorph-engine\.venv\Scripts\python.exe'`.

The **development** launcher resolves a sibling Engine interpreter or `FAIRMORPH_ENGINE_PYTHON`. A **release build** looks for an Engine executable bundled in the Tauri `engine/` resources; completing a development run does **not** imply a distributable installer has been validated.

The Workbench opens with a **Connection** section (currently labelled `Engine-Verbindung`). Click **Check Connection** (`Verbindung prüfen`). A successful handshake confirms that the local Engine process can be launched; it does not confirm the scientific validity of any dataset.

## 2. Create a project and import unchanged source files

Create two UTF-8 CSVs on disk **outside** the intended new project folder.

**`events.csv`**

```csv
id,date
1,2026-01-01
2,2026-01-02
```

**`counts.csv`**

```csv
id,count
1,0
2,4
```

In **New Project / Setup** (currently `Neues Projekt · Einrichtung`):

1. Enter a **project name**, e.g. `Synthetic Survey`.
2. Select an **existing parent folder** using the folder dialog, or provide an absolute directory.
3. Enter a **new, nonexistent child folder** name, e.g. `synthetic.fairmorph`.
4. Click **Create Project** (`Projekt anlegen`). The Engine creates `project.json`, the SQLite catalog and project subdirectories.
5. Use **Choose Files** (`Dateien auswählen`) to add both CSVs. For each file, choose its **technical resource role**: `events` for `events.csv` and `counts` for `counts.csv`. The UI does **not** guess roles, and import is disabled until all pending files have an explicit role.
6. Choose **Import Original Files** (`Originaldateien importieren`).

**Expected result:** Two imported resources, each with a real Engine `resource_id` and SHA-256. The two original CSVs are **unchanged** and are stored as immutable content-addressed objects inside the project. There is **no RUN yet**, and **no scientific mapping, biological entity or metadata approval** has been inferred.

For an already created FAIRmorph project, select its existing folder under **Project** (`Projekt`) or provide the path, then use **Load Project Resources** (`Projektressourcen laden`) when necessary. Existing projects must contain a valid `project.json`; selecting a source directory is not the same as selecting a FAIRmorph project.

## 3. Configure a technical pipeline

The current Workbench does **not** create a scientific mapping. Instead, prepare an existing `fairmorph.pipeline/1` technical JSON **template**. The Workbench only binds input roles and formats; it preserves the template's declared operators.

Save the following as `select-events-template.json` **outside** the FAIRmorph project (it is a template to pick in a file dialog). This example describes a simple two-column selection:

```json
{
  "schema_version": "fairmorph.pipeline/1",
  "resources": {
    "events": {
      "resource_id": "REPLACE_WHEN_SELECTED_IN_UI",
      "format": "csv"
    }
  },
  "steps": [
    {
      "id": "select_events",
      "operator": "table.select/1",
      "inputs": ["events"],
      "output": "selected_events",
      "parameters": {
        "columns": ["id", "date"]
      }
    }
  ],
  "outputs": ["selected_events"]
}
```

In **Configure Technical Pipeline** (`Technische Pipeline konfigurieren`):

1. Click **Choose Pipeline Template (JSON)** (`Pipeline-Vorlage auswählen (JSON)`) and select the file.
2. For the declared input `events`, explicitly choose the imported `events.csv` resource and set format to `csv`.
3. Click **1. Save Configuration** (`1. Konfiguration speichern`). This writes only the editable project draft `configurations/drafts/pipeline-technical.json`.
4. Click **2. Register Pipeline** (`2. Pipeline registrieren`). The Engine validates the declaration and returns a 64-character `pipeline_sha256`. It stores a content-addressed registered configuration.
5. Click **3. Select Pipeline** (`3. Pipeline auswählen`). The Engine marks the registered pipeline as active for the project.

**Expected result:** A registered and selected pipeline SHA-256 is visible; the original template steps are unchanged and the Engine owns the registered configuration. **No RUN has started simply because a pipeline was registered or selected.**

The Workbench supports only **technical** templates here. A template containing scientific `entities` or `profile_bindings` cannot be treated as a shortcut to approved entity projections.

## 4. Execute and verify a RUN

In **Execute RUN/2** (`RUN/2 ausführen`), click **Start Technical RUN** (`Technischen RUN starten`) **after** pipeline selection. The Workbench passes only the project path to the Engine's `run-pipeline`; the Engine uses the project's previously registered and selected configuration.

**Expected output for the two-record example:**

| Output table | Rows | Columns |
| --- | ---: | --- |
| `selected_events` | **2** | `id`, `date` |

The UI should show the generated `run_id` and available table summary/column information. Do **not** expect a fixed RUN ID; it is generated at execution time. On disk the Engine stores its completed, checksummed RUN under `runs/<run-id>/`, with a manifest, input/configuration snapshots, output Parquet and audit evidence.

Click **Verify Project Integrity** (`Projektintegrität prüfen`): the Engine's `verify-project` should report a **verified** project. This confirms specified technical integrity checks only. It is **not** a declaration that the source's biological meaning was confirmed.

## 5. Optional: Edit and confirm the dataset title

With the project open:

1. Use **Load Project Metadata** (`Projektmetadaten laden`) to read the currently effective title revision, if any.
2. Enter a new title in **Dataset Title** (`Datensatztitel`). The current UI accepts approximately **5–255 characters**.
3. Click **1. Save Draft** (`1. Entwurf speichern`). The Workbench writes an editable project-local draft file.
4. Click **2. Register Revision** (`2. Revision registrieren`). The Engine validates the draft and returns a content-addressed revision SHA-256. **This is not approval.**
5. Read that exact staged revision and enter the **actual responsible confirming user identity**; then explicitly confirm/approve it. The Workbench writes a separate approval-decision draft; the Engine validates and records the approval and current revision pointer.

A subsequent **new** generic RUN can snapshot the approved effective dataset title. Earlier RUNs retain their original snapshots. The title is descriptive metadata, **not** an automatically inferred ecological field and **not** a generated `eml.xml`. Changing the title does not approve field mappings or scientific publication.

The Engine retains the authoritative immutable revisions and SQLite review records. Manually editing a JSON file or clicking Save does not alter an approved revision.

## 6. What you should see (and what you should not)

| Action | Expected visible evidence | What it does **not** establish |
| --- | --- | --- |
| Connect | Engine handshake/version information | Data or publication validation |
| Import | Resource IDs, original file checksums | Semantic field definitions |
| Register and select technical pipeline | 64-character registered SHA-256 | Successful execution or scientific approval |
| Execute RUN | Generated RUN ID; `selected_events` with two rows | Population estimates or domain Entity records |
| Verify | Engine reports `verified` | Research-author endorsement |
| Stage dataset title | New immutable revision SHA | An approved title |
| Confirm dataset title | Explicit approval and current revision | Approved SourceFieldDefinitions, mappings, EML |

## 7. Run the checks independently

From the **Workbench** repository:

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm test:frontend
pnpm test:rust
```

With the compatible Engine installed and its interpreter selected:

```bash
node scripts/test-project-setup.mjs
```

The Node integration test creates temporary synthetic CSVs, invokes the actual Engine CLI, tests SHA-256 and source immutability, registers and selects a technical pipeline, and verifies that project setup **does not execute a RUN**. It is **not** a test of the full visual Tauri application or manual native dialogs. CI separately runs Linux frontend/Rust checks, Windows checks, and the pinned Engine setup integration where configured.

For genuine scientific input and full recorded-data checks, see [FAIRmorph validations](https://github.com/FAIRmorph/fairmorph-validations/blob/main/docs/CS-PHOC-REPRODUCTION-AND-RESULTS.md). That test exercises the Engine and dataset-specific validation scripts—not the Workbench UI.

## 8. Limitations and troubleshooting

- **Engine connection fails:** Ensure a compatible Engine is installed into the interpreter identified by `FAIRMORPH_ENGINE_PYTHON` or the expected sibling virtual environment. Start from the Workbench directory.
- **Create Project disabled:** Provide nonempty name, parent directory and a valid, new folder name. A project folder must not already exist.
- **Import disabled:** Assign a supported explicit role to every selected pending file.
- **Cannot configure a pipeline:** Select/open a valid project, load imported resources, and pick a valid **technical** template with every declared input bound.
- **RUN disabled:** Register and **select** the technical pipeline in the currently opened project; registration alone does not launch or authorize a RUN.
- **Title approval unavailable:** Load title state, save a draft, stage its actual revision, and provide a responsible user identity. An unconfirmed draft remains unapproved.
- **Project verification fails:** Check the Engine's error result; do not rewrite imported source objects or completed RUN files.
- **Release executable missing:** Development mode requires a Python interpreter; a production build expects a separately bundled Engine executable. See [architecture](ARCHITECTURE.md).

**Current UI localization:** Some user-facing UI text is still in German. This guide uses English names with the actual German labels in parentheses for navigability. This documentation update does not translate the application interface.

**Scope reminder:** The current Workbench does not support full SourceFieldDefinition/mapping interviews, complete scientific publication workflows, or automatic AI interpretations. The [Engine](https://github.com/FAIRmorph/fairmorph-engine) owns such technical and review contracts; future UI support must invoke them without inventing a separate approval system.
