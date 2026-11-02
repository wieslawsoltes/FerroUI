#!/usr/bin/env python3
"""Compare two already-built WinUI build scripts against identical XAML and type models."""

import argparse
import hashlib
import json
import os
import resource
import subprocess
import tempfile
import time
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def child_cpu():
    times = resource.getrusage(resource.RUSAGE_CHILDREN)
    return times.ru_utime + times.ru_stime


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--theme", type=Path, required=True, help="FerroUI.Themes.WinUI source directory")
    parser.add_argument("--models", type=Path, nargs="+", required=True, help="Base, Controls, Dialogs and Markup.Xaml .xamlmeta files")
    args = parser.parse_args()
    hosts = {"before": args.before.resolve(), "after": args.after.resolve()}
    inputs = {p.stem: p.resolve() for p in args.models}
    if set(inputs) != {"ferroui_base", "ferroui_controls", "ferroui_dialogs", "ferroui_markup_xaml"}:
        parser.error("pass exactly the four WinUI dependency models")
    env = {**os.environ, "CARGO_MANIFEST_DIR": str(args.theme.resolve()), "CARGO_PKG_NAME": "ferroui-themes-winui"}
    env.pop("FERROUI_WINUI_THEME_ROOTS", None)
    env.pop("CARGO_FEATURE_BINARY_RESOURCES", None)
    for name, path in inputs.items():
        env[f"DEP_{name.upper()}_XAML_XAMLMETA"] = str(path)
    rows = []
    for mode in ("before", "after", "after", "before"):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            cpu, start = child_cpu(), time.perf_counter()
            subprocess.run([hosts[mode]], cwd=args.theme, env={**env, "OUT_DIR": directory}, stdout=subprocess.DEVNULL, check=True)
            elapsed, total_cpu = time.perf_counter() - start, child_cpu() - cpu
            rust = out / "xaml" / "compiled_xaml.rs"
            rows.append(dict(which=mode, wall_s=elapsed, cpu_s=total_cpu, rust_bytes=rust.stat().st_size,
                             rust_sha256=digest(rust), metadata_sha256=digest(out / "ferroui_themes_winui.xamlmeta")))
    if len({(row["rust_sha256"], row["metadata_sha256"]) for row in rows}) != 1:
        raise ValueError("the build scripts produced different Rust or exported metadata")
    print(json.dumps({"models": {k: digest(p) for k, p in inputs.items()}, "runs": rows}, indent=2))


if __name__ == "__main__":
    main()
