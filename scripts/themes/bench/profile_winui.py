#!/usr/bin/env python3
"""Train and apply whole-application WinUI PGO using the locked consumer made by measure.py."""
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path

CRATES = ("ferroui_base", "ferroui_controls", "ferroui_markup_xaml",
          "ferroui_markup_xaml_loader", "ferroui_themes_winui", "winui_theme_probe")
REQUIRED = ("ferroui_base", "ferroui_controls", "ferroui_markup_xaml_loader", "ferroui_themes_winui")
CRATE_ROOT = re.compile(r"C(?:s[0-9A-Za-z]*_)?(u?)([1-9][0-9]*)_?")


def executed_definitions(lines):
    names = {crate: set() for crate in CRATES}
    for line in lines:
        # LLVM's covered view emits one mangled name per line. An IR-local
        # definition can be qualified with its codegen unit and a semicolon.
        symbol = line.strip().rsplit(";", 1)[-1].removesuffix(":")
        if not symbol.startswith("_R"):
            continue
        root = CRATE_ROOT.search(symbol, 2)
        if root and not root[1]:
            start = root.end()
            crate = symbol[start:start + int(root[2])]
            if crate in names:
                names[crate].add(symbol)
    return {crate: len(symbols) for crate, symbols in names.items()}

HERE = Path(__file__).resolve().parent
PROBE = "winui-theme-probe"
FEATURES = "ferroui-themes-winui/solid-theme,retain-archive"
LINK = ["-Clink-arg=-fuse-ld=lld", "-Clink-arg=-Wl,--icf=safe",
        "-Clink-arg=-Wl,-z,pack-relative-relocs", "-Cstrip=symbols"]


def make_archives(source, run, packer):
    root = source / "src/FerroUI.Themes.WinUI"
    docs = run / "theme-sources"
    for src in root.rglob("*.xaml"):
        if src.is_symlink() or not src.is_file():
            continue
        dst = docs / src.relative_to(root)
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
    outputs = []
    for file, size in [("DensityStyles/Compact.xaml", 17), ("Resources/Generic.xaml", 19)]:
        path = docs / file
        original = path.read_bytes()
        token = b'<x:Double x:Key="ControlContentThemeFontSize">14</x:Double>'
        if token not in original:
            raise ValueError(f"Training edit no longer matches {path}")
        path.write_bytes(original.replace(token, token.replace(b">14<", f">{size}<".encode())))
        archive = run / (path.stem + ".fxp")
        subprocess.run([packer, "--pack-paged", docs, archive], check=True)
        outputs.append(archive)
    return outputs


