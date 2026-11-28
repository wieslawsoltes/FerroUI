#!/usr/bin/env python3
"""Check mixed declarations against the actual Cargo Base library and smoke-test the corresponding desktop."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import shutil

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path, help="Directory containing the Catalog build pair")
parser.add_argument("arm", choices=["stock", "candidate"])
args = parser.parse_args()
out = args.output.resolve()
binary = out / args.arm
cargo = [json.loads(line) for line in (out / f"{args.arm}-cargo.jsonl").read_text().splitlines() if line.startswith("{")]
features = {"full-themes", "markup-functions", "type-tables"}
libs = {Path(name) for a in cargo if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "ferroui_base"
        and set(a["features"]) == features and a["profile"]["opt_level"] == "3" for name in a["filenames"] if name.endswith(".rlib")}
assert len(libs) == 1
lib = libs.pop()
deps = lib.parent
frozen = out / f"{args.arm}-base" / lib.name
frozen.parent.mkdir(exist_ok=True)
if not frozen.exists():
    shutil.copyfile(lib, frozen)
validation = out / f"{args.arm}-declarations"
subprocess.run(["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "panic=abort", "-C", "codegen-units=1",
                "-C", "lto=fat", "-C", "strip=symbols", "--extern", f"ferroui_base={frozen}", "-L",
                f"dependency={deps}", str(HERE / "declarations.rs"), "-o", str(validation)], check=True)
subprocess.run([validation], check=True)
env = {k:v for k,v in os.environ.items() if not k.startswith("FERROUI_")}
with (out / f"{args.arm}-default.log").open("w") as log:
    subprocess.run([binary], env=dict(env, FERROUI_SMOKE_EXIT_MS="750"), stdout=log, stderr=subprocess.STDOUT,
                   timeout=60, check=True)
assert "Timer fired: closing the main window" in (out / f"{args.arm}-default.log").read_text()
pages = ["DrawerPage", "TabbedPage", "ListBox"]
images = out / f"{args.arm}-pages"
images.mkdir(exist_ok=True)
with (images / "run.log").open("w") as log:
    subprocess.run([binary], env=dict(env, FERROUI_CATALOG_THEME_VARIANT="light", FERROUI_SMOKE_WINDOW_SIZE="950x680",
                   FERROUI_SMOKE_EXTEND_CLIENT_AREA="0", FERROUI_SMOKE_WINDOW_POSITION="20,40",
                   FERROUI_SMOKE_SCREENSHOTS=str(images), FERROUI_SMOKE_PAGES="1800",
                   FERROUI_SMOKE_SCREENSHOT_PAGES=",".join(pages)), stdout=log, stderr=subprocess.STDOUT, timeout=90, check=True)
log = (images / "run.log").read_text()
assert all("Screenshot of "+page+":" in log for page in pages)
print(json.dumps({"default_window_launch":"passed", "production_declaration_checks":"passed", "captured_pages":pages}))
