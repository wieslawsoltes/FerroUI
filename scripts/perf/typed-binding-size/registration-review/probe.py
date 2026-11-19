#!/usr/bin/env python3
"""Build and exercise both 32-method fixtures against the same prebuilt Catalog Base crate."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import re
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("library", type=Path, help="Target ferroui_base rlib from the ControlCatalog dist build")
parser.add_argument("output", type=Path)
args = parser.parse_args()
output, library = args.output.resolve(), args.library.resolve()
output.mkdir(parents=True, exist_ok=True)
fingerprint = library.parent.parent / ".fingerprint" / library.stem.replace("libferroui_base", "ferroui-base") / "lib-ferroui_base.json"
features = json.loads(json.loads(fingerprint.read_text())["features"])
assert sorted(features) == ["full-themes", "markup-functions", "type-tables"]
compiler = ["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "panic=abort", "-C", "codegen-units=1",
            "-C", "lto=fat", "-C", "strip=symbols", "--extern", f"ferroui_base={library}",
            "-L", f"dependency={library.parent}"]
subprocess.run(compiler + [str(HERE / "layout.rs"), "-o", str(output / "layout")], check=True)
layout = subprocess.check_output([output / "layout"], text=True)
assert layout == (HERE / "layout.txt").read_text(), "The supplied Base crate has a different layout"
results = {}
for arm in ["stock", "shared"]:
    binary, linkmap = output / arm, output / f"{arm}.map"
    variant = ["--cfg", "shared"] if arm == "shared" else []
    subprocess.run(compiler + variant + ["--crate-name", "handler_probe", str(HERE / "handlers.rs"),
                   "-C", f"link-arg=-Wl,-map,{linkmap}", "-o", str(binary)], check=True)
    subprocess.run([binary], check=True, stdout=subprocess.DEVNULL)
    text_section = re.search(r"^0x\w+\s+(0x\w+)\s+__TEXT\s+__text$", linkmap.read_text(errors="replace"), re.M)
    assert text_section
    results[arm] = {"file_bytes": binary.stat().st_size, "text_bytes": int(text_section[1], 16),
                    "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
samples = []
rng = random.Random(18011)
for i in range(31):
    order = list(results)
    rng.shuffle(order)
    elapsed = {arm: int(subprocess.check_output([output / arm], text=True)) for arm in order}
    samples.append(elapsed)
ratios = [s["shared"] / s["stock"] for s in samples]
bootstrap = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(10000))
for arm in results:
    results[arm]["median_ns_per_call"] = statistics.median(s[arm] for s in samples) / 1_000_000
print(json.dumps({
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(), "base_rlib": library.name,
    "base_features": features, "method_count": 32, "receiver_types": 4,
    "warmup_calls_per_process": 100_000, "timed_calls_per_process": 1_000_000,
    "arms": results, "paired_median_ratio": statistics.median(ratios),
    "paired_median_ratio_bootstrap_95pct": [bootstrap[249], bootstrap[9749]], "pairs_elapsed_ns": samples,
}, indent=2))
