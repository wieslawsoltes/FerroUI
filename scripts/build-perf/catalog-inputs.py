#!/usr/bin/env python3
"""Validate catalog asset freshness and bytes with actual source code and offline Cargo.

The dependency-free fixture extracts collect, placeholder substitution,
write_if_changed, the main collection/feature block, and the complete asset loop
from each source checkout. It exercises 46 cases per revision, including an
actual rustc recompilation with deleted intermediate XAML and a fresh build
script. This is correctness evidence, not a performance benchmark. The complete
catalog compiler and dependency graph require a separate build comparison.
"""

from pathlib import Path
import argparse, hashlib, json, os, re, subprocess, time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--baseline", required=True, type=Path)
parser.add_argument("--candidate", required=True, type=Path)
parser.add_argument("--output", required=True, type=Path)
parser.add_argument(
    "--require-baseline-self-input",
    action="store_true",
    help="Require the historical baseline's first repeat to be dirty because it watches generated XAML",
)
args = parser.parse_args()
if not __debug__:
    parser.error("Run without Python optimization: correctness assertions must remain enabled")
BASE = args.baseline.resolve()
CANDIDATE = args.candidate.resolve()
ROOT = args.output.resolve()
for source in [BASE, CANDIDATE]:
    if not (source / "samples/ControlCatalog/build.rs").is_file():
        parser.error(f"Not a FerroUI checkout: {source}")
ROOT.mkdir(parents=True, exist_ok=False)
ENV = dict(
    os.environ,
    CARGO_INCREMENTAL="0",
    CARGO_TERM_COLOR="never",
    RUSTC_WRAPPER="",
    RUSTC_WORKSPACE_WRAPPER="",
    CARGO_BUILD_RUSTC_WRAPPER="",
    CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER="",
    CARGO_LOG="cargo::compiler::fingerprint=trace,cargo::core::compiler::fingerprint=trace",
)
for key in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET"]:
    ENV.pop(key, None)
for command in [[ENV.get("RUSTC", "rustc"), "-Vv"], ["cargo", "-V"]]:
    if (
        len(
            {
                subprocess.check_output(command, cwd=source, env=ENV, text=True, encoding="utf-8").strip()
                for source in [BASE, CANDIDATE]
            }
        )
        != 1
    ):
        parser.error("Both revisions must use the same compiler and Cargo")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def section(text, start, end):
    # A source refactor must be reviewed, never silently select a different body.
    if text.count(start) != 1 or text.count(end) != 1:
        raise RuntimeError(f"Catalog source extraction markers changed: {start!r} / {end!r}")
    begin = text.index(start)
    finish = text.index(end)
    if finish <= begin:
        raise RuntimeError(f"Catalog source extraction markers are out of order: {start!r}")
    return text[begin:finish]


