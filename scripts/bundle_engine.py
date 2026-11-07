"""Build the existing PyInstaller spec into Tauri's resource directory."""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "packaging" / "engine.spec"
DIST = ROOT / "src-tauri"
ENGINE = DIST / "engine"

def main() -> None:
    subprocess.run(
        [sys.executable, "-m", "PyInstaller", "--noconfirm", "--clean",
         "--distpath", str(DIST), "--workpath", str(ROOT / "build" / "pyinstaller"),
         str(SPEC)],
        cwd=ROOT,
        check=True,
    )
    binary = ENGINE / ("fairmorph-engine.exe" if sys.platform == "win32" else "fairmorph-engine")
    if not binary.is_file():
        raise SystemExit(f"Bundled engine executable missing: {binary}")
    print(f"Engine bundled at {binary}")

if __name__ == "__main__":
    main()
