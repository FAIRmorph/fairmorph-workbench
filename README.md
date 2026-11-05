<div align="center">

# FAIRmorph Workbench

**Local-First Desktop Workbench for FAIR & Reproducible Ecological Data Curation**

[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](#license)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-FFC107?logo=tauri&logoColor=black)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/Docs-fairmorph.org-brightgreen)](https://fairmorph.github.io/fairmorph-documentation/)

</div>

---

## 📌 Overview

**FAIRmorph Workbench** is the cross-platform desktop user interface and process orchestrator for the **FAIRmorph** ecosystem. Built on **Tauri v2** and **Rust**, it provides a local-first, visual workbench designed for marine biologists, ecologists, and research data managers to curate heterogeneous observation, sampling, and media data into standardized, FAIR-compliant scientific packages without writing code.

The application serves as a non-invasive bridge between raw field assets (CSVs, Excel files, image folders) and global biodiversity publishing endpoints (**OBIS**, **GBIF**, and institutional repositories).

> 🔒 **Local-First Assurance:** All file processing, schema inspection, image metadata extraction, and packaging execute entirely on your local machine. No research data, unpublished coordinates, or confidential observations are uploaded to third-party cloud services.



## ✨ Key Features

* 🗺️ **Visual Semantic Mapping:** Interactively map arbitrary source columns to standard **Darwin Core (DwC)** concepts (`scientificName`, `eventDate`, `decimalLatitude`, etc.).
* 🔄 **Reusable Project Recipes:** Save mapping configurations, structural rules, and value normalizations into versioned project files (`.yaml`) to automate recurring monthly intakes.
* ⚡ **Streaming Processing at Scale:** Process multi-gigabyte CSV datasets and tens of thousands of media files with minimal RAM utilization via local batch streaming.
* 📸 **Image & EXIF Metadata Integration:** Associate photographs with biological occurrences or sampling events using header-only metadata parsing (`DateTimeOriginal`, GPS, image identifiers).
* ⚙️ **Automated Validation:** Execute domain-specific semantic and structural quality-control checks before packaging outputs.
* 📦 **Standard Output Packages:** Export complete **Darwin Core Archives (DwC-A)**, **Ecological Metadata Language (EML)** metadata files, and machine-readable **Provenance Manifests** (input/output cryptographic hashes).

## Supported Standards & Target Infrastructure

- 📦 **Biodiversity Data Packages:** Darwin Core Archive (DwC-A), Darwin Core Data Package (DwC-DP), and Frictionless Data Packages.
- 🌐 **Global Aggregators:** OBIS (Ocean Biodiversity Information System), GBIF (Global Biodiversity Information Facility), and EMODnet.
- 🗺️ **GIS & Spatial Formats:** OGC GeoPackage (`.gpkg`) for analysis in QGIS, ArcGIS, and spatial R/Python pipelines.
- 📚 **General Archival & DOIs:** Zenodo, PANGAEA, and Dryad for citable data publishing.


## 🏗️ Architecture

`fairmorph-workbench` acts as the user-facing shell and runtime supervisor. It manages native window lifecycles, file pickers, and OS integrations while orchestrating the underlying execution engine (`fairfield-engine`).