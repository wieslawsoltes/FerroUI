#!/usr/bin/env python3
"""Compare public registration/conversion fixtures against preserved release rlib graphs.

Build each graph with profile_winui.py first and preserve its target rlibs. Pass
the matching profile separately for each revision; a fixed-input comparison may
deliberately use the same profile. This driver never creates training profiles.
Use --build-only followed by --measure-only to keep compilation out of timings.
Native-linker provenance uses FERROUI_CC (default /usr/bin/cc) and the selected
rustc sysroot's bundled LLD, falling back to ld.lld on PATH when absent.
Use --capture-native on both build and measure invocations to retain each final
FatLTO object and original CC arguments in <label>-<fixture>.native directories.
Capture builds require fresh capture directories; the measured flags are unchanged.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import time

from compact_unwind_link import RESERVE, inspect


HERE = Path(__file__).resolve().parent
FIXTURES = {
    "registration": HERE / "registration_hot_paths.rs",
    "interfaces": HERE / "interface_registration.rs",
}
LANES = {
    "register_element": 5_000,
    "property_registration": 500,
    "boxed_live": 500_000,
    "borrowed_live": 500_000,
    "boxed_wrong": 500_000,
    "borrowed_wrong": 500_000,
    "boxed_expired": 500_000,
    "borrowed_expired": 500_000,
}

NATIVE_CAPTURE_WRAPPER = r'''#!/usr/bin/env python3
"""Capture the original FatLTO input, then execute CC with unchanged arguments."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys

arguments = sys.argv[1:]
compiler = os.environ.get("FERROUI_CAPTURE_REAL_CC", "/usr/bin/cc")
if Path(compiler).resolve() == Path(__file__).resolve():
    raise RuntimeError("native capture compiler must not be the capture wrapper")
# Version queries and the compact-unwind relinks pass straight through. Only
# the ordinary link receives the original .rcgu.o, which rustc later removes.
if any(argument.endswith(".rcgu.o") for argument in arguments):
    directory = Path(os.environ["FERROUI_CAPTURE_DIR"])
    manifest = directory / "original-link.json"
    if manifest.exists():
        raise RuntimeError("refusing to overwrite an original native link capture")
    objects, replacements = [], {}
    for index, argument in enumerate(arguments):
        source = Path(argument)
        if argument.endswith(".o") and source.is_file():
            destination = directory / (f"{index:03d}-" + source.name)
            with source.open("rb") as incoming, destination.open("xb") as outgoing:
                shutil.copyfileobj(incoming, outgoing)
            with destination.open("rb") as stream:
                sha = hashlib.file_digest(stream, "sha256").hexdigest()
            objects.append({"original": argument, "captured": str(destination), "sha256": sha,
                            "bytes": destination.stat().st_size,
                            "fat_lto_object": argument.endswith(".rcgu.o")})
            replacements[argument] = str(destination)
    record = {"cwd": os.getcwd(), "command": [compiler, *arguments], "objects": objects,
              "command_with_captured_objects": [compiler, *[replacements.get(a, a) for a in arguments]]}
    with manifest.open("x") as output:
        json.dump(record, output, indent=2)
        output.write("\n")