def extracted_script(source):
    text = (source / "samples/ControlCatalog/build.rs").read_text(encoding="utf-8")
    portions = [
        section(text, "fn write_if_changed(", "\n/// Removes the files below"),
        section(text, "fn collect(", "\n/// The value of the"),
        section(text, "fn substitute_placeholders(", "\n/// Replaces the path"),
        section(text, "fn substitute_placeholder_geometries(", "\n/// What the build of the documents wrote"),
    ]
    prefix = section(text, "fn main() {", "    // The application the tests of a document start:")
    # Cargo's timestamp resolution can put this tiny fixture's writes in the same
    # tick as script start. Separate the ticks; these are correctness checks, not
    # timing measurements. The actual collected/script code is unchanged.
    prefix = prefix.replace(
        "fn main() {", "fn main() {\n    std::thread::sleep(std::time::Duration::from_millis(20));", 1
    )
    table = section(
        text,
        "    let separate_assets =",
        '    text.push_str("];\\n");\n    text.push_str("#[cfg(test)]\\npub(crate) static DOCUMENT_ASSETS:',
    )
    if (
        table.count("    for (asset_path, path) in &assets {") != 1
        or table.count("let table = if compiled.documents.contains(asset_path)") != 1
    ):
        raise RuntimeError("The actual catalog asset loop changed; review fixture extraction")
    constants = "\n".join(
        line
        for line in text.splitlines()
        if line.startswith(
            (
                "const PLACEHOLDER_DIRECTORY:",
                "const SKIPPED_DIRECTORIES:",
                "const ASSET_FILES:",
                "const CLASSIC_THEMES_DOCUMENT:",
                "const ASSEMBLY_NAME:",
                "const SITE_ASSET_DIRECTORY:",
            )
        )
    )
    source_code = (
        'use std::{collections::{BTreeMap,BTreeSet},env,fmt::Write as _,fs,path::{Path,PathBuf}};\n#[path="build/placeholder_geometry.rs"]mod placeholder_geometry;\n'
        + constants
        + "\n"
        + "\n".join(portions)
        + "\nstruct CompiledMarkup{documents:BTreeSet<String>}\n"
        + prefix
        + """
    let compiled = CompiledMarkup { documents: assets.iter().filter(|(name,_)| name.ends_with(".xaml") && env::var_os("CARGO_FEATURE_RUNTIME_MARKUP").is_none()).map(|(name,_)|name.clone()).collect() };
"""
        + table
        + """
    text.push_str("];\\n");
    // The fixture reads both tables; the actual catalog makes DOCUMENT_ASSETS cfg(test).
    text.push_str("pub(crate) static DOCUMENT_ASSETS: &[(&str, &[u8])] = &[\\n");
    text.push_str(&document_assets);
    text.push_str("];\\n");
    write_if_changed(&out_dir.join("assets.rs"), text.as_bytes());
}
"""
    )
    return source_code, {
        "helper_sections_sha256": [sha(p.encode()) for p in portions],
        "actual_asset_loop_sha256": sha(table.encode()),
        "main_prefix_sha256": sha(prefix.encode()),
        "source_build_rs_sha256": sha(text.encode()),
        "contains_original_unguarded_asset_watch": (
            '        println!("cargo::rerun-if-changed={}", path.display());\n'
            '        if separate_assets && !asset_path.ends_with(".xaml") {'
        )
        in table,
    }


main_code = """use std::io::Write;
include!(concat!(env!("OUT_DIR"),"/assets.rs"));
fn main(){
 let mut stdout=std::io::stdout().lock();
 for(kind,table)in [(0u8,ASSETS),(1u8,DOCUMENT_ASSETS)]{
  for(name,data)in table{
   stdout.write_all(&[kind]).unwrap();
   stdout.write_all(&(name.len()as u64).to_le_bytes()).unwrap();
   stdout.write_all(&(data.len()as u64).to_le_bytes()).unwrap();
   stdout.write_all(name.as_bytes()).unwrap();stdout.write_all(data).unwrap();
  }
 }
}
"""
unicode_document = """<ResourceDictionary xmlns="https://github.com/wieslawsoltes/FerroUI" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">\r\n<StreamGeometry x:Key="AppIcon">M9 9</StreamGeometry>\n<StreamGeometry x:Key="ExtraIcon">M2 2</StreamGeometry>\n<TextBlock Text="雪 🦀 &quot;quoted&quot; C:\\art\\icon"/>\n<!-- literal backslash \\ and tab\t, Unicode line separator \u2028 and paragraph separator \u2029 -->\n</ResourceDictionary>\n"""
app = (BASE / "samples/ControlCatalog/App.xaml").read_bytes()
geometry = (BASE / "samples/ControlCatalog/PlaceholderAssets/StreamGeometry/AppIcon.txt").read_bytes()
for relative in [
    "samples/ControlCatalog/App.xaml",
    "samples/ControlCatalog/PlaceholderAssets/StreamGeometry/AppIcon.txt",
    "samples/ControlCatalog/build/placeholder_geometry.rs",
]:
    if (BASE / relative).read_bytes() != (CANDIDATE / relative).read_bytes():
        raise RuntimeError(f"Fixture source input differs between revisions: {relative}")
manifest = """[package]
name="catalog-actual-code-fixture"
version="0.1.0"
edition="2021"
[workspace]
[[bin]]
name="catalog-actual-code-fixture"
path="main.rs"
[features]
default=["placeholder-branding"]
placeholder-branding=[]
classic-themes=[]
runtime-markup=[]
separate-assets=[]
"""


def parse_runtime(data):
    result = {}
    i = 0
    while i < len(data):
        kind = data[i]
        keylen = int.from_bytes(data[i + 1 : i + 9], "little")
        length = int.from_bytes(data[i + 9 : i + 17], "little")
        i += 17
        key = data[i : i + keylen].decode()
        i += keylen
        content = data[i : i + length]
        i += length
        assert len(content) == length and (kind, key) not in result
        result[kind, key] = content
    assert i == len(data)
    return result


