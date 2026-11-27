"""Link, normalize and attribute one ControlCatalog backend on macOS."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def source_patch():
    patch = subprocess.check_output(["git", "diff", "--binary", "HEAD", "--", "Cargo.lock", "src"])
    paths = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "--", "src"], text=True)
    for path in sorted(paths.splitlines()):
        diff = subprocess.run(["git", "diff", "--no-index", "--binary", "/dev/null", path], capture_output=True)
        if diff.returncode != 1:
            raise RuntimeError(f"Cannot archive {path}")
        patch += diff.stdout
    return patch


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--backend", choices=("native", "bytecode"), default="bytecode")
    args = parser.parse_args()
    out = args.directory.resolve()
    out.mkdir(parents=True, exist_ok=True)
    scripts = Path(__file__).resolve().parent
    patch = source_patch()
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    (out / "source.patch.gz").write_bytes(gzip.compress(patch, mtime=0))
    (out / "source.json").write_text(json.dumps({
        "revision": revision, "backend": args.backend, "patch_sha256": hashlib.sha256(patch).hexdigest(),
    }, indent=2) + "\n")
    env = dict(os.environ, FERROUI_XAML_BACKEND=args.backend)
    target = Path(env.get("CARGO_TARGET_DIR", "target"))
    with (out / "catalog-build.log").open("w") as log:
        subprocess.run(["cargo", "rustc", "--locked", "--offline", "--profile", "dist", "-p",
                        "control-catalog-desktop", "--", "-C", f"link-arg=-Wl,-map,{out}/catalog.map"],
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    if source_patch() != patch:
        raise RuntimeError("Source changed during compilation")
    exe = out / args.backend
    shutil.copy2(target / "dist/control-catalog-desktop", exe)
    subprocess.run(["strip", exe], check=True)
    subprocess.run(["codesign", "-s", "-", "-f", exe], check=True)
    for script, arguments, result in [
        ("catalog_allocation.py", [exe, out / "catalog.map"], "allocation.json"),
        ("application_image.py", [target / "dist/control-catalog-desktop.d"], "catalog-generated.json"),
    ]:
        with (out / result).open("w") as data:
            subprocess.run([sys.executable, scripts / script, *arguments], stdout=data, check=True)
    print(f"{exe.stat().st_size:,} bytes in {exe}")


if __name__ == "__main__":
    main()
