# Installation and development

> **Prototype Fund (Germany) · Proof of concept — not production software.** FAIRmorph is an experimental open-source project for Germany's [Prototype Fund](https://prototypefund.de/). These instructions start the **developer prototype**, not a validated end-user installer. Complete no-code curation and automatic publication into multiple repositories are not implemented.

## Prerequisites

- **Node.js 22+**, [pnpm](https://pnpm.io/), **Rust/Cargo** and the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/).
- **Python 3.11+**, Git and a local checkout of the compatible [Engine](https://github.com/FAIRmorph/fairmorph-engine).
- A desktop session with access to native file dialogs and a writable project directory.

## Install the Engine first

```bash
git clone https://github.com/FAIRmorph/fairmorph-engine.git
cd fairmorph-engine
python -m venv .venv
# Linux/macOS: source .venv/bin/activate
# Windows PowerShell: .venv\Scripts\Activate.ps1
python -m pip install -e .
python -m fairmorph_engine.cli handshake
```

For platform details and troubleshooting see [Engine installation](https://github.com/FAIRmorph/fairmorph-engine/blob/main/INSTALL.md).

## Run the Workbench developer application

From the parent folder, create a sibling checkout:

```bash
cd ..
git clone https://github.com/FAIRmorph/fairmorph-workbench.git
cd fairmorph-workbench
pnpm install --frozen-lockfile
pnpm check
pnpm tauri dev
```

The developer launcher looks for a compatible Engine interpreter in the sibling `fairmorph-engine/.venv`. If stored elsewhere, set `FAIRMORPH_ENGINE_PYTHON` to the absolute path of the Engine virtual environment's Python executable:

```bash
export FAIRMORPH_ENGINE_PYTHON=/absolute/path/fairmorph-engine/.venv/bin/python
pnpm tauri dev
```

In Windows PowerShell: `$env:FAIRMORPH_ENGINE_PYTHON = 'C:\path\fairmorph-engine\.venv\Scripts\python.exe'`.

Run checks with `pnpm test:frontend`, `pnpm test:rust` and—when the compatible Engine is configured—`node scripts/test-project-setup.mjs`. The tests do not replace a complete graphical or scientific end-to-end acceptance.

Continue with the [step-by-step Workbench user guide](docs/USER-GUIDE.md) for a two-file CSV example and expected outputs. The Workbench does not provide a production-ready packaged installer, generalized DwC-A/PANGAEA export or fully implemented scientific mapping approvals.
