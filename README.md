# FAIRmorph Workbench

> **Prototype Fund application proof of concept — not funded software.** FAIRmorph was developed solely to prepare an application to Germany’s [Prototype Fund](https://prototypefund.de/); no funding award, selection or official endorsement is claimed. The current Workbench is a technical prototype, not a finished no-code research-data product. [Evidence and six-month development plan](https://www.fairmorph.org/guide/prototype-fund/).

**A local-first desktop user interface for FAIRmorph Engine.** Workbench is built with **Svelte 5 and Tauri 2/Rust**. It lets users initialize or open a FAIRmorph project, import unchanged data files, configure and execute a **technical** Engine pipeline, inspect RUN results and verify project integrity, and create/review a dataset-title revision.

**Maturity:** Technical **proof of concept**; package version `0.1.0`. This is **not** a complete scientific curation application or publication client. The [FAIRmorph Engine](https://github.com/FAIRmorph/fairmorph-engine) owns data contracts, source object storage, reviewed decisions, pipeline execution, RUN/2 snapshots and verification. The Workbench is a user interface; it does **not** reinterpret scientific values or infer mappings from file names.

## What currently works

| User task | Workbench behavior | Limit |
| --- | --- | --- |
| Connect to Engine | Local process handshake and connection status | A compatible, separately installed Engine is needed for development |
| New or existing project | Create an Engine project in a new local folder, or open one with `project.json` | Existing directories are not overwritten |
| Import original data | Select files and assign an **explicit** technical role (`events` or `counts`); display imported source identity/hash | The role names are presently limited; **not** scientific Entity classification |
| Technical pipeline | Select an existing `fairmorph.pipeline/1` JSON template, bind each declared input to an imported resource, save the project-local draft, register and select it via Engine | No creation of scientific field mappings or arbitrary pipeline editor |
| Run and verify | Execute a separately selected technical pipeline, inspect returned RUN ID and table summaries, verify project integrity | The UI does not automatically execute a RUN after import or selection |
| Dataset title | Load the current title, edit a draft, register an immutable revision, and separately confirm approval with a responsible user identity | Title approval is **not** approval of field semantics or a scientific publication |

The Workbench does **not** yet provide SourceFieldDefinition interviews, semantic mapping previews, structured scientific findings, complete Entity-profile workflows, generalized EML/DwC-A/PANGAEA/GeoJSON publication, or scientific approval of CS-PHOC. **No AI or LLM integration is required.**

## Try the Workbench locally

Requirements: **Node.js 22+**, **pnpm**, **Rust/Cargo**, Tauri 2 platform dependencies, and **Python 3.11+** with the compatible Engine installed. Begin with [installation and environment setup](INSTALL.md), then follow the [user guide](docs/USER-GUIDE.md#1-install-and-start).

```bash
# In a separate checkout of FAIRmorph Engine:
python -m venv .venv
# Activate the environment, then:
python -m pip install -e .

# In the Workbench repository:
pnpm install --frozen-lockfile
pnpm check
pnpm test:frontend
pnpm test:rust
pnpm tauri dev
```

For development, the launcher looks for a sibling `fairmorph-engine/.venv` or an explicitly set `FAIRMORPH_ENGINE_PYTHON` interpreter that can import `fairmorph_engine.cli`. A production/release build instead expects a packaged Engine executable in its Tauri `engine/` resources; the published installer and exact Engine version-pinning are **not** established by these commands.

## A reproducible example

Create two simple CSVs **outside** the project folder.

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

In the Workbench choose **New Project**, select a parent directory and a new project folder; import these two files with explicit roles `events` and `counts`. Choose a **technical** pipeline JSON template, bind its `events` input to the imported events file, and register/select it. A template using `table.select/1` and `outputs: ["selected_events"]` is provided in the [user guide](docs/USER-GUIDE.md#3-configure-a-technical-pipeline). Then explicitly start the RUN and verify the project.

**Expected outcome:** Two imported resources with Engine-computed SHA-256 digests; a selected immutable pipeline hash; **one** technical RUN with an output table `selected_events` containing **two rows**; and successful integrity verification. The source CSV files remain untouched. This is **technical processing**, not an ecological interpretation or publication.

## Documentation

- **[Installation](INSTALL.md)** — prerequisites, local Engine setup and development launch.
- **[Workbench user guide](docs/USER-GUIDE.md)** — install, screen-by-screen example, configuration, expected results and troubleshooting.
- **[Architecture and boundaries](docs/ARCHITECTURE.md)** — UI versus Engine responsibilities, project files, explicit decisions, test coverage and limits.
- **[Engine usage and contracts](https://github.com/FAIRmorph/fairmorph-engine/blob/main/README.md)** — CLI, review/approval, RUN/2 and data contract details.
- **[Independent real-data validation](https://github.com/FAIRmorph/fairmorph-validations/blob/main/docs/CS-PHOC-REPRODUCTION-AND-RESULTS.md)** — evidence from original CS-PHOC data. **This is Engine and dataset-specific validation code, not an automated Workbench UI test.**

## Quality checks

```bash
pnpm check
pnpm test:frontend
pnpm test:rust
# With a compatible Engine installed and FAIRMORPH_ENGINE_PYTHON set:
node scripts/test-project-setup.mjs
```

The project-setup integration test uses **synthetic CSVs** and verifies that Engine import, pipeline registration/selection and project integrity work without changing the source or silently running a pipeline. Rust and frontend tests exercise the bridge and UI rules, but **native dialogs and the full Tauri window still need manual end-to-end review on the target OS**.

**Current UI language:** Some labels and feedback are currently German. These English documentation pages describe the existing controls; they do not claim UI localization has been implemented.

License: [MIT](LICENSE).
