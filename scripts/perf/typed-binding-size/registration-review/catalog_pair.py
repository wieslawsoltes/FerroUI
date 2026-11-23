#!/usr/bin/env python3
"""Reproduce a normal stock and patched ControlCatalog link in one disposable checkout."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
BASE = "933fd46325772fd1cdec81add9d4912dda2fe931"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path, help="New private checkout/build directory")
args = parser.parse_args()
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=False)
tree = out / "workspace"
subprocess.run(["git", "worktree", "add", "--detach", str(tree), BASE], cwd=ROOT, check=True)
env = dict(os.environ, CARGO_TARGET_DIR=str(out / "cache"))
env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True).strip())
for arm in ["stock", "candidate"]:
    if arm == "candidate":
        subprocess.run(["git", "apply", str(HERE / "routed-handler.patch")], cwd=tree, check=True)
    with (out / f"{arm}-cargo.jsonl").open("w") as stdout, (out / f"{arm}-build.log").open("w") as stderr:
        subprocess.run(["cargo", "rustc", "--locked", "--profile", "dist", "--message-format=json",
                        "-p", "control-catalog-desktop", "--bin", "control-catalog-desktop", "--",
                        "-C", "link-arg=-Wl,-map,"+str(out / f"{arm}.map")],
                       cwd=tree, env=env, stdout=stdout, stderr=stderr, check=True)
    shutil.copyfile(out / "cache/dist/control-catalog-desktop", out / arm)
    shutil.copymode(out / "cache/dist/control-catalog-desktop", out / arm)
    with (out / f"{arm}-verified.json").open("w") as checks:
        subprocess.run([sys.executable, str(HERE / "verify_arm.py"), str(out), arm], stdout=checks, check=True)
subprocess.run([sys.executable, str(HERE / "record_catalog.py"), str(out)], check=True)
