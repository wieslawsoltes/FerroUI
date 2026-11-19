"""Run the complete WinUI probes from the checkout root. Use the catalog argument for the desktop A/B."""
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "target/full-opcode-spike"
ENV = dict(os.environ)
TARGET = Path(ENV.get("SPIKE_TARGET", OUT / "cargo")).resolve()
ENV.update(SPIKE_TARGET=str(TARGET), CARGO_TARGET_DIR=str(TARGET))

def script(path, *args):
    subprocess.run([sys.executable, "-B", path, *args], cwd=ROOT, env=ENV, check=True)

def catalog():
    script(HERE / "catalog_project.py")
    case = OUT / "catalog"
    for mode, manifest in [("native", ROOT / "Cargo.toml"), ("ast", case / "Cargo.toml")]:
        subprocess.run(["cargo", "build", "--offline", "--profile", "dist", "--manifest-path", manifest,
                        "-p", "control-catalog-desktop", "--bin", "control-catalog-desktop"],
                       cwd=ROOT, env=ENV, check=True)
        exe = case / mode
        shutil.copy2(TARGET / "dist/control-catalog-desktop", exe)
        subprocess.run(["strip", exe], check=True)
        subprocess.run(["codesign", "-f", "-s", "-", exe], check=True)
        subprocess.run(["codesign", "--verify", exe], check=True)
        print(mode, exe.stat().st_size, flush=True)
    script(HERE / "catalog_smoke.py")
    script(HERE / "catalog_startup.py")

if __name__ == "__main__":
    script(HERE.parent / "semantic-spike/prepare.py")
    script(HERE / "prepare.py")
    script(HERE / "ast_project.py")
    if sys.argv[1:] == ["catalog"]:
        catalog()
    else:
        script(HERE / "measure.py", "native", "ast", "runtime", "timings")