def prepare_training_sources(source, pkg, original):
    property_report = source / "docs/benchmarks/compiled-theme-binding-dispatch-linux-x86_64.json"
    prop = json.loads(property_report.read_text())["property"]["property_workload_source"]
    if prop.count("fn main(){") != 1:
        raise ValueError("unexpected property benchmark entry point")
    (pkg / "property_work.rs").write_text(prop.replace("fn main(){", "pub fn run(){", 1))
    shutil.copyfile(HERE / "pgo_templates.rs", pkg / "template_work.rs")
    if original.count("fn main() {") != 1:
        raise ValueError("unexpected benchmark entry point")
    entry = """mod property_work;
mod template_work;
fn main() {
    match std::env::var("FERROUI_PGO_TRAIN").as_deref() {
        Ok("property") => return property_work::run(),
        Ok("templates") => return template_work::run(),
        _ => {}
    }"""
    (pkg / "main.rs").write_text(original.replace("fn main() {", entry))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", type=Path, required=True)
    parser.add_argument("--source", type=Path, default=Path.cwd(), help="repository root (default: current directory)")
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--llvm-profdata", type=Path)
    parser.add_argument("--use-profile", type=Path, help="apply an existing merged LLVM profile for a fixed-input comparison")
    parser.add_argument("--use-opt-level", choices=("s", "z"), action="append",
                        help="target optimization for the profile-use build (default: s); repeat to compare both")
    parser.add_argument("--compact-unwind", action="store_true",
                        help="try smaller x86-64 unwind records at the final profile-use link")
    parser.add_argument("--packer", type=Path, help="reuse a ferroui-fxd built from this source")
    args = parser.parse_args()
    if args.use_profile and not args.use_profile.is_file():
        parser.error(f"missing merged LLVM profile: {args.use_profile}")
    source = args.source.resolve()
    rustc = dict(line.split(": ", 1) for line in subprocess.check_output(
        ["rustc", "-vV"], text=True).splitlines() if ": " in line)
    host = rustc["host"]
    if host != "x86_64-unknown-linux-gnu":
        parser.error(f"the ELF policy and thread CPU fixture require native x86-64 GNU/Linux; found {host}")
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    profdata = (args.llvm_profdata or sysroot / "lib/rustlib" / host / "bin/llvm-profdata").resolve()
    if not profdata.is_file():
        parser.error("install rustup component llvm-tools-preview, or supply the matching --llvm-profdata")
    tool_env = os.environ.copy()
    tool_env["LD_LIBRARY_PATH"] = os.pathsep.join(filter(None, [str(sysroot / "lib"), tool_env.get("LD_LIBRARY_PATH")]))
    tool_version = subprocess.check_output([profdata, "--version"], env=tool_env, text=True)
    major = re.escape(rustc["LLVM version"].split(".")[0])
    if not re.search(r"\bversion " + major + r"\.", tool_version, flags=re.I):
        parser.error("llvm-profdata and rustc must use the same LLVM major version")

    args.work_dir.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="winui-pgo-", dir=args.work_dir.resolve()))
    print(f"PGO work directory {run}", flush=True)
    pkg = run / "consumer"
    pkg.mkdir()
    for name in ["Cargo.toml", "Cargo.lock", "main.rs"]:
        shutil.copyfile(args.consumer / name, pkg / name)
    manifest = tomllib.loads((pkg / "Cargo.toml").read_text())
    theme_path = Path(manifest["dependencies"]["ferroui-themes-winui"]["path"])
    if theme_path.resolve() != source / "src/FerroUI.Themes.WinUI":
        parser.error("the reference consumer and source must refer to the same checkout")
    original = (pkg / "main.rs").read_text()
    if original != (source / "scripts/themes/bench/winui.rs").read_text():
        parser.error("--consumer must be the reference consumer for this --source")
    raw = run / "profiles"
    raw.mkdir()
    target = run / "target"
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_INCREMENTAL": "0",
           "CARGO_PROFILE_RELEASE_LTO": "thin", "CARGO_PROFILE_RELEASE_OPT_LEVEL": "s",
           "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "16", "CARGO_PROFILE_RELEASE_PANIC": "unwind"}
    for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE", "FERROUI_PGO_TRAIN",
                 "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE", "FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE"):
        env.pop(name, None)
    if args.packer:
        packer = args.packer.resolve()
    else:
        subprocess.run(["cargo", "build", "--locked", "--manifest-path", source / "Cargo.toml",
                        "-p", "ferroui-markup-xaml-loader", "--no-default-features", "--features",
                        "portable-xaml", "--bin", "ferroui-fxd", "--target", host], env=env, check=True)
        packer = target / host / "debug/ferroui-fxd"
    scalar, structure = make_archives(source, run, packer)

    def build(profile_flag, link, target_opt="s"):
        # Explicit --target keeps the profiler out of host build scripts and proc macros.
        target_flags = [profile_flag, "-Cllvm-args=--addrsig"]
        if target_opt == "z":
            target_flags.append("-Copt-level=z")
        encoded = "\x1f".join(target_flags)
        subprocess.run(["cargo", "rustc", "--locked", "--target", host, "--release", "--features",
                        FEATURES, "--bin", PROBE, "--", *LINK, *link], cwd=pkg, check=True,
                       env={**env, "CARGO_ENCODED_RUSTFLAGS": encoded})
        return target / host / "release" / PROBE

    def exercise(binary, lane, *arguments, expected=None):
        settings = {**env, "FERROUI_PGO_TRAIN": lane, "LLVM_PROFILE_FILE": str(raw / f"{lane}-%m-%p.profraw")}
        if expected:
            normal, compact = expected
            settings["FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE"] = str(normal)
            settings["FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE"] = str(compact)
        subprocess.run([binary, *arguments], check=True, env=settings)

    merged = run / "combined.profdata"
    if args.use_profile:
        shutil.copyfile(args.use_profile, merged)
    else:
        prepare_training_sources(source, pkg, original)
        binary = build(f"-Cprofile-generate={raw}", [])
        for round_index in range(3):
            exercise(binary, "startup")
            exercise(binary, "templates")
            exercise(binary, "property", str(round_index))
            exercise(binary, "edit", scalar, expected=(14, 17))
        exercise(binary, "structure", structure, expected=(19, 17))
        # Keep zero-count records: absence and proven cold execution are different optimizer inputs.
        for lane in ("startup", "templates", "property", "edit", "structure"):
            if not any(f.stat().st_size for f in raw.glob(f"{lane}-*.profraw")):
                raise RuntimeError(f"No raw profile from {lane}")
        subprocess.run([profdata, "merge", *sorted(raw.glob("*.profraw")), "-o", merged], env=tool_env, check=True)
    with merged.open("rb") as stream:
        profile = run / (hashlib.file_digest(stream, "sha256").hexdigest() + ".profdata")
    merged.rename(profile)
    with subprocess.Popen([profdata, "show", "--all-functions", "--covered", profile], text=True,
                          stdout=subprocess.PIPE, env=tool_env) as shown:
        counts = executed_definitions(shown.stdout)
        if shown.wait() != 0 or not all(counts[k] for k in REQUIRED):
            raise RuntimeError(f"Missing or unreadable target profile coverage: {counts}")
    (run / "coverage.json").write_text(json.dumps(counts, indent=2) + "\n")
    (pkg / "main.rs").write_text(original)
    results = {}
    for level in dict.fromkeys(args.use_opt_level or ["s"]):
        link = ["-Clto=fat", "-Ccodegen-units=1"]
        if args.compact_unwind:
            link.append(f"-Clinker={HERE / 'compact_unwind_link.py'}")
        binary = build(f"-Cprofile-use={profile}", link, target_opt=level)
        exercise(binary, "shipping")
        exercise(binary, "shipping-edit", scalar, expected=(14, 17))
        exercise(binary, "shipping-structure", structure, expected=(19, 17))
        image = run / f"ProfiledWinUI-{level}"
        shutil.copy2(binary, image)
        with image.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        results[level] = {"bytes": image.stat().st_size, "sha256": digest, "file": image.name}
        print(f"{image.stat().st_size} bytes  {image}", flush=True)
    (run / "results.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