def expected_runtime(source, features):
    branding = "feature-off" not in features
    runtime = "runtime-markup" in features
    separate = "separate-assets" in features
    expected = {}
    site = {}
    for path in sorted(source.rglob("*")):
        if not path.is_file():
            continue
        name = path.relative_to(source).as_posix()
        if name.startswith(("build/", "PlaceholderAssets/")):
            continue
        if not (name.endswith(".xaml") or name.startswith("Assets/")):
            continue
        content = path.read_bytes()
        if branding:
            if name.startswith("Assets/") and (source / "PlaceholderAssets" / path.name).is_file():
                content = (source / "PlaceholderAssets" / path.name).read_bytes()
            elif name.endswith(".xaml"):
                text = content.decode()
                for geometry in sorted((source / "PlaceholderAssets/StreamGeometry").glob("*.txt")):
                    key = geometry.stem
                    value = geometry.read_text(encoding="utf-8").strip()
                    text = re.sub(
                        '(<StreamGeometry x:Key="' + re.escape(key) + '">).*?(</StreamGeometry>)',
                        lambda m: m[1] + value + m[2],
                        text,
                        flags=re.S,
                    )
                content = text.encode()
        if separate and not name.endswith(".xaml"):
            site[name] = content
            continue
        expected[(1 if name.endswith(".xaml") and not runtime else 0, "/" + name)] = content
    return expected, site


