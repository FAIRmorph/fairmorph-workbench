# -*- mode: python ; coding: utf-8 -*-
import os
from PyInstaller.utils.hooks import collect_data_files, collect_submodules, copy_metadata

# PyInstaller stellt 'SPECPATH' bereit (Verzeichnis dieser .spec-Datei)
entrypoint = os.path.join(SPECPATH, "engine_entrypoint.py")

# Hilfsfunktionen zum sicheren Einsammeln optionaler Pakete
def safe_collect_data(package_name):
    try:
        return collect_data_files(package_name)
    except Exception:
        return []

def safe_collect_submodules(package_name):
    try:
        return collect_submodules(package_name)
    except Exception:
        return []

def safe_copy_metadata(package_name):
    try:
        return copy_metadata(package_name)
    except Exception:
        return []

# 1. Daten und Schemas der Engine einsammeln
datas = safe_collect_data('fairmorph_engine')
datas += safe_copy_metadata('fairmorph_engine')

# 2. Optionale GIS-/Wissenschafts-Assets einsammeln
for pkg in ['pyproj', 'shapely', 'geopandas']:
    datas += safe_collect_data(pkg)

# 3. Submodule auflösen
hiddenimports = safe_collect_submodules('fairmorph_engine')
hiddenimports.append('fairmorph_engine.cli')

a = Analysis(
    [entrypoint],
    pathex=[SPECPATH],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['tkinter', 'matplotlib', 'notebook', 'pytest'],
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure, a.zipped_data)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='fairmorph-engine',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name='engine',
)