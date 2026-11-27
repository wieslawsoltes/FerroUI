"""Measure a second complete theme in the same process, with the loader's transformed group cached."""
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "target/full-opcode-spike"
out = root / "target/semantic-spike/cache"
target = Path(os.environ.get("CACHED_SPIKE_TARGET", out / "cargo"))
driver = (root / "scripts/themes/compact-spike/template_bench.rs").read_text()
driver = driver.replace("fn main() {", "fn benchmark() {", 1)
driver += "\nfn main() { benchmark(); benchmark(); }\n"
for mode in ("native", "ast"):
    p = out / mode
    p.mkdir(parents=True, exist_ok=True)
    manifest = (source / mode / "Cargo.toml").read_text().replace("full-opcode-probe", "theme-cache-probe")
    (p / "Cargo.toml").write_text(manifest)
    (p / "main.rs").write_text(driver)
    shutil.copy(source / mode / "Cargo.lock", p / "Cargo.lock")
    subprocess.run(["cargo", "build", "--offline", "--profile", "dist", "--manifest-path", p / "Cargo.toml"],
                   check=True, env=dict(os.environ, CARGO_TARGET_DIR=str(target)))
    shutil.copy(target / "dist/theme-cache-probe", p / "probe")
    with (p / "stages.log").open("w") as dest:
        subprocess.run([p / "probe"], check=True, env=dict(os.environ, FERROUI_XAML_TIMINGS="1"), stderr=dest, stdout=dest)
samples = {mode: [] for mode in ("ast", "native")}
for i in range(9):
    for mode in (("ast", "native") if i % 2 else ("native", "ast")):
        raw = subprocess.check_output([out / mode / "probe"], text=True)
        rows = [[int(n) for n in line.split()] for line in raw.splitlines()]
        assert len(rows) == 2 and all(len(row) == 4 for row in rows)
        samples[mode].append(rows)
report = {"samples_ns": samples, "fields": ["construct theme", "lookup Button template", "first template", "mean of 128 builds"]}
report["median_ns"] = {mode: [list(map(statistics.median, zip(*(pair[idx] for pair in pairs)))) for idx in range(2)]
                       for mode, pairs in samples.items()}
(out / "results.json").write_text(json.dumps(report, indent=2) + "\n")
print(report["median_ns"])