report = {
    "schema_version": 1,
    "complete": False,
    "source_revisions": {
        label: subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
        for label, source in [("baseline", BASE), ("candidate", CANDIDATE)]
    },
    "compiler": subprocess.check_output([ENV.get("RUSTC", "rustc"), "-Vv"], env=ENV, text=True).strip(),
    "cargo": subprocess.check_output(["cargo", "-V"], env=ENV, text=True).strip(),
    "scope": "dependency-free real-Cargo fixture extracting actual catalog collect, placeholder substitution, write_if_changed, and asset table loop; does not compile the XAML type-system or full catalog graph",
    "timestamp_tick_separation_ms": 20,
    "harness_sha256": sha(Path(__file__).read_bytes()),
    "require_baseline_self_input": args.require_baseline_self_input,
    "sources": {},
    "cases": [],
}
reference = {}
for label, repo in [("baseline", BASE), ("candidate", CANDIDATE)]:
    source = ROOT / label
    source.mkdir()
    (source / "Assets").mkdir()
    (source / "PlaceholderAssets/StreamGeometry").mkdir(parents=True)
    (source / "build").mkdir()
    (source / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (source / "main.rs").write_text(main_code, encoding="utf-8")
    (source / "App.xaml").write_bytes(app)
    (source / "Unicode.xaml").write_bytes(unicode_document.encode())
    (source / "Assets/raw.bin").write_bytes(bytes(range(256)))
    (source / "Assets/logo.txt").write_bytes(b"original brand asset")
    (source / "PlaceholderAssets/logo.txt").write_bytes(b"placeholder brand asset")
    (source / "PlaceholderAssets/StreamGeometry/AppIcon.txt").write_bytes(geometry)
    (source / "build/placeholder_geometry.rs").write_bytes(
        (repo / "samples/ControlCatalog/build/placeholder_geometry.rs").read_bytes()
    )
    script, provenance = extracted_script(repo)
    (source / "build.rs").write_text(script, encoding="utf-8")
    report["sources"][label] = provenance
    target = ROOT / f"target-{label}"
    env = dict(ENV, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], cwd=source, env=env, capture_output=True, check=True
    )
    state = {}

    def run(phase, features=()):
        command = ["cargo", "build", "--locked", "--offline", "-vv", "--message-format=json"]
        if "feature-off" in features:
            command.append("--no-default-features")
        for feature in features:
            if feature != "feature-off":
                command.extend(["--features", feature])
        dest = ROOT / f"{label}-{phase}"
        dest.mkdir()
        p = subprocess.run(command, cwd=source, env=env, text=True, encoding="utf-8", capture_output=True)
        (dest / "cargo.stdout").write_text(p.stdout, encoding="utf-8")
        (dest / "cargo.stderr").write_text(p.stderr, encoding="utf-8")
        if p.returncode:
            raise RuntimeError(f"{label} {phase}:\n{p.stderr[-8000:]}")
        messages = [json.loads(line) for line in p.stdout.splitlines() if line.startswith("{")]
        units = [
            {
                "name": m["target"]["name"],
                "features": m["features"],
                "profile": m["profile"],
                "fresh": m["fresh"],
            }
            for m in messages
            if m.get("reason") == "compiler-artifact"
        ]
        executables = [
            m["executable"]
            for m in messages
            if m.get("reason") == "compiler-artifact"
            and m["target"]["name"] == "catalog-actual-code-fixture"
            and m.get("executable")
        ]
        if len(executables) != 1:
            raise RuntimeError(f"Expected one fixture executable, got {executables}")
        raw = subprocess.check_output(executables)
        actual = parse_runtime(raw)
        expected, site = expected_runtime(source, features)
        assert actual == expected, (
            label,
            phase,
            "runtime bytes differ",
            [(k, len(v)) for k, v in actual.items()],
            [(k, len(v)) for k, v in expected.items()],
        )
        outputs = [Path(m["out_dir"]) for m in messages if m.get("reason") == "build-script-executed"]
        assert len(outputs) == 1
        outdir = outputs[0]
        actual_site = {
            p.relative_to(outdir / "browser-site/assets/ControlCatalog").as_posix(): p.read_bytes()
            for p in (outdir / "browser-site/assets/ControlCatalog").rglob("*")
            if p.is_file()
        }
        if "separate-assets" in features:
            assert actual_site == site
        watches = [
            line.split("=", 1)[1]
            for line in (outdir.parent / "output").read_text(encoding="utf-8").splitlines()
            if line.startswith("cargo::rerun-if-changed=")
        ]
        record = {
            "label": label,
            "phase": phase,
            "features": list(features),
            "all_units_fresh": all(u["fresh"] for u in units),
            "units": units,
            "runtime_byte_hash": sha(raw),
            "runtime_asset_count": len(actual),
            "runtime_bytes_match_expected": True,
            "intermediate_files": sorted(
                p.relative_to(outdir).as_posix() for p in (outdir / "placeholder").rglob("*.xaml")
            ),
            "generated_intermediates_watched": sum(str(outdir) in path for path in watches),
            "dirty_reasons": [
                line.strip().replace(str(source), "$SOURCE").replace(str(target), "$TARGET")
                for line in p.stderr.splitlines()
                if line.lstrip().startswith("Dirty ")
            ],
        }
        compare = {
            "runtime": raw,
            "site": site,
            "units": [{k: v for k, v in u.items() if k != "fresh"} for u in units],
        }
        if label == "baseline":
            reference[phase] = compare
        else:
            assert compare == reference[phase], (phase, "baseline/candidate mismatch")
        if label == "candidate" and ("repeat" in phase or phase == "delete-intermediate"):
            assert record["all_units_fresh"], (phase, record["dirty_reasons"])
        if label == "baseline" and phase == "repeat-initial":
            record["baseline_self_input_reproduced"] = not record["all_units_fresh"] and any(
                "/out/placeholder/" in reason.replace("\\", "/") for reason in record["dirty_reasons"]
            )
            if args.require_baseline_self_input and not record["baseline_self_input_reproduced"]:
                raise RuntimeError(
                    "The requested historical baseline did not reproduce the generated-placeholder input defect"
                )
        if label == "candidate":
            assert record["generated_intermediates_watched"] == 0
        state.update(executable=Path(executables[0]), outdir=outdir, record=record)
        report["cases"].append(record)
        (ROOT / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(
            label, phase, "fresh=" + str(record["all_units_fresh"]), "assets=" + str(len(actual)), flush=True
        )
        return outdir

    def change(fn):
        time.sleep(0.02)
        fn()

    out = run("populate")
    run("repeat-initial")
    run("repeat-initial-settled")
    changes = [
        (
            "source-edit",
            lambda: (source / "App.xaml").write_bytes(
                (source / "App.xaml").read_bytes() + b"\n<!-- edited original source -->\n"
            ),
        ),
        (
            "geometry-edit",
            lambda: (source / "PlaceholderAssets/StreamGeometry/AppIcon.txt").write_text(
                "M0 0 L1 0 L1 1 Z\n"
            ),
        ),
        (
            "geometry-add",
            lambda: (source / "PlaceholderAssets/StreamGeometry/ExtraIcon.txt").write_text("M4 4 L5 5 Z\n"),
        ),
        ("geometry-remove", lambda: (source / "PlaceholderAssets/StreamGeometry/ExtraIcon.txt").unlink()),
        (
            "document-add",
            lambda: (source / "Added.xaml").write_text(
                '<ResourceDictionary><StreamGeometry x:Key="AppIcon">M3 3</StreamGeometry></ResourceDictionary>\n'
            ),
        ),
        ("document-remove", lambda: (source / "Added.xaml").unlink()),
        ("ordinary-asset-edit", lambda: (source / "Assets/raw.bin").write_bytes(bytes(reversed(range(256))))),
        ("remove-last-geometry", lambda: (source / "PlaceholderAssets/StreamGeometry/AppIcon.txt").unlink()),
        (
            "restore-last-geometry",
            lambda: (source / "PlaceholderAssets/StreamGeometry/AppIcon.txt").write_bytes(geometry),
        ),
    ]
    for name, fn in changes:
        change(fn)
        run(name)
        run(name + "-repeat")
        run(name + "-repeat-settled")

    def delete_intermediates():
        for path in (out / "placeholder").rglob("*.xaml"):
            path.unlink()

    change(delete_intermediates)
    run("delete-intermediate")
    run("delete-intermediate-repeat")
    run("delete-intermediate-repeat-settled")
    # Force a new rustc invocation while preserving every build-script artifact.
    # A successful unchanged Cargo build alone could hide a missing include behind
    # an old executable. Deleting only consumer fingerprints/output removes that
    # ambiguity and works across platforms without depending on rustc hash names.
    change(delete_intermediates)
    script_timestamps = {
        name: (out.parent / name).stat().st_mtime_ns for name in ["output", "invoked.timestamp"]
    }
    consumer_fingerprints = list((target / "debug/.fingerprint").glob("*/bin-catalog-actual-code-fixture"))
    if not consumer_fingerprints:
        raise RuntimeError("Consumer fingerprint layout changed; cannot force the rustc-only check")
    for fingerprint in consumer_fingerprints:
        fingerprint.unlink()
    state["executable"].unlink()
    run("recompile-without-intermediate")
    record = state["record"]
    consumer = [unit for unit in record["units"] if unit["name"] == "catalog-actual-code-fixture"]
    scripts = [unit for unit in record["units"] if unit["name"] == "build-script-build"]
    assert len(consumer) == 1 and not consumer[0]["fresh"] and len(scripts) == 1 and scripts[0]["fresh"]
    if label == "candidate":
        assert not record["intermediate_files"], "Rustc-only check unexpectedly regenerated intermediate XAML"
        assert script_timestamps == {
            name: (out.parent / name).stat().st_mtime_ns for name in script_timestamps
        }, "Build script ran during the rustc-only check"
        record["rustc_recompiled_with_missing_intermediate_and_fresh_build_script"] = True
    (ROOT / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    change(
        lambda: (source / "App.xaml").write_bytes(
            (source / "App.xaml").read_bytes() + b"\n<!-- recovery -->\n"
        )
    )
    recovered = run("source-edit-recovery")
    assert len(list((recovered / "placeholder").rglob("*.xaml"))) == 2
    run("source-edit-recovery-repeat")
    run("source-edit-recovery-repeat-settled")
    for features in [("feature-off",), ("runtime-markup",), ("separate-assets",)]:
        name = "feature-" + features[0]
        run(name, features)
        run(name + "-repeat", features)
        run(name + "-repeat-settled", features)
report["complete"] = True
report["summary"] = {
    "cases_per_revision": len(report["cases"]) // 2,
    "baseline_self_input_reproduced": next(
        case["baseline_self_input_reproduced"]
        for case in report["cases"]
        if case["label"] == "baseline" and case["phase"] == "repeat-initial"
    ),
    "baseline_and_candidate_build_scripts_identical": (
        report["sources"]["baseline"]["source_build_rs_sha256"]
        == report["sources"]["candidate"]["source_build_rs_sha256"]
    ),
    "all_runtime_bytes_and_site_outputs_equal": True,
    "candidate_all_unchanged_repeats_fresh": True,
    "candidate_no_generated_intermediate_watches": True,
    "actual_collection_substitution_table_code_used": True,
    "deletion_does_not_break_existing_or_next_changed_build": True,
    "rustc_only_recompile_with_deleted_intermediate_succeeds": True,
    "source_geometry_document_asset_transitions_validated": True,
    "unicode_escaping_crlf_and_binary_assets_preserved": True,
    "feature_off_runtime_markup_separate_assets_parity": True,
}
(ROOT / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report["summary"], indent=2), flush=True)