os.execv(compiler, [compiler, *arguments])
'''


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def dependency_artifacts(directory):
    if not directory.is_dir():
        raise RuntimeError(f"dependency search directory does not exist: {directory}")
    return {
        path.name: digest(path) for path in sorted(directory.iterdir())
        if path.is_file() and (path.suffix in {".rlib", ".rmeta", ".so", ".a", ".o"}
                               or ".so." in path.name)
    }


def executable_identity(command, env):
    executable = shutil.which(str(command), path=env.get("PATH"))
    if executable is None:
        raise RuntimeError(f"executable not found: {command}")
    # Preserve the invocation name for multicall executables such as ld.lld.
    invoked_as = Path(executable).absolute()
    resolved = invoked_as.resolve()
    return {
        "invoked_as": str(invoked_as), "path": str(resolved), "sha256": digest(resolved),
        "version": subprocess.check_output([str(invoked_as), "--version"], env=env, text=True,
                                           stderr=subprocess.STDOUT).strip(),
    }


def native_linker_identity(rustc, host, env):
    sysroot = Path(subprocess.check_output([rustc, "--print=sysroot"], env=env, text=True).strip()).resolve()
    bundled = sysroot / "lib" / "rustlib" / host / "bin" / "gcc-ld" / "ld.lld"
    cc = executable_identity(env.get("FERROUI_CC", "/usr/bin/cc"), env)
    lld = executable_identity(bundled if bundled.is_file() else "ld.lld", env)
    lld["source"] = "rustc sysroot" if bundled.is_file() else "PATH fallback"
    # Rust's bundled ld.lld can be a shim which invokes the sibling rust-lld.
    backend = bundled.parent.parent / "rust-lld"
    if bundled.is_file() and backend.is_file():
        lld["backend"] = {"path": str(backend.resolve()), "sha256": digest(backend)}
    # Keep the compiler driver and its normal -fuse-ld lookup on these identities.
    env["FERROUI_CC"] = cc["invoked_as"]
    env["PATH"] = str(Path(lld["invoked_as"]).parent) + os.pathsep + env.get("PATH", os.defpath)
    return {"cc": cc, "lld": lld}


def prepare_native_capture(binary, env, compiler, measure_only):
    directory = binary.with_name(binary.name + ".native")
    wrapper = directory / "capture-cc.py"
    if not measure_only:
        if directory.exists() and any(path != wrapper for path in directory.iterdir()):
            raise RuntimeError(f"{directory}: existing native capture; use a fresh work directory")
        directory.mkdir(parents=True, exist_ok=True)
        wrapper.write_text(NATIVE_CAPTURE_WRAPPER)
        wrapper.chmod(0o755)
    capture_env = env.copy()
    capture_env.update(FERROUI_CC=str(wrapper), FERROUI_CAPTURE_REAL_CC=compiler["invoked_as"],
                       FERROUI_CAPTURE_DIR=str(directory))
    wrapper_identity = executable_identity(wrapper, capture_env)
    if wrapper_identity["sha256"] != hashlib.sha256(NATIVE_CAPTURE_WRAPPER.encode()).hexdigest():
        raise RuntimeError(f"{wrapper}: capture wrapper changed; rebuild before measuring")
    if wrapper_identity["version"] != compiler["version"]:
        raise RuntimeError(f"{wrapper}: compiler version forwarding differs")
    return capture_env, {"directory": str(directory), "wrapper": wrapper_identity, "compiler": compiler}


def read_native_capture(identity):
    directory = Path(identity["directory"])
    manifest = directory / "original-link.json"
    record = json.loads(manifest.read_text())
    if record["command"][0] != identity["compiler"]["invoked_as"]:
        raise RuntimeError(f"{manifest}: unexpected captured compiler")
    if sum(item["fat_lto_object"] for item in record["objects"]) != 1:
        raise RuntimeError(f"{manifest}: require exactly one captured FatLTO object")
    for item in record["objects"]:
        path = Path(item["captured"])
        if not path.resolve().is_relative_to(directory.resolve()):
            raise RuntimeError(f"{manifest}: captured object is outside its capture directory")
        if path.stat().st_size != item["bytes"] or digest(path) != item["sha256"]:
            raise RuntimeError(f"{path}: captured native object changed")
    return {"manifest": {"path": str(manifest), "sha256": digest(manifest)}, **record}


def paired_ratio(candidate, reference):
    ratios = [c / r for c, r in zip(candidate, reference, strict=True)]
    rng = random.Random(9961)
    samples = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(7_000))
    return {"median": statistics.median(ratios), "bootstrap_95_percent": [samples[174], samples[6824]]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for label in ("reference", "candidate"):
        parser.add_argument(f"--{label}-graph", type=Path, required=True,
                            help="directory containing the exact release rlibs, including ferroui_base")
        parser.add_argument(f"--{label}-profile", type=Path, required=True,
                            help="LLVM profile used to build this graph")
        parser.add_argument(f"--{label}-dependency", type=Path, action="append", default=[],
                            help="additional dependency search directory, if needed (repeatable)")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--cpu", type=int, required=True, help="one allowed Linux CPU on which to build and measure")
    parser.add_argument("--pairs", type=int, default=31)
    parser.add_argument("--interface-rounds", type=int, default=4096)
    parser.add_argument("--capture-native", action="store_true",
                        help="preserve final FatLTO objects and original CC arguments; use on build and measure calls")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--build-only", action="store_true")
    mode.add_argument("--measure-only", action="store_true")
    args = parser.parse_args()
    if args.pairs < 3 or args.interface_rounds < 1:
        parser.error("--pairs must be at least 3 and --interface-rounds must be positive")
    if not hasattr(os, "sched_getaffinity") or args.cpu not in os.sched_getaffinity(0):
        parser.error("--cpu must name an allowed Linux CPU")
    os.sched_setaffinity(0, {args.cpu})
    args.work_dir = args.work_dir.resolve()
    args.work_dir.mkdir(parents=True, exist_ok=True)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    for key in ("FERROUI_PGO_TRAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    # Isolate even an accidentally supplied instrumented object from training.
    env.update(LLVM_PROFILE_FILE=str(args.work_dir / "qa-only-%m-%p.profraw"),
               RAYON_NUM_THREADS="1", MALLOC_ARENA_MAX="1")
    toolchain = subprocess.check_output([args.rustc, "-vV"], text=True, env=env)
    if "host: x86_64-unknown-linux-gnu" not in toolchain:
        parser.error("the thread CPU probe and ELF link policy require native x86-64 GNU/Linux")
    native_linker = native_linker_identity(args.rustc, "x86_64-unknown-linux-gnu", env)
    report = {
        "method": "Same public API sources; production opt-z, FatLTO, unwind, safe ICF, RELR, guarded compact "
                  "unwind (actual application recorded per binary). "
                  "Two warmup rounds, rotated lane order, alternating fresh processes pinned to one CPU. "
                  "Element/property results use thread CPU ns per operation; interface results use wall ns "
                  "per class registration or successful conversion. No held-out fixture enters PGO training.",
        "cpu": args.cpu, "pairs": args.pairs, "toolchain": toolchain, "native_linker": native_linker,
        "interface_rounds": args.interface_rounds, "lane_iterations": LANES,
        "inputs": {}, "builds": {}, "samples": {"reference": {}, "candidate": {}},
        "raw_samples": {"reference": {}, "candidate": {}},
    }
    for label in ("reference", "candidate"):
        graph = getattr(args, f"{label}_graph").resolve()
        profile = getattr(args, f"{label}_profile").resolve()
        bases = list(graph.glob("libferroui_base-*.rlib"))
        if len(bases) != 1 or not profile.is_file():
            parser.error(f"{label}: require one Base rlib and a readable profile")
        extra = [p.resolve() for p in getattr(args, f"{label}_dependency")]
        artifacts = {str(directory): dependency_artifacts(directory)
                     for directory in dict.fromkeys([graph, *extra])}
        inputs = {"graph": str(graph), "profile": str(profile), "profile_sha256": digest(profile),
                  "rlib_sha256": {name: sha for name, sha in artifacts[str(graph)].items()
                                  if name.endswith(".rlib")},
                  "additional_dependencies": [str(p) for p in extra],
                  "dependency_artifacts_sha256": artifacts}
        report["inputs"][label] = inputs
        report["builds"][label] = {}
        for name, source in FIXTURES.items():
            binary = args.work_dir / f"{label}-{name}"
            manifest = binary.with_suffix(".json")
            command = [args.rustc, str(source), "--edition=2021", "--crate-name", f"{name}_probe",
                       "--target", "x86_64-unknown-linux-gnu", "-Cpanic=unwind", "-Copt-level=z",
                       "-Clto=fat", "-Ccodegen-units=1", f"-Cprofile-use={profile}",
                       "-Cllvm-args=--addrsig", "-Cstrip=symbols", "-Clink-arg=-fuse-ld=lld",
                       "-Clink-arg=-Wl,--icf=safe",
                       "-Clink-arg=-Wl,-z,pack-relative-relocs",
                       f"-Clinker={HERE / 'compact_unwind_link.py'}",
                       f"-Ldependency={graph}"]
            command.extend(f"-Ldependency={p}" for p in extra)
            command.extend(["--extern", f"ferroui_base={bases[0]}", "-o", str(binary)])
            identity = {"command": command, "source_sha256": digest(source), "inputs": inputs,
                        "toolchain": toolchain, "native_linker": native_linker, "linker_source_sha256": {
                            name: digest(HERE / name) for name in
                            ("compact_unwind_link.py", "compact_unwind.py", "attribute_codegen.py")}}
            build_env = env
            if args.capture_native:
                build_env, identity["native_capture"] = prepare_native_capture(
                    binary, env, native_linker["cc"], args.measure_only)
            if args.measure_only:
                saved = json.loads(manifest.read_text())
                if saved["identity"] != identity or saved["sha256"] != digest(binary):
                    raise RuntimeError(f"{binary}: inputs or artifact changed; rebuild before measuring")
                if args.capture_native and saved["native_capture"] != read_native_capture(identity["native_capture"]):
                    raise RuntimeError(f"{binary}: native capture changed; rebuild before measuring")
            else:
                started = time.monotonic()
                with binary.with_suffix(".log").open("w") as log:
                    subprocess.run(command, env=build_env, stdout=log, stderr=log, check=True)
                saved = {"identity": identity, "build_seconds": time.monotonic() - started,
                         "bytes": binary.stat().st_size, "sha256": digest(binary),
                         "compact_unwind_applied": RESERVE in inspect(binary)[2]}
                if args.capture_native:
                    saved["native_capture"] = read_native_capture(identity["native_capture"])
                manifest.write_text(json.dumps(saved, indent=2) + "\n")
            report["builds"][label][name] = saved
        subprocess.run([args.work_dir / f"{label}-registration", "check"], env=env, check=True)
        subprocess.run([args.work_dir / f"{label}-interfaces", "16"], env=env, check=True,
                       stdout=subprocess.DEVNULL)
    if args.build_only:
        args.out.write_text(json.dumps(report, indent=2) + "\n")
        print("Both fixture pairs built and passed their public API assertions.", flush=True)
        return

    checksums, allocations = {}, {}
    lanes = list(LANES) + ["interfaces"]
    for round_index in range(-2, args.pairs):
        order = ("reference", "candidate") if round_index % 2 == 0 else ("candidate", "reference")
        offset = abs(round_index) % len(lanes)
        for lane in lanes[offset:] + lanes[:offset]:
            for label in order:
                if lane == "interfaces":
                    output = subprocess.check_output([args.work_dir / f"{label}-interfaces",
                                                      str(args.interface_rounds)], env=env, text=True)
                    registration, conversion, count, size = map(int, output.split())
                    if registration <= 0 or conversion <= 0 or count < 0 or size < 0:
                        raise RuntimeError(f"invalid timing/allocation result in {label} interfaces")
                    allocation = {"allocations": count, "allocated_bytes": size, "classes": 32}
                    if allocations.setdefault(label, allocation) != allocation:
                        raise RuntimeError(f"unstable allocation count in {label}")
                    sample = {"registration_wall_ns_per_class": registration / (32 * args.interface_rounds),
                              "conversion_wall_ns_per_value": conversion / (32 * args.interface_rounds)}
                    raw = {"operations": 32 * args.interface_rounds,
                           "registration_wall_ns": registration, "conversion_wall_ns": conversion,
                           "allocations": count, "allocated_bytes": size}
                else:
                    output = subprocess.check_output([args.work_dir / f"{label}-registration", lane,
                                                      str(LANES[lane])], env=env, text=True)
                    actual, operations, checksum, cpu, wall = output.split()
                    operations, checksum, cpu, wall = map(int, (operations, checksum, cpu, wall))
                    expected_operations = LANES[lane] * (8 if lane in ("register_element", "property_registration") else 1)
                    if actual != lane or operations != expected_operations or checksums.setdefault(lane, checksum) != checksum:
                        raise RuntimeError(f"workload/result mismatch in {label} {lane}")
                    if cpu <= 0 or wall <= 0:
                        raise RuntimeError(f"nonpositive duration in {label} {lane}")
                    sample = {"thread_cpu_ns_per_operation": cpu / operations,
                              "wall_ns_per_operation": wall / operations}
                    raw = {"operations": operations, "checksum": checksum,
                           "thread_cpu_ns": cpu, "wall_ns": wall}
                if round_index >= 0:
                    report["samples"][label].setdefault(lane, []).append(sample)
                    report["raw_samples"][label].setdefault(lane, []).append(raw)
    report["checksums"] = checksums
    report["interface_warmed_allocations"] = allocations
    report["comparisons"] = {}
    for lane in lanes:
        reference = report["samples"]["reference"][lane]
        candidate = report["samples"]["candidate"][lane]
        report["comparisons"][lane] = {}
        for metric in reference[0]:
            r, c = [s[metric] for s in reference], [s[metric] for s in candidate]
            report["comparisons"][lane][metric] = {
                "reference_median": statistics.median(r), "candidate_median": statistics.median(c),
                "candidate_reference_ratio": paired_ratio(c, r),
            }
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"comparisons": report["comparisons"],
                      "interface_warmed_allocations": allocations}, indent=2), flush=True)


if __name__ == "__main__":
    main()
