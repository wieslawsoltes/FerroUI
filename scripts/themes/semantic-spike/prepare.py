"""Build PR 496's pinned FXP codec and encode the current checkout's complete theme corpora."""
import csv
import hashlib
import json
from pathlib import Path
import shutil
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
OUT = ROOT / "target/semantic-spike"
CODEC = OUT / "codec"
REV = "8678e15b8ae738931237c0e70ef66f61a6ecbfd2"
SOURCE = "src/Markup/FerroUI.Markup.Xaml.Loader/binary_document"

def prepare():
    dest = CODEC / "src/binary_document"
    dest.mkdir(parents=True, exist_ok=True)
    names = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", REV, SOURCE], cwd=ROOT, text=True)
    for name in names.splitlines():
        (dest / Path(name).name).write_bytes(subprocess.check_output(["git", "show", f"{REV}:{name}"], cwd=ROOT))
    shutil.copy(HERE / "lib.rs", CODEC / "src/lib.rs")
    shutil.copy(HERE / "build_archives.rs", CODEC / "src/main.rs")
    (CODEC / "src/bin").mkdir(exist_ok=True)
    shutil.copy(HERE / "parse_bench.rs", CODEC / "src/bin/parse-bench.rs")
    path = json.dumps(str(ROOT / "external/XamlX/src/XamlX"))
    (CODEC / "Cargo.toml").write_text(f'''[workspace]
[package]
name = "ferro-spike-codec"
version = "0.0.1"
edition = "2021"

[dependencies]
xamlx = {{ path = {path} }}
flate2 = {{ version = "1.1", default-features = false, features = ["rust_backend"] }}
''')

def measure_archive_readers():
    """Exclude disk I/O, include destruction of each parsed or decoded syntax graph."""
    manifest = CODEC / "Cargo.toml"
    report = {}
    for theme in ("WinUI", "Fluent"):
        name = theme.lower()
        source = ROOT / f"src/FerroUI.Themes.{theme}"
        archive = OUT / f"{name}.bin"
        csv_path = OUT / f"{name}-parse.csv"
        with csv_path.open("w") as output:
            subprocess.run(["cargo", "run", "--offline", "--release", "--manifest-path", manifest,
                            "--bin", "parse-bench", "--", source, archive], check=True, stdout=output)
        with csv_path.open() as f:
            samples = [{k: int(v) for k, v in row.items()} for row in csv.DictReader(f)]
        paths = list(source.rglob("*.xaml"))
        report[name] = {
            "documents": len(paths),
            "source_bytes": sum(p.stat().st_size for p in paths),
            "archive_bytes": archive.stat().st_size,
            "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "expanded_archive_bytes": samples[0]["expanded_bytes"],
            "reader_median_ns": {k: statistics.median(row[k] for row in samples)
                                 for k in ("xml_ns", "expand_index_ns", "decode_drop_ns")},
            "fxp_total_median_ns": statistics.median(row["expand_index_ns"] + row["decode_drop_ns"] for row in samples),
            "reader_samples": samples,
        }
    (OUT / "readers.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--readers", action="store_true", help="also measure in-memory XML and FXP reader costs")
    args = parser.parse_args()
    prepare()
    subprocess.run(["cargo", "run", "--offline", "--release", "--manifest-path", str(CODEC / "Cargo.toml"),
                    "--bin", "ferro-spike-codec", "--", str(ROOT / "src"), str(OUT)], check=True, cwd=ROOT)
    if args.readers:
        measure_archive_readers()
