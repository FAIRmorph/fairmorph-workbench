# FAIRmorph Workbench

Local-first desktop user interface for the **FAIRmorph Engine**.

**Status (framework v0.13):** early **vertical Proof of Concept**, not a released product or a scientifically validated publication workflow. The Engine owns the normative data contracts, decisions, verification, RUN/2 artifacts and exports.

## Current vertical integration

- Tauri v2/Rust starts the local Engine only through allowlisted commands: `handshake`, `init-project`, `import-resource`, `run-pipeline` and dataset-metadata actions.
- Svelte displays progress and the Engine's result (`run_id`, output table names, row counts, columns).
- The Workbench passes **no scientific configuration values** as CLI arguments or direct compiler parameters. The Engine reads the chosen project and its already approved, SHA-256-pinned configuration.
- No approval is implied by editing a field. The current UI edits only the project-local dataset-title draft and a separate explicit approval decision file. Engine validation, revision registration and the current revision pointer remain Engine-owned.

The retired benchmark `run_poc` / `PoCRunner` is not part of the Workbench integration.

## Development

Requirements: Node.js, pnpm, Rust/Cargo with Tauri v2 platform prerequisites, and a compatible local FAIRmorph Engine checkout.

```sh
pnpm install
pnpm check
pnpm test:frontend
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

For development the Rust launcher finds a sibling `fairmorph-engine/.venv` or uses `FAIRMORPH_ENGINE_PYTHON`. The Python interpreter must have the current Engine installed. Release builds expect a packaged executable in the Tauri `engine/` resource folder; release bundling and commit pinning remain open work.

The **New Project** Svelte setup section now creates an Engine-owned project directory and imports original files with an explicit user-selected technical role (`events` or `counts`). A scientific mapping, pipeline registration and approval are **not** inferred from imported data. To execute a RUN, separately register/select the applicable pipeline and complete scientific confirmation when required. See [Project setup MVP](docs/POC-PROJECT-SETUP.md).

## Architecture and limits

The user interface now **edits one project-local dataset-title draft**, not scientific CLI arguments. Engine commands stage it as a new content-addressed immutable revision and approve it only after an explicit, separate confirmation action. Existing content-addressed pipeline files, source bytes and completed RUNs must never be overwritten. The current Engine stores certain approvals/revision histories in SQLite, so directly modifying JSON files is **not** sufficient to register approved changes.

A single UI text input flowing straight into an EML compiler would violate this boundary. EML publication from a RUN-bound, approved metadata revision needs a separate target-specific export contract (P0.7).

Besides the setup MVP, the current milestone provides only the dataset-title revision editor; it does **not** provide field-semantic cards, mapping preview, findings, generalized publication or CS-PHOC scientific regression. Those remain P0.6–P0.8 follow-up work.

See [P0.6 Workbench vertical checkpoint](docs/P0.6-WORKBENCH-VERTICAL.md) and the Engine's [v0.13 binding scope](https://github.com/FAIRmorph/fairmorph-engine/blob/main/docs/FRAMEWORK-v0.13-POC-SCOPE.md).

License: MIT.

The dataset metadata draft and approval file format is specified in the [Engine P0.6 checkpoint](https://github.com/FAIRmorph/fairmorph-engine/blob/feature/p06-dataset-metadata-revisions/docs/P0.6-DATASET-METADATA-CHECKPOINT.md). Both feature branches must be tested together before merging.
