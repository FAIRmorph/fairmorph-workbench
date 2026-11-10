# FAIRmorph Workbench

Local-first desktop user interface for the **FAIRmorph Engine**.

**Status (framework v0.13):** early **vertical Proof of Concept**, not a released product or a scientifically validated publication workflow. The Engine owns the normative data contracts, decisions, verification, RUN/2 artifacts and exports.

## Current vertical integration

- Tauri v2/Rust starts the local Engine only through an allowlisted `handshake` or `run-pipeline --project <path>` action.
- Svelte displays progress and the Engine's result (`run_id`, output table names, row counts, columns).
- The Workbench passes **no scientific configuration values** as CLI arguments or direct compiler parameters. The Engine reads the chosen project and its already approved, SHA-256-pinned configuration.
- No approval is implied by editing a field. The current UI does not yet edit project configurations.

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

To use the Workbench, initialize a project using the Engine, import resources, register and explicitly confirm the applicable mapping/Entity pipeline and profile bindings, then select the pipeline in the project. The Workbench runs that selected project **without changing its config**.

## Architecture and limits

The user interface will eventually **edit project-local configuration drafts**, not submit scientific values directly to the Engine. Drafts must be registered as *new* immutable revisions, validated and explicitly approved before selection; existing content-addressed pipeline files, source bytes and completed RUNs must never be overwritten. The current Engine stores certain approvals/revision histories in SQLite, so directly modifying JSON files is **not** sufficient to register approved changes.

A single UI text input flowing straight into an EML compiler would violate this boundary. EML publication from a RUN-bound, approved metadata revision needs a separate target-specific export contract (P0.7).

The current milestone does **not** provide field-semantic cards, mapping preview, findings, a revision editor, generalized publication or CS-PHOC scientific regression. Those remain P0.6–P0.8 follow-up work.

See [P0.6 Workbench vertical checkpoint](docs/P0.6-WORKBENCH-VERTICAL.md) and the Engine's [v0.13 binding scope](https://github.com/FAIRmorph/fairmorph-engine/blob/main/docs/FRAMEWORK-v0.13-POC-SCOPE.md).

License: MIT.
