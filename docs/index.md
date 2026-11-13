---
title: "FAIRmorph Workbench documentation"
description: "Installation, desktop usage, Engine integration, expected outputs and limitations"
---

# FAIRmorph Workbench documentation

FAIRmorph Workbench is a **local Svelte/Tauri desktop frontend** for the separate FAIRmorph Engine. The current implementation supports project creation/opening, explicit source import, technical pipeline template selection and binding, RUN execution and verification, and a separately confirmed dataset-title revision.

**Start with the [Workbench user guide](USER-GUIDE.md)** for step-by-step local installation and a reproducible two-record example, or read the [architecture guide](ARCHITECTURE.md) for the precise division between UI state, project draft JSON, Engine registries and immutable RUN evidence.

The Workbench is a v0.13 proof of concept, not a scientific publication system. The [independent real-data validation](https://github.com/FAIRmorph/fairmorph-validations/blob/main/docs/CS-PHOC-REPRODUCTION-AND-RESULTS.md) tests the Engine and dataset-specific scripts **without Workbench UI automation**.
