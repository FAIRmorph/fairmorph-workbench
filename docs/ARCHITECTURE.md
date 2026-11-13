# FAIRmorph Workbench — Architecture, Contracts and Verification

**Scope:** Tauri 2/Rust + Svelte 5 **desktop proof of concept**. This document describes implemented behavior, not an abandoned implementation plan. The Workbench is not the generic Engine, an independent scientific reviewer, or an EML/DwC-A publication service.

## 1. Responsibilities

```text
User
  |
  v
Svelte Workbench UI
  |  (explicit source selection, project setup, template input bindings,
  |   dataset title draft and explicit confirmation, RUN and verify actions)
  v
Tauri/Rust allowlisted commands
  |  project and path checks; native file dialogs;
  |  local draft JSON writes; Engine subprocess + NDJSON event parsing
  v
FAIRmorph Engine (separate local Python package)
  |  immutable original object import and hashes; SQLite catalog;
  |  registered immutable pipeline/metadata revisions;
  |  reviewer/approval checks; RUN/2 Parquet + manifests; verification
  v
Local FAIRmorph project directory
```

**Authority is separated:** The Workbench may create or update **editable** draft files and request Engine operations, but must not edit immutable registered source objects, approved revision payloads, pipeline records, completed RUNs or Engine SQLite decisions directly. Scientific values and mapping configuration are read from project files by the Engine—not injected into the Engine compiler via UI CLI arguments.

The current interface uses a **bounded set of Engine commands**: `handshake`, `init-project`, `import-resource`, `register-pipeline`, `select-pipeline`, `run-pipeline`, `verify-project`, `show-dataset-metadata`, `stage-dataset-metadata`, `approve-dataset-metadata`. The Rust bridge validates the command, project path and returned NDJSON; it is not an arbitrary shell-command executor.

## 2. Implemented project lifecycle

| User action | Workbench writes or asks | Engine responsibility |
| --- | --- | --- |
| Create project | Explicit name, new path | Creates `project.json`, SQLite catalog and project structure |
| Import file | Explicit selected path and technical role | Registers source resource identity, stores original bytes and SHA-256 |
| Open existing project | Project folder with `project.json` | Current project resources are read from Engine-created import manifests |
| Select technical template | Existing JSON with `resources`, `steps`, `outputs` | Template is not executed; input slots are explicitly bound |
| Save pipeline draft | `configurations/drafts/pipeline-technical.json` | No immutable revision or RUN yet |
| Register/select pipeline | Calls Engine with draft path / returned SHA | Validates and records immutable pipeline; selects active hash |
| Run/verify | Calls Engine with project path | Executes selected pipeline, persists verified artifacts; verifies project |
| Save dataset title | `configurations/drafts/dataset-metadata.json` | Stages a **separate** immutable title revision when requested |
| Confirm dataset title | `configurations/drafts/dataset-metadata-approval.json` | Verifies the exact SHA/parent/reviewer and records approval |

**No automatic RUN:** Importing, editing, staging, registering or selecting a pipeline does not start execution. The RUN action is explicit and requires an applicable selected SHA. A selected **technical pipeline** requires no invented scientific entity approval. Scientific pipelines, if added later, must observe the Engine's existing field, profile and mapping review contracts.

**No automatic approval:** Editing a title or saving a draft is not equivalent to approving it. A separate project-local decision and Engine validation are required. The Engine's approved current title revision may be included in later generic RUN snapshots. Earlier RUNs remain unchanged.

## 3. UI, subprocess and packaging

- Frontend: Svelte 5/SvelteKit, state and protocol validation using TypeScript and Zod.
- Desktop: Tauri 2/Rust, native file/folder dialog plugin, explicitly registered `tauri::command` handlers and controlled Engine process execution.
- Development transport: Python `-m fairmorph_engine.cli` launched from `FAIRMORPH_ENGINE_PYTHON` (or a sibling `fairmorph-engine/.venv`). The Engine must actually be importable by that interpreter.
- Release transport: expected prepackaged executable at Tauri resource `engine/fairmorph-engine` (Windows: `engine/fairmorph-engine.exe`). Its presence is a packaging prerequisite; an end-user installer/release with compatible pinned Engine is **not** demonstrated merely by development CI.
- Engine protocol: JSON-line progress and one result event; the UI validates the result and displays RUN ID, table names, row counts and columns when present.

These restrictions are important: Workbench provides user interaction, while the **Engine owns scientific decisions, immutable revisions and RUN evidence**. A single input field directly passed to an EML compiler would bypass those boundaries and is not supported.

## 4. Concrete example and expected artifacts

The [user guide](USER-GUIDE.md) imports a two-row `events.csv` and a two-row `counts.csv`. The optional technical `table.select/1` template binds to the imported events resource.

Expected:

- New Engine project with **two original, unchanged source resources**, each identified by an Engine SHA-256.
- New content-addressed pipeline SHA-256 and explicit project selection.
- **No RUN** before clicking the RUN action.
- One executed RUN with `selected_events` containing exactly **two** rows and `id`, `date` columns.
- A `verified` project response after explicit integrity verification.
- If the optional title editor is used: draft JSON, registered immutable revision, and separate approval decision; **not** an automatically generated `eml.xml`.

The Workbench does not implement scientific mapping of the `counts` resource merely because it was imported. A technical role named `counts` is **not** an `Occurrence`, `MeasurementOrFact` or population estimate.

## 5. Test and assurance boundaries

| Command / workflow | Checks | Does not prove |
| --- | --- | --- |
| `pnpm check` | Svelte and TypeScript correctness | Real scientific user flow |
| `pnpm test:frontend` | Protocol validation and frontend project-setup rules | Native dialogs work on all systems |
| `pnpm test:rust` | Rust bridge safety, subprocess integration where configured | Desktop distribution is production-ready |
| `node scripts/test-project-setup.mjs` | Real Engine CLI against synthetic originals: import hashes, deduplication, pipeline selection, no implicit RUN | Completed graphical UI run or real research validity |
| Linux / Windows GitHub Actions | Platform-specific synthetic build/test checks | Native graphical workflows and publication approval |

The integration test intentionally verifies `completed_runs: 0` **during setup**. That is an expected and desirable result, not a failed RUN. A user can subsequently execute an explicitly selected pipeline through the Workbench interface; that separate step requires a live desktop review.

For scientific input, [fairmorph-validations](https://github.com/FAIRmorph/fairmorph-validations/blob/main/docs/CS-PHOC-REPRODUCTION-AND-RESULTS.md) tests a pinned Engine on original CS-PHOC census data and audits dataset-specific exports, **without exercising this Workbench UI**.

## 6. Current limits and future UI scope

The Workbench currently offers only a bounded technical setup, RUN execution, project verification and dataset-title revision editor. There is **no** general-purpose SourceFieldDefinition interview, source→target semantic mapping preview, structured research findings, curated relations UI, generic publication export interface, or AI runtime.

Future interviews must use the existing Engine Registry and approved mapping/metadata contracts. The UI must not introduce separate scientific approval systems, silently infer species/counting meanings, or claim publisher review.

The current Svelte UI contains **German labels and messages**. English documentation records the actual behavior and gives matching control names; translating the UI itself would be a separate application change.

## See also

- [Step-by-step Workbench user guide](USER-GUIDE.md)
- [FAIRmorph Engine usage and contracts](https://github.com/FAIRmorph/fairmorph-engine/blob/main/docs/README.md)
- [Independent CS-PHOC real-data evaluation](https://github.com/FAIRmorph/fairmorph-validations/blob/main/docs/CS-PHOC-REPRODUCTION-AND-RESULTS.md)
