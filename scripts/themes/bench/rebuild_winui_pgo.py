#!/usr/bin/env python3
"""Rebuild one frozen WinUI revision with its own original-workload PGO profile.

Use the same disposable --source checkout and --shared-work paths for both
revisions, and a different initially empty --work-dir for each result. This
driver records correctness/build provenance, not compile-time benchmarks.
It never checks out revisions, changes framework source, or trains QA fixtures.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import sys
import tarfile
import time
import tomllib

HOST = "x86_64-unknown-linux-gnu"
MALLOC = "narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false"
HERE = Path(__file__).resolve().parent
SHIPPING_MAIN_SHA256 = "8a1325ededc43128ad7be5cb8d5c3f4975865ec582ef2ac8bc9fea62cd1bb269"
TRAINING_INPUT_SHA256 = {
    "main.rs": "d9c9306b8de920a2b6fc72b9db9409937cf343b3c547b0e3fcd75badcfc54bcd",
    "property_work.rs": "9e04adc991cf9342de2fb0846fa03ac72e372b2937b0e01a2e5d7b86d3e57970",
    "template_work.rs": "37824fc1775d438a62d01406f115096408e60b032dc91589483dc7b8b4fa34e2",
}

RUSTC_RECORDER = r'''#!/usr/bin/env python3
from pathlib import Path
import json,os,resource,shutil,signal,subprocess,sys,time
args=sys.argv[1:];args[0]=shutil.which(args[0]) or args[0]
if '--crate-name' not in args:os.execv(args[0],args)
crate=args[args.index('--crate-name')+1]
target=args[args.index('--target')+1] if '--target' in args else 'host'
directory=Path(os.environ['FERRO_COMPILER_RECORDS']);directory.mkdir(parents=True,exist_ok=True)
path=directory/f'{time.time_ns()}-{os.getpid()}-{crate}-{target}.json'
record={'argv':args,'cwd':os.getcwd(),'crate':crate,'target':target,'started_unix':time.time(),
        'environment':{k:v for k,v in os.environ.items() if k.startswith('CARGO_PKG_') or k in
        ['OUT_DIR','CARGO_MANIFEST_DIR','CARGO_CRATE_NAME','TMPDIR','RAYON_NUM_THREADS',
         'MALLOC_ARENA_MAX','MALLOC_CONF','CARGO_ENCODED_RUSTFLAGS','LLVM_PROFILE_FILE']}}
def save():
    temp=path.with_suffix('.json.tmp');temp.write_text(json.dumps(record,indent=2)+'\n');temp.replace(path)
save()
if os.environ.get('FERRO_REQUIRE_TARGET_BITCODE')=='1' and target!='host' and '--crate-type' in args:
    kind=args[args.index('--crate-type')+1]
    if kind in ('lib','rlib'):
        options=[args[i+1] for i,a in enumerate(args[:-1]) if a=='-C']
        options += [a[2:] for a in args if a.startswith('-C') and len(a)>2]
        if not any(a in ('linker-plugin-lto','linker-plugin-lto=yes') for a in options):
            record.update(returncode=2,policy_error='Target library must use Cargo dependency linker-plugin-lto')
            save();raise SystemExit(2)
result=subprocess.run(args,close_fds=False)
record.update(returncode=result.returncode,finished_unix=time.time(),
              compiler_child_maxrss_kib=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss)
save()
if result.returncode<0:os.kill(os.getpid(),-result.returncode)
raise SystemExit(result.returncode)
'''

CC_RECORDER = r'''#!/usr/bin/env python3
from pathlib import Path
import fcntl,hashlib,json,os,shutil,subprocess,sys,time
args=sys.argv[1:];cc=os.environ['FERRO_CAPTURE_CC'];root=Path(os.environ['FERRO_CAPTURE_DIRECTORY'])
if '-o' not in args:os.execv(cc,[cc,*args])
root.mkdir(parents=True,exist_ok=True);objects=root/'objects';objects.mkdir(exist_ok=True)
with (root/'counter').open('a+') as counter:
    fcntl.flock(counter,fcntl.LOCK_EX);counter.seek(0);index=int(counter.read() or '0')+1
    counter.seek(0);counter.truncate();counter.write(str(index));counter.flush()
def identity(path):
    with path.open('rb') as stream:sha=hashlib.file_digest(stream,'sha256').hexdigest()
    return {'path':str(path),'bytes':path.stat().st_size,'sha256':sha}
record={'command':[cc,*args],'cwd':os.getcwd(),'started_unix':time.time(),'captured_inputs':[]}
for i,value in enumerate(args):
    p=Path(value)
    if not p.is_file() or p.suffix not in ('.o','.ld'):continue
    item=identity(p);dest=objects/(item['sha256']+p.suffix)
    if not dest.exists():shutil.copy2(p,dest)
    assert identity(dest)['sha256']==item['sha256']
    record['captured_inputs'].append({'argument_index':i,'original':item,'captured':identity(dest)})
path=root/f'{index:03d}.json';path.write_text(json.dumps(record,indent=2)+'\n')
result=subprocess.run([cc,*args],close_fds=False);record.update(returncode=result.returncode,finished_unix=time.time())
output=Path(args[args.index('-o')+1])
if result.returncode==0 and output.is_file():
    saved=root/f'{index:03d}.image';shutil.copy2(output,saved);record['output']=identity(saved)
path.write_text(json.dumps(record,indent=2)+'\n');raise SystemExit(result.returncode)
'''


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(path: Path) -> dict:
    return {"path": str(path), "sha256": digest(path), "bytes": path.stat().st_size}


def save(path: Path, value: dict) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def copy_verified(source: Path, target: Path) -> dict:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    result = identity(target)
    if result["sha256"] != digest(source):
        raise RuntimeError(f"Copy did not preserve {source}")
    return result


def source_inventory(source: Path) -> dict:
    files = subprocess.check_output(["git", "ls-files", "-z"], cwd=source).split(b"\0")
    result = {}
    for name in files:
        if not name:
            continue
        path = source / os.fsdecode(name)
        result[os.fsdecode(name)] = ({"symlink": os.readlink(path)} if path.is_symlink()
                                    else {"bytes": path.stat().st_size, "sha256": digest(path)})
    return result


def verify_source_files(source: Path, expected: dict) -> None:
    for name, value in expected.items():
        path = source / name
        actual = ({"symlink": os.readlink(path)} if path.is_symlink() else
                  {"bytes": path.stat().st_size, "sha256": digest(path)})
        if actual != value:
            raise RuntimeError(f"Frozen source identity changed: {path}")


def resources(work: Path) -> dict:
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    vm = dict(line.split() for line in Path("/proc/vmstat").read_text().splitlines())
    return {"unix": time.time(), "MemAvailable": int(memory["MemAvailable"].split()[0]) * 1024,
            "aggregate_oom_kill": int(vm["oom_kill"]), "free_disk_bytes": shutil.disk_usage(work).free}


def consumer_manifest(source: Path, packer: bool = False) -> str:
    package = "ferroui-packer-replay" if packer else "winui-theme-probe"
    text = ('[workspace]\nresolver = "2"\n[package]\nname = ' + json.dumps(package) +
            '\nversion = "0.0.0"\nedition = "2021"\n[[bin]]\n')
    if packer:
        text += ('name = "ferroui-fxd"\npath = ' +
                 json.dumps(str(source / "src/Markup/FerroUI.Markup.Xaml.Loader/bin/ferroui-fxd.rs")) +
                 '\n[dependencies]\nferroui-markup-xaml-loader = { path = ' +
                 json.dumps(str(source / "src/Markup/FerroUI.Markup.Xaml.Loader")) +
                 ', default-features = false, features = ["portable-xaml"] }\n')
    else:
        text += 'name = "winui-theme-probe"\npath = "main.rs"\n[dependencies]\n'
        for name, location, extra in [
            ("ferroui-base", "src/FerroUI.Base", ""),
            ("ferroui-controls", "src/FerroUI.Controls", ""),
            ("ferroui-themes-winui", "src/FerroUI.Themes.WinUI", ""),
            ("ferroui-markup-xaml", "src/Markup/FerroUI.Markup.Xaml", ""),
            ("ferroui-markup-xaml-loader", "src/Markup/FerroUI.Markup.Xaml.Loader",
             ', features = ["runtime", "portable-xaml"]'),
        ]:
            text += name + " = { path = " + json.dumps(str(source / location)) + extra + " }\n"
        text += '\n[features]\nretain-archive = []\nlegacy-archive = []\noperand-updates = []\n'
    return text + '\n[profile.release]\nopt-level = "s"\ncodegen-units = 16\n'


class Replay:
    def __init__(self, args: argparse.Namespace):
        self.args = args
        self.source = args.source.resolve(strict=True)
        self.work = args.work_dir.resolve()
        self.work.mkdir(parents=True, exist_ok=False)
        self.frozen_source = self.work / "source"
        self.shared = args.shared_work.resolve()
        self.shared.mkdir(parents=True, exist_ok=True)
        self.consumer = self.shared / "consumer"
        self.consumer.mkdir(exist_ok=True)
        self.target = self.shared / "target"
        self.toolchain = args.toolchain_prefix.resolve(strict=True)
        self.llvm = self.toolchain / "lib/rustlib" / HOST / "bin"
        self.rustc = self.toolchain / "bin/rustc"
        self.cargo = self.toolchain / "bin/cargo"
        self.report_path = self.work / "build.json"
        if subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=self.source):
            raise RuntimeError("--source must be a clean frozen checkout")
        cpus = {int(value) for value in args.cpus.split(",")}
        if not cpus or not cpus <= os.sched_getaffinity(0):
            raise ValueError("--cpus must contain allowed CPU numbers")
        os.sched_setaffinity(0, cpus)
        self.environment = os.environ.copy()
        for key in list(self.environment):
            if (key in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE", "RUSTC_WRAPPER", "RUSTC_BOOTSTRAP",
                        "RUSTC_WORKSPACE_WRAPPER"} or key.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_"))):
                self.environment.pop(key)
            elif key.startswith("CARGO_PROFILE_"):
                self.environment.pop(key)
        self.controlled = {
            "PATH": str(self.toolchain / "bin") + os.pathsep + str(self.llvm / "gcc-ld") + os.pathsep + self.environment["PATH"],
            "LD_LIBRARY_PATH": str(self.toolchain / "lib") +
            (os.pathsep + self.environment["LD_LIBRARY_PATH"] if self.environment.get("LD_LIBRARY_PATH") else ""),
            "RUSTC": str(self.rustc), "CARGO_HOME": str(args.cargo_home.resolve()),
            "CARGO_TARGET_DIR": str(self.target), "CARGO_INCREMENTAL": "0", "CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_RELEASE_LTO": "thin", "CARGO_PROFILE_RELEASE_OPT_LEVEL": "s",
            "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "16", "CARGO_PROFILE_RELEASE_PANIC": "unwind",
            "CARGO_PROFILE_RELEASE_DEBUG": "0", "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1",
            "CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS": "false", "CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS": "false",
            "MALLOC_CONF": MALLOC, "LLVM_PROFILE_FILE": os.devnull,
            "TMPDIR": str(self.work / "compiler-tmp"),
        }
        Path(self.controlled["TMPDIR"]).mkdir()
        self.environment.update(self.controlled)
        self.recorder = self.work / "record-rustc.py"
        self.recorder.write_text(RUSTC_RECORDER)
        self.recorder.chmod(0o755)
        self.cc_recorder = self.work / "record-cc.py"
        self.cc_recorder.write_text(CC_RECORDER)
        self.cc_recorder.chmod(0o755)
        self.original = (self.source / "scripts/themes/bench/winui.rs").read_text()
        if hashlib.sha256(self.original.encode()).hexdigest() != SHIPPING_MAIN_SHA256:
            raise RuntimeError("Shipping probe differs from the retained workload; review the protocol before building")
        spec = importlib.util.spec_from_file_location("frozen_profile_winui", self.source / "scripts/themes/bench/profile_winui.py")
        self.policy = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.policy)
        self.files = source_inventory(self.source)
        source_record = {"directory": str(self.frozen_source), "compile_directory": str(self.source),
                         "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.source, text=True).strip(),
                         "tree": subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=self.source, text=True).strip(),
                         "files": self.files}
        save(self.work / "source.json", source_record)
        self.report = {"schema": 1, "scope": __doc__, "source": identity(self.work / "source.json"),
                       "producer": identity(Path(__file__)), "rustc_recorder": identity(self.recorder),
                       "cc_recorder": identity(self.cc_recorder), "controlled_environment": self.controlled,
                       "cpu_affinity": sorted(cpus), "commands": [], "training_runs": [], "complete": False,
                       "rustc": subprocess.check_output([self.rustc, "-vV"], env=self.environment, text=True),
                       "profile_policy": "Original 13 profile_winui training launches only; each revision trains a fresh profile.",
                       "resource_scope": "Guest aggregate memory/oom counters; per-command RUSAGE_CHILDREN peak is cumulative, not a compiler attribution."}
        self.tool_paths = {"rustc": self.rustc, "cargo": self.cargo,
                           "cc": Path(shutil.which("cc", path=self.environment["PATH"])).resolve(),
                           "llvm-profdata": self.llvm / "llvm-profdata", "llvm-ar": self.llvm / "llvm-ar",
                           "llvm-dis": self.llvm / "llvm-dis", "llvm-nm": self.llvm / "llvm-nm",
                           "lld-shim": self.llvm / "gcc-ld/ld.lld", "rust-lld": self.llvm / "rust-lld"}
        for path in (self.toolchain / "lib").glob("*.so*"):
            if path.is_file() and (path.name.startswith("libLLVM") or path.name.startswith("librustc_driver")):
                self.tool_paths[path.name] = path
        self.report["tools"] = {name: identity(path) for name, path in self.tool_paths.items()}
        self.report["helper_inputs"] = {name: identity(self.source / "scripts/themes/bench" / name) for name in
                                       ("profile_winui.py", "winui.rs", "pgo_templates.rs", "compact_unwind_link.py",
                                        "compact_unwind.py", "attribute_codegen.py", "attribute_runtime.py")}
        self.persist()

    def persist(self) -> None:
        save(self.report_path, self.report)

    def check_stop(self) -> None:
        if (self.work / "STOP").exists():
            raise RuntimeError("Owner STOP file requested a pause")

    def run(self, label: str, command: list, cwd: Path | None = None,
            extra_environment: dict | None = None, cargo_json: bool = False) -> dict:
        self.check_stop()
        current = resources(self.work)
        if current["free_disk_bytes"] < self.args.min_free_mib * 1024**2:
            raise RuntimeError(f"Insufficient workspace disk before {label}: {current}")
        environment = {**self.environment, **(extra_environment or {})}
        row = {"label": label, "argv": list(map(str, command)), "cwd": str(cwd or self.source),
               "environment_overrides": extra_environment or {}, "started_unix": time.time(),
               "resources": [current], "log": str(self.work / (label + ".log"))}
        messages = self.work / (label + ".cargo.jsonl")
        self.report["commands"].append(row)
        self.persist()
        print("START", label, flush=True)
        start = time.monotonic()
        with Path(row["log"]).open("w") as log, messages.open("w") as output:
            process = subprocess.Popen(row["argv"], cwd=cwd or self.source, env=environment,
                                       stdout=output if cargo_json else log, stderr=log, start_new_session=True)
            row["pid_in_supervisor_namespace"] = process.pid
            while process.poll() is None:
                row["elapsed_seconds"] = time.monotonic() - start
                sample = resources(self.work)
                row["resources"].append(sample)
                reason = ("owner STOP file" if (self.work / "STOP").exists() else
                          "guest available-memory floor" if sample["MemAvailable"] < self.args.min_available_mib * 1024**2 else
                          "workspace disk floor" if sample["free_disk_bytes"] < 256 * 1024**2 else None)
                if reason:
                    row["supervisor_termination_reason"] = reason
                    self.persist()
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=20)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                    break
                self.persist()
                time.sleep(3)
        row.update(returncode=process.returncode, elapsed_seconds=time.monotonic() - start,
                   finished_unix=time.time(), log_identity=identity(Path(row["log"])),
                   cumulative_child_maxrss_kib=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss)
        if cargo_json:
            row["cargo_messages"] = identity(messages)
        self.persist()
        print("END", label, process.returncode, round(row["elapsed_seconds"], 3), flush=True)
        if process.returncode:
            raise RuntimeError(f"{label} failed; retained log: {row['log']}")
        return row

    def prepare_package(self, directory: Path, packer: bool = False) -> None:
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "Cargo.toml").write_text(consumer_manifest(self.source, packer))
        shutil.copy2(self.source / "Cargo.lock", directory / "Cargo.lock")
        if not packer:
            (directory / "main.rs").write_text(self.original)
        label = "packer" if packer else "consumer"
        # Preserve the repository's versions while adding the isolated consumer
        # package to its lock. `generate-lockfile` would resolve newer versions.
        self.run(label + "-resolve", [self.cargo, "fetch", "--target", HOST], directory)
        original_lock = tomllib.loads((self.source / "Cargo.lock").read_text())
        final_lock = tomllib.loads((directory / "Cargo.lock").read_text())
        def external_packages(lock):
            return {(item["name"], item["version"], item["source"], item.get("checksum"))
                    for item in lock["package"] if item.get("source")}
        if not external_packages(final_lock) <= external_packages(original_lock):
            raise RuntimeError("Isolated consumer resolution changed a locked external dependency")
        self.run(label + "-locked-fetch", [self.cargo, "fetch", "--locked", "--target", HOST], directory)
        self.report[label + "_manifest"] = copy_verified(directory / "Cargo.toml", self.work / (label + ".Cargo.toml"))
        self.report[label + "_compile_directory"] = str(directory)
        self.report[label + "_lock"] = copy_verified(directory / "Cargo.lock", self.work / (label + ".Cargo.lock"))
        self.persist()

    def compiler_environment(self, label: str, flags: list[str]) -> dict:
        return {"CARGO_NET_OFFLINE": "true", "CARGO_ENCODED_RUSTFLAGS": "\x1f".join(flags),
                "RUSTC_WRAPPER": str(self.recorder), "FERRO_COMPILER_RECORDS": str(self.work / "compilers" / label),
                "FERRO_REQUIRE_TARGET_BITCODE": "1"}

    def build(self, label: str, flags: list[str], link: list[str]) -> tuple[Path, dict]:
        command = [self.cargo, "rustc", "--locked", "--target", HOST, "--release",
                   "--message-format=json-render-diagnostics", "--features", self.policy.FEATURES,
                   "--bin", "winui-theme-probe", "--", *self.policy.LINK, *link]
        environment = self.compiler_environment(label, flags)
        if label == "optimized":
            environment.update(FERROUI_CC=str(self.cc_recorder), FERRO_CAPTURE_CC=str(self.tool_paths["cc"]),
                               FERRO_CAPTURE_DIRECTORY=str(self.work / "native-link"))
        row = self.run(label, command, self.consumer, environment, cargo_json=True)
        return self.target / HOST / "release/winui-theme-probe", row

    def snapshot_graph(self, cargo_row: dict, profile: Path) -> None:
        graph = self.work / "graph"
        macros = self.work / "host-deps"
        graph.mkdir()
        macros.mkdir()
        inventories = {graph: {}, macros: {}}
        rows = [json.loads(line) for line in Path(cargo_row["cargo_messages"]["path"]).read_text().splitlines()]
        for row in rows:
            if row.get("reason") != "compiler-artifact":
                continue
            for filename in row["filenames"]:
                path = Path(filename)
                destination = (graph if path.suffix == ".rlib" and (self.target / HOST) in path.parents else
                               macros if path.suffix == ".so" and "proc-macro" in row["target"]["kind"] else None)
                if destination:
                    item = copy_verified(path, destination / path.name)
                    inventories[destination][path.name] = item["sha256"]
        for directory, files in inventories.items():
            save(directory / "hashes.json", files)
        base, = graph.glob("libferroui_base-*.rlib")
        records = []
        for path in (self.work / "compilers/optimized").glob("*.json"):
            record = json.loads(path.read_text())
            if record["crate"] == "ferroui_base" and record["target"] == HOST and record.get("returncode") == 0:
                records.append(path)
        if len(records) != 1:
            raise RuntimeError(f"Expected one completed optimized Base compiler, got {records}")
        base_dir = self.work / "base"
        base_dir.mkdir()
        compiler = copy_verified(records[0], base_dir / "compiler.json")
        record = {"source_directory": str(self.frozen_source / "src/FerroUI.Base"), "source": self.report["source"],
                  "compile_source_directory": str(self.source / "src/FerroUI.Base"),
                  "base": identity(base), "profile": identity(profile),
                  "graph": {"path": str(graph), "inventory_sha256": digest(graph / "hashes.json"),
                            "rlibs": len(inventories[graph]), "scope": "Complete freshly profiled optimized WinUI target dependency closure."},
                  "host_dependencies": str(macros), "host_inventory_sha256": digest(macros / "hashes.json"),
                  "compiler_record": compiler["path"], "compiler_record_identity": compiler,
                  "compiler": self.report["rustc"], "phases": {"build": {"exit": 0}}}
        save(base_dir / "manifest.json", record)
        self.report["base_manifest"] = identity(base_dir / "manifest.json")
        self.report["graph"] = record["graph"]
        self.report["host_dependencies"] = record["host_dependencies"]
        self.persist()

    def execute(self) -> None:
        self.run("source-archive", ["git", "archive", "--format=tar.gz", "-o", self.work / "source.tar.gz", "HEAD"])
        self.report["source_archive"] = identity(self.work / "source.tar.gz")
        self.frozen_source.mkdir()
        with tarfile.open(self.work / "source.tar.gz", "r:gz") as archive:
            archive.extractall(self.frozen_source, filter="data")
        verify_source_files(self.frozen_source, self.files)
        self.report["helper_compile_paths"] = {name: row["path"] for name, row in self.report["helper_inputs"].items()}
        self.report["helper_inputs"] = {name: identity(self.frozen_source / "scripts/themes/bench" / name)
                                       for name in self.report["helper_inputs"]}
        self.report["extracted_source_verified"] = True
        self.prepare_package(self.consumer)
        if self.args.packer:
            packer = self.args.packer.resolve(strict=True)
            self.report["packer_reuse"] = identity(packer)
        else:
            package = self.shared / "packer-consumer"
            self.prepare_package(package, packer=True)
            command = [self.cargo, "rustc", "--locked", "--release", "--target", HOST,
                       "--message-format=json-render-diagnostics", "--bin", "ferroui-fxd", "--",
                       "-Clink-arg=-fuse-ld=lld", "-Clto=thin", "-Ccodegen-units=16"]
            self.run("packer-build", command, package,
                     self.compiler_environment("packer", []), cargo_json=True)
            packer = self.target / HOST / "release/ferroui-fxd"
        saved_packer = self.work / "ferroui-fxd"
        self.report["packer"] = copy_verified(packer, saved_packer)
        scalar, structure = self.policy.make_archives(self.source, self.work, str(saved_packer))
        self.report["training_archives"] = {"scalar": identity(scalar), "structure": identity(structure)}
        raw = self.work / "training-raw"
        raw.mkdir()
        instrument_location = self.shared / "instrumentation"
        instrument_location.mkdir(exist_ok=True)
        self.policy.prepare_training_sources(self.source, self.consumer, self.original)
        self.report["training_compile_sources"] = {name: str(self.consumer / name) for name in TRAINING_INPUT_SHA256}
        self.report["training_sources"] = {name: copy_verified(self.consumer / name, self.work / "training-sources" / name) for name in
                                           ("main.rs", "property_work.rs", "template_work.rs")}
        actual = {name: row["sha256"] for name, row in self.report["training_sources"].items()}
        if actual != TRAINING_INPUT_SHA256:
            (self.consumer / "main.rs").write_text(self.original)
            raise RuntimeError(f"Original training source identity changed: {actual}")
        self.report["shipping_main_sha256"] = hashlib.sha256(self.original.encode()).hexdigest()
        self.report["shipping_main"] = copy_verified(self.source / "scripts/themes/bench/winui.rs", self.work / "shipping-main.rs")
        self.persist()
        try:
            binary, _ = self.build("instrumented", ["-Cprofile-generate=" + str(instrument_location), "-Cllvm-args=--addrsig"], [])
            saved = self.work / "instrumented"
            self.report["instrumented_binary"] = copy_verified(binary, saved)
            self.persist()
            for round_index in range(3):
                for lane, arguments, expected in [("startup", [], None), ("templates", [], None),
                                                  ("property", [str(round_index)], None),
                                                  ("edit", [str(scalar)], (14, 17))]:
                    self.train(saved, lane, arguments, expected, raw, round_index)
            self.train(saved, "structure", [str(structure)], (19, 17), raw, None)
        finally:
            (self.consumer / "main.rs").write_text(self.original)
        if len(self.report["training_runs"]) != 13:
            raise RuntimeError("Original training protocol must execute exactly 13 launches")
        for lane in ("startup", "templates", "property", "edit", "structure"):
            if not any(path.stat().st_size for path in raw.glob(lane + "-*.profraw")):
                raise RuntimeError("Missing raw profile for " + lane)
        merged = self.work / "merged.profdata"
        self.run("profile-merge", [self.llvm / "llvm-profdata", "merge", *sorted(raw.glob("*.profraw")), "-o", merged])
        profile = self.work / (digest(merged) + ".profdata")
        merged.rename(profile)
        self.report["profile"] = identity(profile)
        self.report["raw_profiles"] = [identity(path) for path in sorted(raw.glob("*.profraw"))]
        with subprocess.Popen([self.llvm / "llvm-profdata", "show", "--all-functions", "--covered", profile],
                              env=self.environment, text=True, stdout=subprocess.PIPE) as process:
            counts = self.policy.executed_definitions(process.stdout)
            if process.wait() or not all(counts[name] for name in self.policy.REQUIRED):
                raise RuntimeError(f"Missing target profile coverage: {counts}")
        self.report["covered_definitions"] = counts
        self.persist()
        binary, row = self.build("optimized", ["-Cprofile-use=" + str(profile), "-Cllvm-args=--addrsig", "-Copt-level=z"],
                                 ["-Clto=fat", "-Ccodegen-units=1",
                                  "-Clinker=" + str(self.source / "scripts/themes/bench/compact_unwind_link.py")])
        self.report["binary"] = copy_verified(binary, self.work / "winui-theme-probe")
        self.snapshot_graph(row, profile)
        if self.files != source_inventory(self.source):
            raise RuntimeError("Frozen checkout files changed during build")
        verify_source_files(self.frozen_source, self.files)
        if self.report["tools"] != {name: identity(path) for name, path in self.tool_paths.items()}:
            raise RuntimeError("Compiler/linker tool identity changed during build")
        self.report.update(source_files_unchanged=True, tool_identities_unchanged=True, complete=True, finished_unix=time.time())
        self.persist()
        print(json.dumps({key: self.report[key] for key in ("binary", "profile", "graph", "base_manifest")}, indent=2), flush=True)

    def train(self, binary: Path, lane: str, arguments: list[str], expected: tuple[int, int] | None,
              raw: Path, round_index: int | None) -> None:
        environment = {"FERROUI_PGO_TRAIN": lane, "LLVM_PROFILE_FILE": str(raw / (lane + "-%m-%p.profraw"))}
        if expected:
            environment.update(FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE=str(expected[0]),
                               FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE=str(expected[1]))
        row = self.run(f"train-{len(self.report['training_runs']):02d}-{lane}", [binary, *arguments],
                       extra_environment=environment)
        self.report["training_runs"].append({"lane": lane, "round": round_index, **row})
        self.persist()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--shared-work", type=Path, required=True)
    parser.add_argument("--toolchain-prefix", type=Path, required=True)
    parser.add_argument("--cargo-home", type=Path, required=True)
    parser.add_argument("--packer", type=Path)
    parser.add_argument("--cpus", default="4")
    parser.add_argument("--min-free-mib", type=int, default=4096)
    parser.add_argument("--min-available-mib", type=int, default=512)
    args = parser.parse_args()
    replay = Replay(args)
    try:
        replay.execute()
    except BaseException as error:
        replay.report["failure"] = {"type": type(error).__name__, "message": str(error), "unix": time.time()}
        replay.persist()
        raise


if __name__ == "__main__":
    main()
